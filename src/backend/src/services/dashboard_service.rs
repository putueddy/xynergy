//! Role-based dashboard service.
//!
//! Composes existing CTC completeness, team capacity, budget, project P&L,
//! cash-flow, CTC validation, and audit services into bounded per-role
//! summaries for the `/api/v1/dashboard` endpoint.
//!
//! The route layer is responsible for role gating and JWT extraction. This
//! module focuses on data orchestration only.

use axum::http::HeaderMap;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::routes::Claims;
use crate::services::budget_service::{
    compute_department_budget_utilization, DepartmentBudgetSummaryResponse,
};
use crate::services::compliance_report::validate_bpjs_compliance_in_transaction;
use crate::services::ctc_completeness::{
    get_completeness_summary_in_transaction, get_missing_employees_in_transaction,
    CompletenessReport,
};
use crate::services::ctc_validation_report::{generate_validation_report, ValidationReportFilters};
use crate::services::project_pl_service::{get_project_pl_dashboard, get_project_pl_forecast};
use crate::services::rls_context::begin_rls_transaction;
use crate::services::team_service::{
    get_capacity_report_in_transaction, get_team_members_in_transaction, CapacityReportResponse,
    TeamMemberResponse,
};

// ─────────────────────────────────────────────────────────────────────────
// Public response contract
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct RoleDashboardResponse {
    pub role: String,
    pub generated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hr: Option<HrDashboard>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub department_head: Option<DepartmentHeadDashboard>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_manager: Option<ProjectManagerDashboard>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finance: Option<FinanceDashboard>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin: Option<AdminDashboard>,
}

// ── HR ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct HrDashboard {
    pub completeness: CompletenessReport,
    pub pending_updates: HrPendingUpdates,
    pub recent_changes: Vec<RecentCtcChange>,
    pub compliance_alerts: ComplianceAlertSummary,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct HrPendingUpdates {
    pub missing_count: i64,
    pub sample: Vec<HrMissingEmployee>,
}

#[derive(Debug, Serialize)]
pub struct HrMissingEmployee {
    pub id: Uuid,
    pub name: String,
    pub department: String,
}

#[derive(Debug, Serialize)]
pub struct RecentCtcChange {
    pub resource_id: Uuid,
    pub resource_name: String,
    pub revision_number: i32,
    pub changed_by_id: Uuid,
    pub changed_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct ComplianceAlertSummary {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub total_validated: i64,
    pub total_passed: i64,
    pub total_discrepancies: i64,
    pub compliance_rate_pct: f64,
    pub top_risks: Vec<ComplianceTopRisk>,
}

#[derive(Debug, Serialize)]
pub struct ComplianceTopRisk {
    pub resource_id: Uuid,
    pub name: String,
    pub status: String,
    pub variance_amount: i64,
}

// ── Department Head ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DepartmentHeadDashboard {
    pub department_id: Uuid,
    pub utilization: UtilizationSummary,
    pub budget: Option<DepartmentBudgetSummaryResponse>,
    pub overallocations: OverallocationSummary,
    pub upcoming_assignments: Vec<UpcomingAssignment>,
    pub warnings: Vec<String>,
    // Story 6.4: deeper Team Utilization Dashboard surface.
    pub team_members: Vec<TeamUtilizationMember>,
    pub utilization_trends: TeamUtilizationTrendBundle,
    pub underutilized_members: Vec<TeamUtilizationMember>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TeamUtilizationMember {
    pub resource_id: Uuid,
    pub resource_name: String,
    pub role: String,
    pub current_utilization_pct: f64,
    pub available_capacity_pct: f64,
    pub is_underutilized: bool,
    pub is_overallocated: bool,
    pub ctc_status: String,
    pub current_projects: Vec<TeamUtilizationCurrentProject>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TeamUtilizationCurrentProject {
    pub project_name: String,
    pub allocation_percentage: f64,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Serialize)]
pub struct TeamUtilizationTrendBundle {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub members: Vec<TeamUtilizationTrend>,
}

#[derive(Debug, Serialize)]
pub struct TeamUtilizationTrend {
    pub resource_id: Uuid,
    pub resource_name: String,
    pub periods: Vec<TeamUtilizationTrendPeriod>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TeamUtilizationTrendPeriod {
    pub period: String,
    pub utilization_pct: f64,
}

/// Caller-resolved team trend range for the Department Head dashboard.
///
/// Validated at the route boundary so the service can assume `start_date <=
/// end_date` and that the span is within `DH_TEAM_RANGE_MAX_DAYS`.
#[derive(Debug, Clone, Copy)]
pub struct DepartmentHeadTeamRange {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

/// Bounded cap on the number of nested current-project rows surfaced for one
/// team member. Story 6.4 calls for "bounded current_projects" to avoid
/// unbounded fan-out when one resource has many concurrent assignments.
pub const DH_CURRENT_PROJECTS_PER_MEMBER_LIMIT: usize = 10;

pub const DH_TEAM_RANGE_DEFAULT_DAYS: i64 = 30;
pub const DH_TEAM_RANGE_MAX_DAYS: i64 = 366;
pub const DH_UNDERUTILIZED_THRESHOLD_PCT: f64 = 50.0;

#[derive(Debug, Serialize)]
pub struct UtilizationSummary {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub average_utilization_pct: f64,
    pub overallocated_count: i64,
    pub top_at_risk: Vec<UtilizationAtRisk>,
}

#[derive(Debug, Serialize)]
pub struct UtilizationAtRisk {
    pub resource_id: Uuid,
    pub resource_name: String,
    pub current_allocation_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct OverallocationSummary {
    pub overallocated_count: i64,
    pub members: Vec<UtilizationAtRisk>,
}

#[derive(Debug, Serialize)]
pub struct UpcomingAssignment {
    pub allocation_id: Uuid,
    pub resource_id: Uuid,
    pub resource_name: String,
    pub project_id: Uuid,
    pub project_name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub allocation_percentage: f64,
}

// ── Project Manager ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ProjectManagerDashboard {
    pub active_projects: Vec<ProjectHealthCard>,
    pub margin_alerts: Vec<MarginAlert>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectHealthCard {
    pub project_id: Uuid,
    pub project_name: String,
    pub status: String,
    pub end_date: NaiveDate,
    pub total_budget_idr: i64,
    pub budget_spent_idr: i64,
    pub budget_remaining_idr: i64,
    pub budget_status: String,
    pub budget_utilization_pct: f64,
    pub is_over_budget: bool,
    pub budget_overrun_idr: i64,
    pub total_revenue_idr: i64,
    pub total_cost_idr: i64,
    pub gross_profit_idr: i64,
    pub margin_pct: f64,
    pub target_margin_pct: f64,
    pub margin_alert_threshold_pct: f64,
    pub margin_alert: Option<String>,
    pub forecast_margin_pct: f64,
    pub forecast_variance_from_target_pct: f64,
    pub projected_total_cost_idr: i64,
    pub forecast_unavailable: bool,
    pub forecast_has_revenue_signal: bool,
    pub health_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MarginAlert {
    pub project_id: Uuid,
    pub project_name: String,
    pub margin_pct: f64,
    pub margin_alert_threshold_pct: f64,
    pub message: String,
}

// ── Finance ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct FinanceDashboard {
    pub cash_position: CashPositionSummary,
    pub ctc_validation: CtcValidationStatus,
    pub audit_alerts: AuditAlertsSummary,
    pub export_requests: ExportRequestsSummary,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CashPositionSummary {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub total_cash_in_idr: i64,
    pub total_cash_out_idr: i64,
    pub net_cash_flow_idr: i64,
    pub ending_cumulative_position_idr: i64,
}

#[derive(Debug, Serialize)]
pub struct CtcValidationStatus {
    pub status: String, // "ok" | "no_data" | "error"
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_compared: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_matches: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_discrepancies: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_rate_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuditAlertsSummary {
    pub since: DateTime<Utc>,
    pub access_denied_count: i64,
    pub login_failed_count: i64,
    pub login_blocked_count: i64,
    pub chain_verification_failure_count: i64,
    pub recent: Vec<AuditAlertEntry>,
}

#[derive(Debug, Serialize)]
pub struct AuditAlertEntry {
    pub id: Uuid,
    pub action: String,
    pub entity_type: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ExportRequestsSummary {
    pub pending_count: i64,
    pub latest_pending: Vec<PendingExportRequest>,
}

#[derive(Debug, Serialize)]
pub struct PendingExportRequest {
    pub id: Uuid,
    pub requested_by: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub report_type: Option<String>,
}

// ── Admin ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AdminDashboard {
    pub total_users: i64,
    pub total_departments: i64,
    pub total_active_projects: i64,
    pub total_active_ctc_records: i64,
    pub pending_export_requests: i64,
    pub access_denied_24h: i64,
}

// ─────────────────────────────────────────────────────────────────────────
// Tunable constants
// ─────────────────────────────────────────────────────────────────────────

pub const RECENT_CHANGE_LIMIT: i64 = 10;
pub const MISSING_CTC_SAMPLE_LIMIT: usize = 5;
pub const COMPLIANCE_TOP_RISK_LIMIT: usize = 5;
pub const DH_AT_RISK_LIMIT: usize = 5;
pub const DH_UPCOMING_LIMIT: i64 = 10;
pub const PM_ACTIVE_PROJECT_LIMIT: i64 = 10;
pub const AUDIT_RECENT_LIMIT: i64 = 10;
pub const AUDIT_WINDOW_DAYS: i64 = 7;
pub const EXPORT_REQUESTS_LIMIT: i64 = 5;

// ─────────────────────────────────────────────────────────────────────────
// Top-level orchestration
// ─────────────────────────────────────────────────────────────────────────

pub async fn build_dashboard(
    pool: &PgPool,
    headers: &HeaderMap,
    claims: &Claims,
    team_range: DepartmentHeadTeamRange,
    dashboard_date: NaiveDate,
) -> Result<RoleDashboardResponse> {
    let role = claims.role.clone();
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Authentication("Invalid user ID in token".to_string()))?;

    let generated_at = Utc::now();

    let mut response = RoleDashboardResponse {
        role: role.clone(),
        generated_at,
        hr: None,
        department_head: None,
        project_manager: None,
        finance: None,
        admin: None,
    };

    match role.as_str() {
        "hr" => {
            let mut tx = begin_rls_transaction(pool, headers).await?;
            response.hr = Some(build_hr_dashboard(&mut tx).await?);
            tx.commit()
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        "department_head" => {
            let mut tx = begin_rls_transaction(pool, headers).await?;
            let department_id = current_department_id(&mut tx).await?;
            ensure_department_head_relationship(&mut tx, user_id, department_id).await?;
            response.department_head = Some(
                build_department_head_dashboard(&mut tx, department_id, team_range, dashboard_date)
                    .await?,
            );
            tx.commit()
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        "project_manager" => {
            response.project_manager = Some(build_project_manager_dashboard(pool, user_id).await?);
        }
        "finance" => {
            response.finance = Some(build_finance_dashboard(pool).await?);
        }
        "admin" => {
            response.admin = Some(build_admin_dashboard(pool).await?);
        }
        other => {
            return Err(AppError::Forbidden(format!(
                "Role '{}' is not supported by the dashboard",
                other
            )));
        }
    }

    Ok(response)
}

async fn current_department_id(tx: &mut Transaction<'_, Postgres>) -> Result<Uuid> {
    let dept_id_str: String =
        sqlx::query_scalar("SELECT current_setting('app.current_department_id', true)")
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

    if dept_id_str.is_empty() {
        return Err(AppError::Forbidden(
            "Department head has no department assignment".to_string(),
        ));
    }

    Uuid::parse_str(&dept_id_str)
        .map_err(|_| AppError::Internal("Invalid department_id in session".to_string()))
}

async fn ensure_department_head_relationship(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    department_id: Uuid,
) -> Result<()> {
    let is_head = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1
            FROM departments
            WHERE id = $1
              AND head_id = $2
        )",
    )
    .bind(department_id)
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if !is_head {
        return Err(AppError::Forbidden(
            "Department head is not assigned to this department".to_string(),
        ));
    }

    Ok(())
}

async fn savepoint(tx: &mut Transaction<'_, Postgres>, name: &str) -> Result<()> {
    sqlx::query(&format!("SAVEPOINT {}", name))
        .execute(&mut **tx)
        .await
        .map(|_| ())
        .map_err(|e| AppError::Database(e.to_string()))
}

async fn release_savepoint(tx: &mut Transaction<'_, Postgres>, name: &str) -> Result<()> {
    sqlx::query(&format!("RELEASE SAVEPOINT {}", name))
        .execute(&mut **tx)
        .await
        .map(|_| ())
        .map_err(|e| AppError::Database(e.to_string()))
}

async fn rollback_savepoint(tx: &mut Transaction<'_, Postgres>, name: &str) -> Result<()> {
    sqlx::query(&format!("ROLLBACK TO SAVEPOINT {}", name))
        .execute(&mut **tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    release_savepoint(tx, name).await
}

// ─────────────────────────────────────────────────────────────────────────
// HR
// ─────────────────────────────────────────────────────────────────────────

async fn build_hr_dashboard(tx: &mut Transaction<'_, Postgres>) -> Result<HrDashboard> {
    let mut warnings = Vec::new();
    let completeness = get_completeness_summary_in_transaction(&mut *tx, None).await?;

    let missing_employees = get_missing_employees_in_transaction(&mut *tx, None).await?;
    let missing_count = missing_employees.len() as i64;
    let sample: Vec<HrMissingEmployee> = missing_employees
        .into_iter()
        .take(MISSING_CTC_SAMPLE_LIMIT)
        .map(|e| HrMissingEmployee {
            id: e.id,
            name: e.name,
            department: e.department,
        })
        .collect();

    savepoint(&mut *tx, "dashboard_recent_ctc").await?;
    let mut recent_changes_unavailable = false;
    let recent_changes = match load_recent_ctc_changes(&mut *tx).await {
        Ok(changes) => changes,
        Err(e) => {
            recent_changes_unavailable = true;
            rollback_savepoint(&mut *tx, "dashboard_recent_ctc").await?;
            tracing::warn!("dashboard recent CTC changes unavailable: {}", e);
            warnings.push("Recent CTC changes are temporarily unavailable.".to_string());
            Vec::new()
        }
    };
    if !recent_changes_unavailable {
        release_savepoint(&mut *tx, "dashboard_recent_ctc").await?;
    }

    let today = Utc::now().date_naive();
    let start_of_window = today
        .checked_sub_signed(chrono::Duration::days(30))
        .unwrap_or(today);
    savepoint(&mut *tx, "dashboard_compliance").await?;
    let mut compliance_unavailable = false;
    let compliance_report =
        match validate_bpjs_compliance_in_transaction(&mut *tx, start_of_window, today).await {
            Ok(report) => report,
            Err(e) => {
                compliance_unavailable = true;
                rollback_savepoint(&mut *tx, "dashboard_compliance").await?;
                tracing::warn!("dashboard compliance summary unavailable: {}", e);
                warnings.push("Compliance alerts are temporarily unavailable.".to_string());
                crate::services::compliance_report::ComplianceReport {
                    results: Vec::new(),
                    total_validated: 0,
                    total_passed: 0,
                    total_discrepancies: 0,
                    compliance_rate_pct: 0.0,
                }
            }
        };
    if !compliance_unavailable {
        release_savepoint(&mut *tx, "dashboard_compliance").await?;
    }

    let mut top_risks: Vec<ComplianceTopRisk> = compliance_report
        .results
        .iter()
        .filter(|r| r.status == "DISCREPANCY")
        .map(|r| ComplianceTopRisk {
            resource_id: r.resource_id,
            name: r.name.clone(),
            status: r.status.clone(),
            variance_amount: r.variance_amount,
        })
        .collect();
    top_risks.sort_by(|a, b| b.variance_amount.cmp(&a.variance_amount));
    top_risks.truncate(COMPLIANCE_TOP_RISK_LIMIT);

    let compliance_alerts = ComplianceAlertSummary {
        start_date: start_of_window,
        end_date: today,
        total_validated: compliance_report.total_validated,
        total_passed: compliance_report.total_passed,
        total_discrepancies: compliance_report.total_discrepancies,
        compliance_rate_pct: compliance_report.compliance_rate_pct,
        top_risks,
    };

    Ok(HrDashboard {
        completeness,
        pending_updates: HrPendingUpdates {
            missing_count,
            sample,
        },
        recent_changes,
        compliance_alerts,
        warnings,
    })
}

async fn load_recent_ctc_changes(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<Vec<RecentCtcChange>> {
    let rows = sqlx::query(
        r#"
        SELECT
            v.id,
            v.resource_id,
            r.name AS resource_name,
            v.revision_number,
            v.changed_by,
            v.reason,
            v.created_at,
            u.first_name AS first_name,
            u.last_name AS last_name
        FROM ctc_revisions v
        JOIN resources r ON r.id = v.resource_id
        LEFT JOIN users u ON u.id = v.changed_by
        ORDER BY v.created_at DESC
        LIMIT $1
        "#,
    )
    .bind(RECENT_CHANGE_LIMIT)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let resource_id: Uuid = row
            .try_get("resource_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let resource_name: String = row
            .try_get("resource_name")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let revision_number: i32 = row
            .try_get("revision_number")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let changed_by_id: Uuid = row
            .try_get("changed_by")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let reason: String = row
            .try_get("reason")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let created_at: DateTime<Utc> = row
            .try_get("created_at")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let first_name: Option<String> = row
            .try_get("first_name")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let last_name: Option<String> = row
            .try_get("last_name")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let changed_by_name = {
            let combined = format!(
                "{} {}",
                first_name.unwrap_or_default(),
                last_name.unwrap_or_default()
            )
            .trim()
            .to_string();
            if combined.is_empty() {
                None
            } else {
                Some(combined)
            }
        };

        out.push(RecentCtcChange {
            resource_id,
            resource_name,
            revision_number,
            changed_by_id,
            changed_by_name,
            created_at,
            reason,
        });
    }

    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────
// Department Head
// ─────────────────────────────────────────────────────────────────────────

async fn build_department_head_dashboard(
    tx: &mut Transaction<'_, Postgres>,
    department_id: Uuid,
    team_range: DepartmentHeadTeamRange,
    today: NaiveDate,
) -> Result<DepartmentHeadDashboard> {
    let mut warnings = Vec::new();
    // `today` anchors the budget period and the upcoming-assignment scan, both
    // of which intentionally remain bound to the calendar regardless of the
    // user-selected trend range (Story 6.4 only widens the trend window).
    let summary_start = today;
    let summary_end = today
        .checked_add_signed(chrono::Duration::days(DH_TEAM_RANGE_DEFAULT_DAYS))
        .unwrap_or(today);
    let trend_start = team_range.start_date;
    let trend_end = team_range.end_date;

    let summary_capacity = get_capacity_report_in_transaction(
        &mut *tx,
        Some(department_id),
        summary_start,
        summary_end,
    )
    .await?;

    let trend_capacity = if trend_start == summary_start && trend_end == summary_end {
        None
    } else {
        savepoint(&mut *tx, "dashboard_team_capacity").await?;
        let capacity = match get_capacity_report_in_transaction(
            &mut *tx,
            Some(department_id),
            trend_start,
            trend_end,
        )
        .await
        {
            Ok(capacity) => {
                release_savepoint(&mut *tx, "dashboard_team_capacity").await?;
                capacity
            }
            Err(e) => match e {
                AppError::Authentication(_) | AppError::Forbidden(_) | AppError::Validation(_) => {
                    return Err(e);
                }
                other => {
                    rollback_savepoint(&mut *tx, "dashboard_team_capacity").await?;
                    tracing::warn!("dashboard utilization trends unavailable: {}", other);
                    warnings.push("Utilization trend data is temporarily unavailable.".to_string());
                    CapacityReportResponse {
                        start_date: trend_start.to_string(),
                        end_date: trend_end.to_string(),
                        employees: Vec::new(),
                    }
                }
            },
        };
        Some(capacity)
    };
    let capacity_for_trends = trend_capacity.as_ref().unwrap_or(&summary_capacity);

    let team_members =
        get_team_members_in_transaction(&mut *tx, Some(department_id), "department_head").await?;

    savepoint(&mut *tx, "dashboard_upcoming").await?;
    let mut upcoming_unavailable = false;
    let upcoming = match load_upcoming_assignments(&mut *tx, department_id, today).await {
        Ok(assignments) => assignments,
        Err(e) => {
            upcoming_unavailable = true;
            rollback_savepoint(&mut *tx, "dashboard_upcoming").await?;
            tracing::warn!("dashboard upcoming assignments unavailable: {}", e);
            warnings.push("Upcoming assignments are temporarily unavailable.".to_string());
            Vec::new()
        }
    };
    if !upcoming_unavailable {
        release_savepoint(&mut *tx, "dashboard_upcoming").await?;
    }

    let period = format!("{:04}-{:02}", today.year(), today.month());
    // Budget data is optional — if no budget configured the service still returns a row
    // with zeroed fields, so we wrap it in Some(...) only after confirming it ran cleanly.
    savepoint(&mut *tx, "dashboard_budget").await?;
    let mut budget_savepoint_open = true;
    let budget = match compute_department_budget_utilization(&mut *tx, department_id, &period).await
    {
        Ok(summary) => Some(summary),
        Err(AppError::NotFound(_)) => {
            rollback_savepoint(&mut *tx, "dashboard_budget").await?;
            budget_savepoint_open = false;
            None
        }
        Err(e) => {
            rollback_savepoint(&mut *tx, "dashboard_budget").await?;
            budget_savepoint_open = false;
            tracing::warn!("dashboard department budget unavailable: {}", e);
            warnings.push("Budget status is temporarily unavailable.".to_string());
            None
        }
    };
    if budget_savepoint_open {
        release_savepoint(&mut *tx, "dashboard_budget").await?;
    }

    let mut avg_total = 0.0_f64;
    let mut avg_count = 0_usize;
    let mut at_risk: Vec<UtilizationAtRisk> = Vec::new();
    for emp in &summary_capacity.employees {
        for p in &emp.periods {
            avg_total += p.total_allocation_percentage;
            avg_count += 1;
        }
    }
    let average_utilization_pct = if avg_count == 0 {
        0.0
    } else {
        (avg_total / avg_count as f64 * 10.0).round() / 10.0
    };

    let overallocated_members: Vec<UtilizationAtRisk> = team_members
        .iter()
        .filter(|m| m.is_overallocated)
        .map(|m| UtilizationAtRisk {
            resource_id: m.resource_id,
            resource_name: m.name.clone(),
            current_allocation_pct: m.current_allocation_percentage,
        })
        .collect();

    for m in &team_members {
        if m.current_allocation_percentage >= 80.0 {
            at_risk.push(UtilizationAtRisk {
                resource_id: m.resource_id,
                resource_name: m.name.clone(),
                current_allocation_pct: m.current_allocation_percentage,
            });
        }
    }
    at_risk.sort_by(|a, b| {
        b.current_allocation_pct
            .partial_cmp(&a.current_allocation_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    at_risk.truncate(DH_AT_RISK_LIMIT);

    // Story 6.4 surface: build dense per-member operational rows from the
    // canonical team_members payload (current_allocation_percentage is the
    // authoritative current utilization signal), and bundle per-resource
    // trends from the same weighted capacity report already loaded above.
    let utilization_members = build_team_utilization_members(&team_members);
    let underutilized: Vec<TeamUtilizationMember> = utilization_members
        .iter()
        .filter(|m| m.is_underutilized)
        .cloned()
        .collect();
    let utilization_trends = build_utilization_trends(capacity_for_trends, trend_start, trend_end);

    Ok(DepartmentHeadDashboard {
        department_id,
        utilization: UtilizationSummary {
            start_date: summary_start,
            end_date: summary_end,
            average_utilization_pct,
            overallocated_count: overallocated_members.len() as i64,
            top_at_risk: at_risk,
        },
        budget,
        overallocations: OverallocationSummary {
            overallocated_count: overallocated_members.len() as i64,
            members: overallocated_members,
        },
        upcoming_assignments: upcoming,
        warnings,
        team_members: utilization_members,
        utilization_trends,
        underutilized_members: underutilized,
    })
}

/// Project the canonical `TeamMemberResponse` rows into the bounded
/// dashboard-facing shape. Available capacity is derived from current
/// utilization (clamped to non-negative for display only); it is never an
/// authorization or allocation-validity check.
fn build_team_utilization_members(
    team_members: &[TeamMemberResponse],
) -> Vec<TeamUtilizationMember> {
    team_members
        .iter()
        .map(|m| {
            let current = m.current_allocation_percentage;
            let available = (100.0 - current).max(0.0);
            // Available capacity rounds to one decimal place to avoid noisy
            // polling deltas while keeping fine-grained capacity visible.
            let available = (available * 10.0).round() / 10.0;
            let mut active_assignments = m.active_assignments.iter().collect::<Vec<_>>();
            active_assignments.sort_by(|a, b| {
                a.project_name
                    .to_lowercase()
                    .cmp(&b.project_name.to_lowercase())
                    .then_with(|| a.start_date.cmp(&b.start_date))
                    .then_with(|| a.end_date.cmp(&b.end_date))
                    .then_with(|| {
                        a.allocation_pct
                            .partial_cmp(&b.allocation_pct)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            });
            let projects: Vec<TeamUtilizationCurrentProject> = active_assignments
                .into_iter()
                .take(DH_CURRENT_PROJECTS_PER_MEMBER_LIMIT)
                .map(|a| TeamUtilizationCurrentProject {
                    project_name: a.project_name.clone(),
                    allocation_percentage: a.allocation_pct,
                    start_date: a.start_date.clone(),
                    end_date: a.end_date.clone(),
                })
                .collect();
            TeamUtilizationMember {
                resource_id: m.resource_id,
                resource_name: m.name.clone(),
                role: m.role.clone(),
                current_utilization_pct: current,
                available_capacity_pct: available,
                is_underutilized: current < DH_UNDERUTILIZED_THRESHOLD_PCT,
                is_overallocated: m.is_overallocated,
                ctc_status: m.ctc_status.clone(),
                current_projects: projects,
            }
        })
        .collect()
}

/// Project the weighted capacity report into a per-member trend bundle.
/// The capacity service is the source of truth for the working-day formula,
/// so the dashboard never re-derives utilization from raw allocation sums.
fn build_utilization_trends(
    capacity: &crate::services::team_service::CapacityReportResponse,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> TeamUtilizationTrendBundle {
    let members = capacity
        .employees
        .iter()
        .map(|emp| TeamUtilizationTrend {
            resource_id: emp.resource_id,
            resource_name: emp.resource_name.clone(),
            periods: emp
                .periods
                .iter()
                .map(|p| TeamUtilizationTrendPeriod {
                    period: p.period.clone(),
                    utilization_pct: p.total_allocation_percentage,
                })
                .collect(),
        })
        .collect();
    TeamUtilizationTrendBundle {
        start_date: range_start,
        end_date: range_end,
        members,
    }
}

async fn load_upcoming_assignments(
    tx: &mut Transaction<'_, Postgres>,
    department_id: Uuid,
    today: NaiveDate,
) -> Result<Vec<UpcomingAssignment>> {
    let rows = sqlx::query(
        r#"
        SELECT
            a.id AS allocation_id,
            a.resource_id,
            r.name AS resource_name,
            a.project_id,
            p.name AS project_name,
            a.start_date,
            a.end_date,
            a.allocation_percentage
        FROM allocations a
        JOIN resources r ON r.id = a.resource_id
        JOIN projects p ON p.id = a.project_id
        WHERE r.department_id = $1
          AND a.start_date >= $2
        ORDER BY a.start_date ASC, p.name ASC
        LIMIT $3
        "#,
    )
    .bind(department_id)
    .bind(today)
    .bind(DH_UPCOMING_LIMIT)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let allocation_id: Uuid = row
            .try_get("allocation_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let resource_id: Uuid = row
            .try_get("resource_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let resource_name: String = row
            .try_get("resource_name")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let project_id: Uuid = row
            .try_get("project_id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let project_name: String = row
            .try_get("project_name")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let start_date: NaiveDate = row
            .try_get("start_date")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let end_date: NaiveDate = row
            .try_get("end_date")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let allocation_percentage_bd: sqlx::types::BigDecimal = row
            .try_get("allocation_percentage")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let allocation_percentage = allocation_percentage_bd
            .to_string()
            .parse::<f64>()
            .unwrap_or(0.0);

        out.push(UpcomingAssignment {
            allocation_id,
            resource_id,
            resource_name,
            project_id,
            project_name,
            start_date,
            end_date,
            allocation_percentage,
        });
    }

    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────
// Project Manager
// ─────────────────────────────────────────────────────────────────────────

async fn build_project_manager_dashboard(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<ProjectManagerDashboard> {
    let mut warnings = Vec::new();
    let today = Utc::now().date_naive();
    let project_rows = sqlx::query(
        r#"
        SELECT id, name, status, end_date, total_budget_idr
        FROM projects
        WHERE project_manager_id = $1
          AND LOWER(status) = 'active'
          AND end_date >= $2
        ORDER BY end_date ASC, name ASC
        LIMIT $3
        "#,
    )
    .bind(user_id)
    .bind(today)
    .bind(PM_ACTIVE_PROJECT_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if project_rows.is_empty() {
        return Ok(ProjectManagerDashboard {
            active_projects: Vec::new(),
            margin_alerts: Vec::new(),
            warnings,
        });
    }

    let year = today.year();

    let mut cards: Vec<ProjectHealthCard> = Vec::with_capacity(project_rows.len());
    let mut alerts: Vec<MarginAlert> = Vec::new();

    for row in project_rows {
        let project_id: Uuid = row
            .try_get("id")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let project_name: String = row
            .try_get("name")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let status: String = row
            .try_get("status")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let end_date: NaiveDate = row
            .try_get("end_date")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let total_budget_idr: i64 = row
            .try_get("total_budget_idr")
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Fetch current P&L and forecast in parallel for this single project.
        // Bounded by PM_ACTIVE_PROJECT_LIMIT; each project has at most two
        // inflight service reads while its card is being assembled.
        let pl_future = get_project_pl_dashboard(pool, project_id, year);
        let forecast_future = get_project_pl_forecast(pool, project_id, year, Some(today));
        let (pl_result, forecast_result) = tokio::join!(pl_future, forecast_future);

        let pl = match pl_result {
            Ok(pl) => pl,
            Err(e) => {
                tracing::warn!(
                    project_id = %project_id,
                    "dashboard project P&L unavailable: {}",
                    e
                );
                warnings.push(format!(
                    "P&L data is temporarily unavailable for {}.",
                    project_name
                ));
                cards.push(safe_unavailable_card(
                    project_id,
                    project_name,
                    status,
                    end_date,
                    total_budget_idr,
                ));
                continue;
            }
        };

        let budget_spent_idr = pl.total_cost_idr;
        let budget_remaining_idr = total_budget_idr - budget_spent_idr;
        let budget_utilization_pct =
            compute_budget_utilization_pct(total_budget_idr, budget_spent_idr);
        let is_over_budget = total_budget_idr > 0 && budget_spent_idr > total_budget_idr;
        let budget_overrun_idr = if is_over_budget {
            budget_spent_idr - total_budget_idr
        } else {
            0
        };
        let budget_status = project_budget_status(total_budget_idr, budget_spent_idr);

        // Forecast data is optional: a forecast failure must not blank the card.
        let (
            forecast_margin_pct,
            forecast_variance_from_target_pct,
            projected_total_cost_idr,
            forecast_unavailable,
            forecast_has_revenue_signal,
            forecast_warning,
        ) = match forecast_result {
            Ok(forecast) => (
                forecast.forecast_margin_pct,
                forecast.variance_from_target_pct,
                forecast.projected_total_cost_idr,
                false,
                forecast.current_revenue_idr > 0,
                None,
            ),
            Err(e) => {
                tracing::warn!(
                    project_id = %project_id,
                    "dashboard project forecast unavailable: {}",
                    e
                );
                warnings.push(format!(
                    "Forecast data is temporarily unavailable for {}.",
                    project_name
                ));
                (
                    0.0,
                    0.0,
                    pl.total_cost_idr,
                    true,
                    false,
                    Some("Forecast data is temporarily unavailable.".to_string()),
                )
            }
        };

        if let Some(message) = pl.margin_alert.clone() {
            alerts.push(MarginAlert {
                project_id,
                project_name: project_name.clone(),
                margin_pct: pl.margin_pct,
                margin_alert_threshold_pct: pl.margin_alert_threshold_pct,
                message,
            });
        }

        let health_status = derive_health_status(
            total_budget_idr,
            is_over_budget,
            &budget_status,
            pl.margin_alert.is_some(),
            forecast_warning.is_some(),
            forecast_has_revenue_signal,
            forecast_variance_from_target_pct,
            pl.margin_alert_threshold_pct,
        );

        cards.push(ProjectHealthCard {
            project_id,
            project_name,
            status,
            end_date,
            total_budget_idr,
            budget_spent_idr,
            budget_remaining_idr,
            budget_status,
            budget_utilization_pct,
            is_over_budget,
            budget_overrun_idr,
            total_revenue_idr: pl.total_revenue_idr,
            total_cost_idr: pl.total_cost_idr,
            gross_profit_idr: pl.gross_profit_idr,
            margin_pct: pl.margin_pct,
            target_margin_pct: pl.target_margin_pct,
            margin_alert_threshold_pct: pl.margin_alert_threshold_pct,
            margin_alert: pl.margin_alert,
            forecast_margin_pct,
            forecast_variance_from_target_pct,
            projected_total_cost_idr,
            forecast_unavailable,
            forecast_has_revenue_signal,
            health_status,
            warning: forecast_warning,
        });
    }

    Ok(ProjectManagerDashboard {
        active_projects: cards,
        margin_alerts: alerts,
        warnings,
    })
}

fn safe_unavailable_card(
    project_id: Uuid,
    project_name: String,
    status: String,
    end_date: NaiveDate,
    total_budget_idr: i64,
) -> ProjectHealthCard {
    ProjectHealthCard {
        project_id,
        project_name,
        status,
        end_date,
        total_budget_idr,
        budget_spent_idr: 0,
        budget_remaining_idr: total_budget_idr,
        budget_status: "unconfigured".to_string(),
        budget_utilization_pct: 0.0,
        is_over_budget: false,
        budget_overrun_idr: 0,
        total_revenue_idr: 0,
        total_cost_idr: 0,
        gross_profit_idr: 0,
        margin_pct: 0.0,
        target_margin_pct: 0.0,
        margin_alert_threshold_pct: 0.0,
        margin_alert: None,
        forecast_margin_pct: 0.0,
        forecast_variance_from_target_pct: 0.0,
        projected_total_cost_idr: 0,
        forecast_unavailable: true,
        forecast_has_revenue_signal: false,
        health_status: "unconfigured".to_string(),
        warning: Some("P&L data is temporarily unavailable.".to_string()),
    }
}

fn compute_budget_utilization_pct(total_budget_idr: i64, spent_idr: i64) -> f64 {
    if total_budget_idr <= 0 {
        0.0
    } else {
        (spent_idr as f64 / total_budget_idr as f64) * 100.0
    }
}

fn project_budget_status(total_budget_idr: i64, spent_idr: i64) -> String {
    if total_budget_idr <= 0 {
        return "unconfigured".to_string();
    }

    let utilization_pct = spent_idr as f64 / total_budget_idr as f64 * 100.0;
    if utilization_pct < 50.0 {
        "healthy".to_string()
    } else if utilization_pct < 80.0 {
        "warning".to_string()
    } else {
        "critical".to_string()
    }
}

/// Derive a stable health severity from canonical P&L + forecast signals.
///
/// `critical` when over budget, near-budget, current margin tripped the alert,
/// or forecast margin is below target by more than the configured threshold.
/// `warning` when budget utilization is mid-range or forecast lags target by
/// any positive amount. `healthy` otherwise. `unconfigured` when no budget.
fn derive_health_status(
    total_budget_idr: i64,
    is_over_budget: bool,
    budget_status: &str,
    margin_alert_present: bool,
    forecast_unavailable: bool,
    forecast_has_revenue_signal: bool,
    forecast_variance_from_target_pct: f64,
    margin_alert_threshold_pct: f64,
) -> String {
    let forecast_has_margin_signal = !forecast_unavailable && forecast_has_revenue_signal;
    if total_budget_idr <= 0 && !margin_alert_present && !forecast_has_margin_signal {
        return "unconfigured".to_string();
    }
    if is_over_budget
        || margin_alert_present
        || budget_status == "critical"
        || (forecast_has_margin_signal
            && forecast_variance_from_target_pct < 0.0
            && forecast_variance_from_target_pct.abs() > margin_alert_threshold_pct)
    {
        return "critical".to_string();
    }
    if budget_status == "warning"
        || (forecast_unavailable && total_budget_idr > 0)
        || (forecast_has_margin_signal && forecast_variance_from_target_pct < 0.0)
    {
        return "warning".to_string();
    }
    "healthy".to_string()
}

// ─────────────────────────────────────────────────────────────────────────
// Finance
// ─────────────────────────────────────────────────────────────────────────

async fn build_finance_dashboard(pool: &PgPool) -> Result<FinanceDashboard> {
    let now = Utc::now();
    let today = now.date_naive();
    let mut warnings = Vec::new();

    let cash_position = match build_cash_position_summary(pool, today).await {
        Ok(summary) => summary,
        Err(e) => {
            tracing::warn!("dashboard cash position unavailable: {}", e);
            warnings.push("Cash position is temporarily unavailable.".to_string());
            default_cash_position(today)
        }
    };

    let ctc_validation = build_ctc_validation_status(pool, today).await?;
    if ctc_validation.status == "error" {
        warnings.push("CTC validation status is temporarily unavailable.".to_string());
    }
    let audit_alerts = match build_audit_alerts_summary(pool, now).await {
        Ok(summary) => summary,
        Err(e) => {
            tracing::warn!("dashboard audit alerts unavailable: {}", e);
            warnings.push("Audit alerts are temporarily unavailable.".to_string());
            default_audit_alerts_summary(now)
        }
    };
    let export_requests = match build_export_requests_summary(pool).await {
        Ok(summary) => summary,
        Err(e) => {
            tracing::warn!("dashboard export requests unavailable: {}", e);
            warnings.push("Export request status is temporarily unavailable.".to_string());
            ExportRequestsSummary {
                pending_count: 0,
                latest_pending: Vec::new(),
            }
        }
    };

    Ok(FinanceDashboard {
        cash_position,
        ctc_validation,
        audit_alerts,
        export_requests,
        warnings,
    })
}

async fn build_cash_position_summary(
    pool: &PgPool,
    today: NaiveDate,
) -> Result<CashPositionSummary> {
    let cash_start = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(today);
    let row = sqlx::query(
        r#"
        SELECT
            COALESCE(SUM(CASE WHEN entry_type = 'cash_in' THEN amount_idr ELSE 0 END), 0)::BIGINT AS total_cash_in_idr,
            COALESCE(SUM(CASE WHEN entry_type = 'cash_out' THEN amount_idr ELSE 0 END), 0)::BIGINT AS total_cash_out_idr
        FROM cash_flow_entries
        WHERE entry_date >= $1
          AND entry_date <= $2
        "#,
    )
    .bind(cash_start)
    .bind(today)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let total_cash_in_idr: i64 = row
        .try_get("total_cash_in_idr")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let total_cash_out_idr: i64 = row
        .try_get("total_cash_out_idr")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let net_cash_flow_idr = total_cash_in_idr - total_cash_out_idr;

    Ok(CashPositionSummary {
        start_date: cash_start,
        end_date: today,
        total_cash_in_idr,
        total_cash_out_idr,
        net_cash_flow_idr,
        ending_cumulative_position_idr: net_cash_flow_idr,
    })
}

fn default_cash_position(today: NaiveDate) -> CashPositionSummary {
    CashPositionSummary {
        start_date: NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(today),
        end_date: today,
        total_cash_in_idr: 0,
        total_cash_out_idr: 0,
        net_cash_flow_idr: 0,
        ending_cumulative_position_idr: 0,
    }
}

async fn build_ctc_validation_status(
    pool: &PgPool,
    today: NaiveDate,
) -> Result<CtcValidationStatus> {
    let start = today
        .checked_sub_signed(chrono::Duration::days(30))
        .unwrap_or(today);

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let filters = ValidationReportFilters {
        start_date: start,
        end_date: today,
        limit: Some(1),
        offset: Some(0),
        employee_ids: None,
    };

    match generate_validation_report(&mut tx, filters).await {
        Ok(report) => {
            tx.commit()
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(CtcValidationStatus {
                status: "ok".to_string(),
                start_date: start,
                end_date: today,
                total_compared: Some(report.total_compared),
                total_matches: Some(report.total_matches),
                total_discrepancies: Some(report.total_discrepancies),
                match_rate_pct: Some(report.match_rate_pct),
                message: None,
            })
        }
        Err(AppError::Validation(message)) => {
            tx.rollback()
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(CtcValidationStatus {
                status: "no_data".to_string(),
                start_date: start,
                end_date: today,
                total_compared: None,
                total_matches: None,
                total_discrepancies: None,
                match_rate_pct: None,
                message: Some(message),
            })
        }
        Err(e) => {
            tracing::warn!("dashboard CTC validation unavailable: {}", e);
            tx.rollback().await.ok();
            Ok(CtcValidationStatus {
                status: "error".to_string(),
                start_date: start,
                end_date: today,
                total_compared: None,
                total_matches: None,
                total_discrepancies: None,
                match_rate_pct: None,
                message: Some("CTC validation is temporarily unavailable.".to_string()),
            })
        }
    }
}

async fn build_audit_alerts_summary(
    pool: &PgPool,
    now: DateTime<Utc>,
) -> Result<AuditAlertsSummary> {
    let since = now - chrono::Duration::days(AUDIT_WINDOW_DAYS);

    let counts_row = sqlx::query(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE action = 'ACCESS_DENIED') AS access_denied_count,
            COUNT(*) FILTER (WHERE action = 'LOGIN_FAILED') AS login_failed_count,
            COUNT(*) FILTER (WHERE action = 'LOGIN_BLOCKED') AS login_blocked_count,
            COUNT(*) FILTER (WHERE action IN ('CHAIN_VERIFICATION_FAILED', 'VERIFY_CHAIN_FAILED')) AS chain_failed_count
        FROM audit_logs
        WHERE created_at >= $1
        "#,
    )
    .bind(since)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let access_denied_count: i64 = counts_row
        .try_get("access_denied_count")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let login_failed_count: i64 = counts_row
        .try_get("login_failed_count")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let login_blocked_count: i64 = counts_row
        .try_get("login_blocked_count")
        .map_err(|e| AppError::Database(e.to_string()))?;
    let chain_verification_failure_count: i64 = counts_row
        .try_get("chain_failed_count")
        .map_err(|e| AppError::Database(e.to_string()))?;

    let recent_rows = sqlx::query(
        r#"
        SELECT id, action, entity_type, created_at
        FROM audit_logs
        WHERE created_at >= $1
          AND action IN ('ACCESS_DENIED', 'LOGIN_FAILED', 'LOGIN_BLOCKED',
                         'CHAIN_VERIFICATION_FAILED', 'VERIFY_CHAIN_FAILED')
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(since)
    .bind(AUDIT_RECENT_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut recent = Vec::with_capacity(recent_rows.len());
    for row in recent_rows {
        recent.push(AuditAlertEntry {
            id: row
                .try_get("id")
                .map_err(|e| AppError::Database(e.to_string()))?,
            action: row
                .try_get("action")
                .map_err(|e| AppError::Database(e.to_string()))?,
            entity_type: row
                .try_get("entity_type")
                .map_err(|e| AppError::Database(e.to_string()))?,
            created_at: row
                .try_get("created_at")
                .map_err(|e| AppError::Database(e.to_string()))?,
        });
    }

    Ok(AuditAlertsSummary {
        since,
        access_denied_count,
        login_failed_count,
        login_blocked_count,
        chain_verification_failure_count,
        recent,
    })
}

fn default_audit_alerts_summary(now: DateTime<Utc>) -> AuditAlertsSummary {
    AuditAlertsSummary {
        since: now - chrono::Duration::days(AUDIT_WINDOW_DAYS),
        access_denied_count: 0,
        login_failed_count: 0,
        login_blocked_count: 0,
        chain_verification_failure_count: 0,
        recent: Vec::new(),
    }
}

async fn build_export_requests_summary(pool: &PgPool) -> Result<ExportRequestsSummary> {
    let pending_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_export_requests WHERE status = 'pending_approval'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let rows = sqlx::query(
        r#"
        SELECT id, requested_by, status, created_at, report_type
        FROM audit_export_requests
        WHERE status = 'pending_approval'
        ORDER BY created_at DESC
        LIMIT $1
        "#,
    )
    .bind(EXPORT_REQUESTS_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut latest_pending = Vec::with_capacity(rows.len());
    for row in rows {
        latest_pending.push(PendingExportRequest {
            id: row
                .try_get("id")
                .map_err(|e| AppError::Database(e.to_string()))?,
            requested_by: row
                .try_get("requested_by")
                .map_err(|e| AppError::Database(e.to_string()))?,
            status: row
                .try_get("status")
                .map_err(|e| AppError::Database(e.to_string()))?,
            created_at: row
                .try_get("created_at")
                .map_err(|e| AppError::Database(e.to_string()))?,
            report_type: row
                .try_get("report_type")
                .map_err(|e| AppError::Database(e.to_string()))?,
        });
    }

    Ok(ExportRequestsSummary {
        pending_count,
        latest_pending,
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Admin
// ─────────────────────────────────────────────────────────────────────────

async fn build_admin_dashboard(pool: &PgPool) -> Result<AdminDashboard> {
    let now = Utc::now();
    let since = now - chrono::Duration::days(1);

    let row = sqlx::query(
        r#"
        SELECT
            (SELECT COUNT(*) FROM users) AS total_users,
            (SELECT COUNT(*) FROM departments) AS total_departments,
            (SELECT COUNT(*) FROM projects WHERE status IN ('Active', 'active')) AS total_active_projects,
            (SELECT COUNT(*) FROM ctc_records WHERE status = 'Active') AS total_active_ctc_records,
            (SELECT COUNT(*) FROM audit_export_requests WHERE status = 'pending_approval') AS pending_export_requests,
            (SELECT COUNT(*) FROM audit_logs WHERE action = 'ACCESS_DENIED' AND created_at >= $1) AS access_denied_24h
        "#,
    )
    .bind(since)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(AdminDashboard {
        total_users: row
            .try_get("total_users")
            .map_err(|e| AppError::Database(e.to_string()))?,
        total_departments: row
            .try_get("total_departments")
            .map_err(|e| AppError::Database(e.to_string()))?,
        total_active_projects: row
            .try_get("total_active_projects")
            .map_err(|e| AppError::Database(e.to_string()))?,
        total_active_ctc_records: row
            .try_get("total_active_ctc_records")
            .map_err(|e| AppError::Database(e.to_string()))?,
        pending_export_requests: row
            .try_get("pending_export_requests")
            .map_err(|e| AppError::Database(e.to_string()))?,
        access_denied_24h: row
            .try_get("access_denied_24h")
            .map_err(|e| AppError::Database(e.to_string()))?,
    })
}
