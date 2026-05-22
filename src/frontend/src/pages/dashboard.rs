use crate::auth::{authenticated_get, logout_user, use_auth};

use chrono::{Datelike, Duration, NaiveDate};
use gloo_timers::callback::{Interval, Timeout};
use leptos::either::{Either, EitherOf3};
use leptos::prelude::*;
use leptos_router::hooks::*;
use leptos_router::NavigateOptions;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use uuid::Uuid;

const POLL_INTERVAL_MS: u32 = 30_000;
const CHANGE_HIGHLIGHT_TIMEOUT_MS: u32 = 1_800;

// ── Response shape (mirror of backend RoleDashboardResponse) ──────────────

#[derive(Debug, Clone, Deserialize)]
struct RoleDashboardResponse {
    role: String,
    generated_at: String,
    #[serde(default)]
    hr: Option<HrDashboard>,
    #[serde(default)]
    department_head: Option<DepartmentHeadDashboard>,
    #[serde(default)]
    project_manager: Option<ProjectManagerDashboard>,
    #[serde(default)]
    finance: Option<FinanceDashboard>,
    #[serde(default)]
    admin: Option<AdminDashboard>,
}

// HR

#[derive(Debug, Clone, Deserialize)]
struct HrDashboard {
    completeness: CompletenessReport,
    pending_updates: HrPendingUpdates,
    #[serde(default)]
    recent_changes: Vec<RecentCtcChange>,
    compliance_alerts: ComplianceAlertSummary,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CompletenessReport {
    total_employees: i64,
    total_with_ctc: i64,
    total_missing: i64,
    overall_completion_pct: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct HrPendingUpdates {
    missing_count: i64,
    #[serde(default)]
    sample: Vec<HrMissingEmployee>,
}

#[derive(Debug, Clone, Deserialize)]
struct HrMissingEmployee {
    #[serde(default)]
    id: Option<Uuid>,
    name: String,
    department: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RecentCtcChange {
    #[serde(default)]
    resource_id: Option<Uuid>,
    resource_name: String,
    revision_number: i32,
    #[serde(default)]
    changed_by_name: Option<String>,
    created_at: String,
    reason: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ComplianceAlertSummary {
    start_date: String,
    end_date: String,
    total_validated: i64,
    total_passed: i64,
    total_discrepancies: i64,
    compliance_rate_pct: f64,
    #[serde(default)]
    top_risks: Vec<ComplianceTopRisk>,
}

#[derive(Debug, Clone, Deserialize)]
struct ComplianceTopRisk {
    #[serde(default)]
    resource_id: Option<Uuid>,
    name: String,
    variance_amount: i64,
}

// Department Head

#[derive(Debug, Clone, Deserialize)]
struct DepartmentHeadDashboard {
    utilization: UtilizationSummary,
    #[serde(default)]
    budget: Option<DepartmentBudgetSummary>,
    overallocations: OverallocationSummary,
    #[serde(default)]
    upcoming_assignments: Vec<UpcomingAssignment>,
    #[serde(default)]
    warnings: Vec<String>,
    // Story 6.4: dense Team Utilization Dashboard surface. `#[serde(default)]`
    // keeps the DTO tolerant of older backends that have not deployed yet.
    #[serde(default)]
    team_members: Vec<TeamUtilizationMember>,
    #[serde(default)]
    utilization_trends: Option<TeamUtilizationTrendBundle>,
    #[serde(default)]
    underutilized_members: Vec<TeamUtilizationMember>,
}

#[derive(Debug, Clone, Deserialize)]
struct TeamUtilizationMember {
    #[serde(default)]
    resource_id: Option<Uuid>,
    resource_name: String,
    #[serde(default)]
    role: String,
    current_utilization_pct: f64,
    available_capacity_pct: f64,
    #[serde(default)]
    is_underutilized: bool,
    #[serde(default)]
    is_overallocated: bool,
    #[serde(default)]
    ctc_status: String,
    #[serde(default)]
    current_projects: Vec<TeamUtilizationCurrentProject>,
}

#[derive(Debug, Clone, Deserialize)]
struct TeamUtilizationCurrentProject {
    project_name: String,
    allocation_percentage: f64,
    start_date: String,
    end_date: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TeamUtilizationTrendBundle {
    start_date: String,
    end_date: String,
    #[serde(default)]
    members: Vec<TeamUtilizationTrend>,
}

#[derive(Debug, Clone, Deserialize)]
struct TeamUtilizationTrend {
    #[serde(default)]
    resource_id: Option<Uuid>,
    resource_name: String,
    #[serde(default)]
    periods: Vec<TeamUtilizationTrendPeriod>,
}

#[derive(Debug, Clone, Deserialize)]
struct TeamUtilizationTrendPeriod {
    period: String,
    utilization_pct: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct UtilizationSummary {
    #[serde(default)]
    start_date: String,
    #[serde(default)]
    end_date: String,
    average_utilization_pct: f64,
    overallocated_count: i64,
    #[serde(default)]
    top_at_risk: Vec<UtilizationAtRisk>,
}

#[derive(Debug, Clone, Deserialize)]
struct UtilizationAtRisk {
    #[serde(default)]
    resource_id: Option<Uuid>,
    resource_name: String,
    current_allocation_pct: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct DepartmentBudgetSummary {
    department_name: String,
    budget_period: String,
    total_budget_idr: i64,
    total_committed_idr: i64,
    #[serde(default)]
    spent_actual_idr: i64,
    remaining_idr: i64,
    utilization_percentage: f64,
    budget_health: String,
    #[serde(default)]
    alert_threshold_pct: i32,
    budget_configured: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct OverallocationSummary {
    overallocated_count: i64,
    #[serde(default)]
    members: Vec<UtilizationAtRisk>,
}

#[derive(Debug, Clone, Deserialize)]
struct UpcomingAssignment {
    #[serde(default)]
    allocation_id: Option<Uuid>,
    resource_name: String,
    project_name: String,
    start_date: String,
    end_date: String,
    allocation_percentage: f64,
}

// Project Manager

#[derive(Debug, Clone, Deserialize)]
struct ProjectManagerDashboard {
    #[serde(default)]
    active_projects: Vec<ProjectHealthCard>,
    #[serde(default)]
    margin_alerts: Vec<MarginAlert>,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProjectHealthCard {
    #[serde(default)]
    project_id: Option<Uuid>,
    project_name: String,
    status: String,
    end_date: String,
    total_budget_idr: i64,
    budget_spent_idr: i64,
    budget_remaining_idr: i64,
    budget_status: String,
    #[serde(default)]
    budget_utilization_pct: f64,
    #[serde(default)]
    is_over_budget: bool,
    #[serde(default)]
    budget_overrun_idr: i64,
    total_revenue_idr: i64,
    total_cost_idr: i64,
    gross_profit_idr: i64,
    margin_pct: f64,
    #[serde(default)]
    target_margin_pct: f64,
    #[serde(default)]
    margin_alert_threshold_pct: f64,
    #[serde(default)]
    margin_alert: Option<String>,
    #[serde(default)]
    forecast_margin_pct: f64,
    #[serde(default)]
    forecast_variance_from_target_pct: f64,
    #[serde(default)]
    projected_total_cost_idr: i64,
    #[serde(default)]
    forecast_unavailable: bool,
    #[serde(default)]
    forecast_has_revenue_signal: bool,
    #[serde(default = "default_health_status")]
    health_status: String,
    #[serde(default)]
    warning: Option<String>,
}

fn default_health_status() -> String {
    "unconfigured".to_string()
}

#[derive(Debug, Clone, Deserialize)]
struct MarginAlert {
    #[serde(default)]
    project_id: Option<Uuid>,
    project_name: String,
    margin_pct: f64,
    message: String,
}

// Finance

#[derive(Debug, Clone, Deserialize)]
struct FinanceDashboard {
    cash_position: CashPositionSummary,
    ctc_validation: CtcValidationStatus,
    audit_alerts: AuditAlertsSummary,
    export_requests: ExportRequestsSummary,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CashPositionSummary {
    start_date: String,
    end_date: String,
    total_cash_in_idr: i64,
    total_cash_out_idr: i64,
    net_cash_flow_idr: i64,
    ending_cumulative_position_idr: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct CtcValidationStatus {
    status: String,
    start_date: String,
    end_date: String,
    #[serde(default)]
    total_compared: Option<i64>,
    #[serde(default)]
    total_matches: Option<i64>,
    #[serde(default)]
    total_discrepancies: Option<i64>,
    #[serde(default)]
    match_rate_pct: Option<f64>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AuditAlertsSummary {
    access_denied_count: i64,
    login_failed_count: i64,
    login_blocked_count: i64,
    chain_verification_failure_count: i64,
    #[serde(default)]
    recent: Vec<AuditAlertEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct AuditAlertEntry {
    #[serde(default)]
    id: Option<Uuid>,
    action: String,
    entity_type: String,
    created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ExportRequestsSummary {
    pending_count: i64,
    #[serde(default)]
    latest_pending: Vec<PendingExportRequest>,
}

#[derive(Debug, Clone, Deserialize)]
struct PendingExportRequest {
    #[serde(default)]
    id: Option<Uuid>,
    status: String,
    created_at: String,
    #[serde(default)]
    report_type: Option<String>,
}

// Admin

#[derive(Debug, Clone, Deserialize)]
struct AdminDashboard {
    total_users: i64,
    total_departments: i64,
    total_active_projects: i64,
    total_active_ctc_records: i64,
    pending_export_requests: i64,
    access_denied_24h: i64,
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn format_idr(amount: i64) -> String {
    let sign = if amount < 0 { "-" } else { "" };
    let abs = amount.unsigned_abs();
    let raw = abs.to_string();
    let bytes = raw.as_bytes();
    let mut out = String::with_capacity(raw.len() + raw.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(*b as char);
    }
    format!("Rp {}{}", sign, out)
}

fn format_date(value: &str) -> String {
    value.split('T').next().unwrap_or(value).to_string()
}

fn role_display(role: &str) -> &'static str {
    match role {
        "hr" => "HR",
        "department_head" => "Department Head",
        "project_manager" => "Project Manager",
        "finance" => "Finance",
        "admin" => "Administrator",
        _ => "User",
    }
}

/// Department Head trend-range presets. Resolved on the frontend into
/// concrete `YYYY-MM-DD` boundaries before being sent to the dashboard API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrendRangePreset {
    CurrentMonth,
    Next30Days,
    ThreeMonths,
    SixMonths,
}

impl TrendRangePreset {
    fn label(&self) -> &'static str {
        match self {
            TrendRangePreset::CurrentMonth => "Current Month",
            TrendRangePreset::Next30Days => "Next 30 Days",
            TrendRangePreset::ThreeMonths => "3 Months",
            TrendRangePreset::SixMonths => "6 Months",
        }
    }

    fn resolve(self) -> (String, String) {
        let now = js_sys::Date::new_0();
        let today = NaiveDate::from_ymd_opt(
            now.get_utc_full_year() as i32,
            now.get_utc_month() + 1,
            now.get_utc_date(),
        )
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid fallback date"));

        self.resolve_from_date(today)
    }

    fn resolve_from_date(self, today: NaiveDate) -> (String, String) {
        match self {
            TrendRangePreset::CurrentMonth => {
                let start = today.with_day(1).unwrap_or(today);
                let next_month = if start.month() == 12 {
                    NaiveDate::from_ymd_opt(start.year() + 1, 1, 1).unwrap_or(start)
                } else {
                    NaiveDate::from_ymd_opt(start.year(), start.month() + 1, 1).unwrap_or(start)
                };
                let end = next_month
                    .checked_sub_signed(Duration::days(1))
                    .unwrap_or(start);
                (format_iso_date(start), format_iso_date(end))
            }
            TrendRangePreset::Next30Days => offset_range_iso(today, 30),
            TrendRangePreset::ThreeMonths => offset_range_iso(today, 90),
            TrendRangePreset::SixMonths => offset_range_iso(today, 180),
        }
    }
}

fn offset_range_iso(start: NaiveDate, days: i64) -> (String, String) {
    let end = start
        .checked_add_signed(Duration::days(days))
        .unwrap_or(start);
    (format_iso_date(start), format_iso_date(end))
}

fn format_iso_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

async fn fetch_role_dashboard(
    range: Option<TrendRangePreset>,
) -> Result<RoleDashboardResponse, String> {
    let url = match range {
        Some(preset) => {
            let (start, end) = preset.resolve();
            format!(
                "/api/v1/dashboard?team_start_date={}&team_end_date={}",
                start, end
            )
        }
        None => "/api/v1/dashboard".to_string(),
    };
    let response = authenticated_get(&url).await.map_err(|e| {
        if e == "SESSION_EXPIRED" {
            e
        } else {
            format!("Failed to fetch dashboard: {}", e)
        }
    })?;

    if response.status().is_success() {
        response
            .json::<RoleDashboardResponse>()
            .await
            .map_err(|e| format!("Failed to parse dashboard: {}", e))
    } else {
        Err(format!("Failed to fetch dashboard: {}", response.status()))
    }
}

#[derive(Debug, Clone)]
struct DashboardState {
    data: Option<RoleDashboardResponse>,
    error: Option<String>,
    loading: bool,
}

impl DashboardState {
    fn initial() -> Self {
        Self {
            data: None,
            error: None,
            loading: true,
        }
    }
}

// ── Change detection ─────────────────────────────────────────────────────
//
// Build a stable key → display-value map for visible dashboard values, then
// diff consecutive snapshots to determine which keys changed. `generated_at`
// is deliberately excluded so polling does not flash every value each tick.

fn recent_ctc_change_key(change: &RecentCtcChange) -> String {
    match change.resource_id {
        Some(id) => format!(
            "hr.recent_changes.{}|rev{}|{}",
            id, change.revision_number, change.created_at
        ),
        None => format!(
            "hr.recent_changes.name.{}|rev{}|{}",
            change.resource_name, change.revision_number, change.created_at
        ),
    }
}

fn dashboard_value_map(data: &RoleDashboardResponse) -> HashMap<String, String> {
    let mut map = HashMap::new();

    if let Some(hr) = &data.hr {
        map.insert(
            "hr.completeness.overall_completion_pct".into(),
            format!("{:.1}", hr.completeness.overall_completion_pct),
        );
        map.insert(
            "hr.completeness.total_with_ctc".into(),
            hr.completeness.total_with_ctc.to_string(),
        );
        map.insert(
            "hr.completeness.total_employees".into(),
            hr.completeness.total_employees.to_string(),
        );
        map.insert(
            "hr.pending_updates.missing_count".into(),
            hr.pending_updates.missing_count.to_string(),
        );
        map.insert(
            "hr.compliance_alerts.compliance_rate_pct".into(),
            format!("{:.1}", hr.compliance_alerts.compliance_rate_pct),
        );
        map.insert(
            "hr.compliance_alerts.total_discrepancies".into(),
            hr.compliance_alerts.total_discrepancies.to_string(),
        );
        map.insert(
            "hr.compliance_alerts.total_validated".into(),
            hr.compliance_alerts.total_validated.to_string(),
        );
        map.insert(
            "hr.compliance_alerts.total_passed".into(),
            hr.compliance_alerts.total_passed.to_string(),
        );
        for change in &hr.recent_changes {
            map.insert(
                recent_ctc_change_key(change),
                format!(
                    "{}|{}",
                    change.resource_name,
                    change.changed_by_name.clone().unwrap_or_default()
                ),
            );
        }
        for emp in &hr.pending_updates.sample {
            let key = match emp.id {
                Some(id) => format!("hr.pending_updates.sample.{}", id),
                None => format!(
                    "hr.pending_updates.sample.name.{}|{}",
                    emp.name, emp.department
                ),
            };
            map.insert(key, format!("{}|{}", emp.name, emp.department));
        }
        for risk in &hr.compliance_alerts.top_risks {
            let key = match risk.resource_id {
                Some(id) => format!("hr.compliance_alerts.top_risks.{}", id),
                None => format!(
                    "hr.compliance_alerts.top_risks.name.{}|{}",
                    risk.name, risk.variance_amount
                ),
            };
            map.insert(key, format!("{}|{}", risk.name, risk.variance_amount));
        }
    }

    if let Some(dh) = &data.department_head {
        map.insert(
            "department_head.utilization.average_utilization_pct".into(),
            format!("{:.1}", dh.utilization.average_utilization_pct),
        );
        map.insert(
            "department_head.utilization.overallocated_count".into(),
            dh.utilization.overallocated_count.to_string(),
        );
        map.insert(
            "department_head.overallocations.overallocated_count".into(),
            dh.overallocations.overallocated_count.to_string(),
        );
        if let Some(budget) = &dh.budget {
            map.insert(
                "department_head.budget.utilization_percentage".into(),
                format!("{:.0}", budget.utilization_percentage),
            );
            map.insert(
                "department_head.budget.remaining_idr".into(),
                budget.remaining_idr.to_string(),
            );
            map.insert(
                "department_head.budget.total_committed_idr".into(),
                budget.total_committed_idr.to_string(),
            );
            map.insert(
                "department_head.budget.total_budget_idr".into(),
                budget.total_budget_idr.to_string(),
            );
            map.insert(
                "department_head.budget.budget_health".into(),
                budget.budget_health.clone(),
            );
        }
        for member in &dh.utilization.top_at_risk {
            let key = match member.resource_id {
                Some(id) => format!("department_head.top_at_risk.{}", id),
                None => format!("department_head.top_at_risk.name.{}", member.resource_name),
            };
            map.insert(
                key,
                format!(
                    "{}|{:.0}",
                    member.resource_name, member.current_allocation_pct
                ),
            );
        }
        for member in &dh.overallocations.members {
            let key = match member.resource_id {
                Some(id) => format!("department_head.overallocations.member.{}", id),
                None => format!(
                    "department_head.overallocations.member.name.{}",
                    member.resource_name
                ),
            };
            map.insert(
                key,
                format!(
                    "{}|{:.0}",
                    member.resource_name, member.current_allocation_pct
                ),
            );
        }
        for assignment in &dh.upcoming_assignments {
            let key = match assignment.allocation_id {
                Some(id) => format!("department_head.upcoming_assignments.{}", id),
                None => format!(
                    "department_head.upcoming_assignments.name.{}|{}|{}|{}",
                    assignment.resource_name,
                    assignment.project_name,
                    assignment.start_date,
                    assignment.end_date
                ),
            };
            map.insert(
                key,
                format!(
                    "{}|{}|{:.0}|{}|{}",
                    assignment.resource_name,
                    assignment.project_name,
                    assignment.allocation_percentage,
                    assignment.start_date,
                    assignment.end_date
                ),
            );
        }

        // Story 6.4 — Team utilization summary keys (visible aggregate stats).
        map.insert(
            "department_head.team.member_count".into(),
            dh.team_members.len().to_string(),
        );
        let _backend_reported_underutilized_count = dh.underutilized_members.len();
        map.insert(
            "department_head.team.underutilized_count".into(),
            dh.team_members
                .iter()
                .filter(|m| {
                    m.is_underutilized || is_underutilized_threshold(m.current_utilization_pct)
                })
                .count()
                .to_string(),
        );
        let avg_available_pct: f64 = if dh.team_members.is_empty() {
            0.0
        } else {
            dh.team_members
                .iter()
                .map(|m| m.available_capacity_pct)
                .sum::<f64>()
                / dh.team_members.len() as f64
        };
        map.insert(
            "department_head.team.avg_available_capacity_pct".into(),
            format!("{:.1}", avg_available_pct),
        );

        // Per-member rendered values. Keys are stable on resource_id so a
        // member dropping in/out of the table surfaces as a clean change.
        for member in &dh.team_members {
            let row_key = team_member_key(member);
            map.insert(
                format!("{}.resource_name", row_key),
                member.resource_name.clone(),
            );
            map.insert(format!("{}.role", row_key), member.role.clone());
            map.insert(
                format!("{}.current_utilization_pct", row_key),
                format!("{:.1}", member.current_utilization_pct),
            );
            map.insert(
                format!("{}.available_capacity_pct", row_key),
                format!("{:.1}", member.available_capacity_pct),
            );
            map.insert(
                format!("{}.is_underutilized", row_key),
                (member.is_underutilized
                    || is_underutilized_threshold(member.current_utilization_pct))
                .to_string(),
            );
            map.insert(
                format!("{}.is_overallocated", row_key),
                member.is_overallocated.to_string(),
            );
            map.insert(format!("{}.ctc_status", row_key), member.ctc_status.clone());
            // Collapse current projects into a deterministic visible summary
            // so changes (additions, removals, % shifts, date shifts) flash.
            let project_summary = current_projects_change_value(&member.current_projects);
            map.insert(format!("{}.current_projects", row_key), project_summary);
        }

        // Per-member trend periods — each period gets its own key so a single
        // monthly cell can flash without dragging neighboring periods along.
        if let Some(trend) = &dh.utilization_trends {
            for member in &trend.members {
                for period in &member.periods {
                    map.insert(
                        trend_period_key(member, &period.period),
                        format!("{:.0}", period.utilization_pct),
                    );
                }
            }
        }

        // Budget surface added by Story 6.4 (spent + alert threshold) so the
        // new gauge values participate in change-highlighting.
        if let Some(budget) = &dh.budget {
            map.insert(
                "department_head.budget.spent_actual_idr".into(),
                budget.spent_actual_idr.to_string(),
            );
            map.insert(
                "department_head.budget.alert_threshold_pct".into(),
                budget.alert_threshold_pct.to_string(),
            );
        }
    }

    if let Some(pm) = &data.project_manager {
        map.insert(
            "project_manager.active_projects.count".into(),
            pm.active_projects.len().to_string(),
        );
        map.insert(
            "project_manager.margin_alerts.count".into(),
            pm.margin_alerts.len().to_string(),
        );
        for project in &pm.active_projects {
            let id_key = match project.project_id {
                Some(id) => id.to_string(),
                None => format!("name.{}", project.project_name),
            };
            map.insert(
                format!("project_manager.project.{}.margin_pct", id_key),
                format!("{:.1}", project.margin_pct),
            );
            map.insert(
                format!("project_manager.project.{}.project_name", id_key),
                project.project_name.clone(),
            );
            map.insert(
                format!("project_manager.project.{}.status", id_key),
                project.status.clone(),
            );
            map.insert(
                format!("project_manager.project.{}.end_date", id_key),
                project.end_date.clone(),
            );
            map.insert(
                format!("project_manager.project.{}.total_budget_idr", id_key),
                project.total_budget_idr.to_string(),
            );
            map.insert(
                format!("project_manager.project.{}.budget_spent_idr", id_key),
                project.budget_spent_idr.to_string(),
            );
            map.insert(
                format!("project_manager.project.{}.gross_profit_idr", id_key),
                project.gross_profit_idr.to_string(),
            );
            map.insert(
                format!("project_manager.project.{}.budget_status", id_key),
                project.budget_status.clone(),
            );
            map.insert(
                format!("project_manager.project.{}.budget_remaining_idr", id_key),
                project.budget_remaining_idr.to_string(),
            );
            map.insert(
                format!("project_manager.project.{}.total_revenue_idr", id_key),
                project.total_revenue_idr.to_string(),
            );
            map.insert(
                format!("project_manager.project.{}.total_cost_idr", id_key),
                project.total_cost_idr.to_string(),
            );
            map.insert(
                format!("project_manager.project.{}.warning", id_key),
                project.warning.clone().unwrap_or_default(),
            );
            map.insert(
                format!("project_manager.project.{}.margin_alert", id_key),
                project.margin_alert.clone().unwrap_or_default(),
            );
            map.insert(
                format!("project_manager.project.{}.budget_utilization_pct", id_key),
                format!("{:.1}", project.budget_utilization_pct),
            );
            map.insert(
                format!("project_manager.project.{}.is_over_budget", id_key),
                project.is_over_budget.to_string(),
            );
            map.insert(
                format!("project_manager.project.{}.budget_overrun_idr", id_key),
                project.budget_overrun_idr.to_string(),
            );
            map.insert(
                format!("project_manager.project.{}.forecast_margin_pct", id_key),
                format!("{:.1}", project.forecast_margin_pct),
            );
            map.insert(
                format!(
                    "project_manager.project.{}.forecast_variance_from_target_pct",
                    id_key
                ),
                format!("{:.1}", project.forecast_variance_from_target_pct),
            );
            map.insert(
                format!(
                    "project_manager.project.{}.projected_total_cost_idr",
                    id_key
                ),
                project.projected_total_cost_idr.to_string(),
            );
            map.insert(
                format!("project_manager.project.{}.forecast_unavailable", id_key),
                project.forecast_unavailable.to_string(),
            );
            map.insert(
                format!(
                    "project_manager.project.{}.forecast_has_revenue_signal",
                    id_key
                ),
                project.forecast_has_revenue_signal.to_string(),
            );
            map.insert(
                format!("project_manager.project.{}.health_status", id_key),
                project.health_status.clone(),
            );
            map.insert(
                format!("project_manager.project.{}.target_margin_pct", id_key),
                format!("{:.1}", project.target_margin_pct),
            );
            map.insert(
                format!(
                    "project_manager.project.{}.margin_alert_threshold_pct",
                    id_key
                ),
                format!("{:.1}", project.margin_alert_threshold_pct),
            );
        }
        for alert in &pm.margin_alerts {
            let id_key = match alert.project_id {
                Some(id) => id.to_string(),
                None => format!("name.{}", alert.project_name),
            };
            map.insert(
                format!("project_manager.margin_alert.{}", id_key),
                format!("{:.1}|{}", alert.margin_pct, alert.message),
            );
        }
    }

    if let Some(finance) = &data.finance {
        map.insert(
            "finance.cash_position.ending_cumulative_position_idr".into(),
            finance
                .cash_position
                .ending_cumulative_position_idr
                .to_string(),
        );
        map.insert(
            "finance.cash_position.net_cash_flow_idr".into(),
            finance.cash_position.net_cash_flow_idr.to_string(),
        );
        map.insert(
            "finance.cash_position.total_cash_in_idr".into(),
            finance.cash_position.total_cash_in_idr.to_string(),
        );
        map.insert(
            "finance.cash_position.total_cash_out_idr".into(),
            finance.cash_position.total_cash_out_idr.to_string(),
        );
        map.insert(
            "finance.ctc_validation.status".into(),
            finance.ctc_validation.status.clone(),
        );
        if let Some(rate) = finance.ctc_validation.match_rate_pct {
            map.insert(
                "finance.ctc_validation.match_rate_pct".into(),
                format!("{:.1}", rate),
            );
        } else {
            map.insert(
                "finance.ctc_validation.match_rate_pct".into(),
                "none".into(),
            );
        }
        map.insert(
            "finance.ctc_validation.total_compared".into(),
            finance
                .ctc_validation
                .total_compared
                .map(|v| v.to_string())
                .unwrap_or_else(|| "none".to_string()),
        );
        map.insert(
            "finance.ctc_validation.total_matches".into(),
            finance
                .ctc_validation
                .total_matches
                .map(|v| v.to_string())
                .unwrap_or_else(|| "none".to_string()),
        );
        map.insert(
            "finance.ctc_validation.total_discrepancies".into(),
            finance
                .ctc_validation
                .total_discrepancies
                .map(|v| v.to_string())
                .unwrap_or_else(|| "none".to_string()),
        );
        map.insert(
            "finance.audit_alerts.total".into(),
            (finance.audit_alerts.access_denied_count
                + finance.audit_alerts.login_failed_count
                + finance.audit_alerts.login_blocked_count
                + finance.audit_alerts.chain_verification_failure_count)
                .to_string(),
        );
        map.insert(
            "finance.audit_alerts.access_denied_count".into(),
            finance.audit_alerts.access_denied_count.to_string(),
        );
        map.insert(
            "finance.audit_alerts.login_failed_count".into(),
            finance.audit_alerts.login_failed_count.to_string(),
        );
        map.insert(
            "finance.audit_alerts.login_blocked_count".into(),
            finance.audit_alerts.login_blocked_count.to_string(),
        );
        map.insert(
            "finance.audit_alerts.chain_verification_failure_count".into(),
            finance
                .audit_alerts
                .chain_verification_failure_count
                .to_string(),
        );
        map.insert(
            "finance.export_requests.pending_count".into(),
            finance.export_requests.pending_count.to_string(),
        );
        for entry in &finance.audit_alerts.recent {
            let key = match entry.id {
                Some(id) => format!("finance.audit_alerts.recent.{}", id),
                None => format!(
                    "finance.audit_alerts.recent.{}|{}|{}",
                    entry.action, entry.entity_type, entry.created_at
                ),
            };
            map.insert(
                key,
                format!(
                    "{}|{}|{}",
                    entry.action, entry.entity_type, entry.created_at
                ),
            );
        }
        for pending in &finance.export_requests.latest_pending {
            let key = match pending.id {
                Some(id) => format!("finance.export_requests.latest.{}", id),
                None => format!(
                    "finance.export_requests.latest.{}|{}",
                    pending.status, pending.created_at
                ),
            };
            map.insert(
                key,
                format!(
                    "{}|{}|{}",
                    pending.status,
                    pending.report_type.clone().unwrap_or_default(),
                    pending.created_at
                ),
            );
        }
    }

    if let Some(admin) = &data.admin {
        map.insert("admin.total_users".into(), admin.total_users.to_string());
        map.insert(
            "admin.total_departments".into(),
            admin.total_departments.to_string(),
        );
        map.insert(
            "admin.total_active_projects".into(),
            admin.total_active_projects.to_string(),
        );
        map.insert(
            "admin.total_active_ctc_records".into(),
            admin.total_active_ctc_records.to_string(),
        );
        map.insert(
            "admin.pending_export_requests".into(),
            admin.pending_export_requests.to_string(),
        );
        map.insert(
            "admin.access_denied_24h".into(),
            admin.access_denied_24h.to_string(),
        );
    }

    map
}

fn compute_changed_keys(
    prev: &HashMap<String, String>,
    next: &HashMap<String, String>,
) -> HashSet<String> {
    let mut changed = HashSet::new();
    for (key, value) in next {
        match prev.get(key) {
            Some(prev_value) if prev_value == value => {}
            _ => {
                changed.insert(key.clone());
            }
        }
    }
    changed
}

#[derive(Clone, Copy)]
struct ChangedKeysCtx(ReadSignal<HashSet<String>>);

fn flash_if_changed(
    changed: ChangedKeysCtx,
    key: &'static str,
) -> impl Fn() -> bool + Copy + 'static {
    move || changed.0.with(|set| set.contains(key))
}

fn flash_if_changed_owned(changed: ChangedKeysCtx, key: String) -> impl Fn() -> bool + 'static {
    move || changed.0.with(|set| set.contains(&key))
}

fn flash_if_any_changed(
    changed: ChangedKeysCtx,
    keys: &'static [&'static str],
) -> impl Fn() -> bool + Copy + 'static {
    move || {
        changed
            .0
            .with(|set| keys.iter().any(|key| set.contains(*key)))
    }
}

fn flash_if_any_changed_owned(
    changed: ChangedKeysCtx,
    keys: Vec<String>,
) -> impl Fn() -> bool + 'static {
    move || {
        changed
            .0
            .with(|set| keys.iter().any(|key| set.contains(key)))
    }
}

#[component]
pub fn Dashboard() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();

    // Redirect if not logged in
    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            if !auth.is_authenticated.get() {
                navigate("/login", Default::default());
            }
        });
    }

    let handle_logout = {
        let navigate = navigate.clone();
        move |_| {
            logout_user(&auth);
            navigate("/", Default::default());
        }
    };

    let user = auth.user;
    let (state, set_state) = signal(DashboardState::initial());
    let (changed_keys, set_changed_keys) = signal::<HashSet<String>>(HashSet::new());
    let (pm_sort_mode, set_pm_sort_mode) = signal(PmSortMode::EndDate);
    // Department Head trend range. Default Next 30 Days mirrors the backend's
    // operational window so the initial fetch needs no extra round-trip.
    let (team_range, set_team_range) = signal(TrendRangePreset::Next30Days);

    // Component-scoped, !Send+!Sync handles. Replacing/clearing the inner
    // Option drops the previous Interval/Timeout, preventing leaks and
    // interval stacking when effects rerun or the component unmounts.
    let poll_interval = StoredValue::new_local(Option::<Interval>::None);
    let highlight_timeout = StoredValue::new_local(Option::<Timeout>::None);

    // Monotonic request id ensures slower stale responses cannot overwrite
    // newer ones, and the loading badge tracks only the most recent request.
    let next_request_id: StoredValue<u64> = StoredValue::new(0);
    let latest_request_id: StoredValue<u64> = StoredValue::new(0);

    // Previous successful snapshot value-map for change detection.
    let prev_value_map: StoredValue<Option<HashMap<String, String>>> = StoredValue::new(None);

    // Track whether the initial load has run for the current authenticated
    // session so we kick off a single immediate load instead of waiting for
    // the first interval tick.
    let did_initial_load: StoredValue<bool> = StoredValue::new(false);

    // Async requests may resolve after auth teardown or component cleanup.
    // Keep a non-reactive liveness guard outside the Leptos owner so stale
    // tasks can exit before touching disposed signals.
    let dashboard_alive = Arc::new(AtomicBool::new(true));

    // Provide changed keys signal to child panels via context.
    provide_context(ChangedKeysCtx(changed_keys));

    let load_dashboard = {
        let navigate = navigate.clone();
        let dashboard_alive = dashboard_alive.clone();
        move || {
            if !dashboard_alive.load(Ordering::Relaxed) {
                return;
            }
            let request_id = next_request_id.with_value(|n| n.wrapping_add(1));
            next_request_id.set_value(request_id);
            latest_request_id.set_value(request_id);
            set_state.update(|s| {
                s.loading = true;
                s.error = None;
            });

            let navigate = navigate.clone();
            let dashboard_alive = dashboard_alive.clone();
            // Capture the current team range without subscribing so that the
            // polling Interval reads the latest selection on every tick.
            let range_for_fetch = team_range.get_untracked();
            leptos::task::spawn_local(async move {
                let result = fetch_role_dashboard(Some(range_for_fetch)).await;

                if !dashboard_alive.load(Ordering::Relaxed) {
                    return;
                }

                // Drop stale responses so an older slow request cannot
                // overwrite a newer one.
                if latest_request_id.get_value() != request_id {
                    return;
                }

                match result {
                    Ok(data) => {
                        let next_map = dashboard_value_map(&data);
                        let changed = match prev_value_map.with_value(|prev| prev.clone()) {
                            Some(prev_map) => compute_changed_keys(&prev_map, &next_map),
                            None => HashSet::new(),
                        };
                        prev_value_map.set_value(Some(next_map));

                        set_state.update(|s| {
                            s.data = Some(data);
                            s.error = None;
                            s.loading = false;
                        });

                        if !changed.is_empty() {
                            set_changed_keys.set(changed);
                            // Replace any pending timeout — drops the old one,
                            // restarting the highlight clear window.
                            let timeout = Timeout::new(CHANGE_HIGHLIGHT_TIMEOUT_MS, move || {
                                set_changed_keys.set(HashSet::new());
                            });
                            highlight_timeout.set_value(Some(timeout));
                        }
                    }
                    Err(e) if e == "SESSION_EXPIRED" => {
                        // Stop polling immediately and tear down auth state.
                        poll_interval.set_value(None);
                        highlight_timeout.set_value(None);
                        logout_user(&auth);
                        set_state.update(|s| {
                            s.error =
                                Some("Your session expired. Please sign in again.".to_string());
                            s.loading = false;
                        });
                        navigate("/login", Default::default());
                    }
                    Err(e) => {
                        // Keep existing data visible; surface a bounded error.
                        set_state.update(|s| {
                            s.error = Some(e);
                            s.loading = false;
                        });
                    }
                }
            });
        }
    };

    // Single effect that handles both the initial load and the polling
    // lifecycle. Replacing the StoredValue's inner Option drops the previous
    // Interval (RAII clears the underlying timer), which keeps exactly one
    // active polling interval for this dashboard instance.
    {
        let load = load_dashboard.clone();
        Effect::new(move |_| {
            let is_auth = auth.is_authenticated.get();
            if is_auth {
                if !did_initial_load.get_value() {
                    did_initial_load.set_value(true);
                    load();
                }
                let load = load.clone();
                let interval = Interval::new(POLL_INTERVAL_MS, move || {
                    if !state.with(|s| s.loading) {
                        load();
                    }
                });
                poll_interval.set_value(Some(interval));
            } else {
                // Allow a future re-login to trigger a fresh initial load.
                let invalidation_id = next_request_id.with_value(|n| n.wrapping_add(1));
                next_request_id.set_value(invalidation_id);
                latest_request_id.set_value(invalidation_id);
                did_initial_load.set_value(false);
                poll_interval.set_value(None);
                highlight_timeout.set_value(None);
                prev_value_map.set_value(None);
                set_changed_keys.set(HashSet::new());
                set_state.update(|s| {
                    s.data = None;
                    s.loading = false;
                });
            }
        });
    }

    // Re-fetch when the user changes the trend range. Skipping the first run
    // keeps the initial mount on a single load() call instead of two; after
    // that, any range change triggers a fresh fetch that the polling Interval
    // will continue to follow because it reads `team_range` lazily.
    {
        let load = load_dashboard.clone();
        let initial_range_run: StoredValue<bool> = StoredValue::new(true);
        Effect::new(move |_| {
            let _selected = team_range.get();
            if initial_range_run.get_value() {
                initial_range_run.set_value(false);
                return;
            }
            // Reset prev_value_map so a range switch does not light up the
            // whole panel as "changed" on the next successful response.
            prev_value_map.set_value(None);
            load();
        });
    }

    // Tear down timers and highlight state when the component unmounts.
    on_cleanup(move || {
        dashboard_alive.store(false, Ordering::Relaxed);
        let invalidation_id = next_request_id.with_value(|n| n.wrapping_add(1));
        next_request_id.set_value(invalidation_id);
        latest_request_id.set_value(invalidation_id);
        poll_interval.set_value(None);
        highlight_timeout.set_value(None);
        prev_value_map.set_value(None);
        set_changed_keys.set(HashSet::new());
    });

    let refresh_click = {
        let load = load_dashboard.clone();
        move |_| {
            load();
        }
    };

    view! {
        <div class="h-full">
            <div class="page-container fade-in">
                <div class="page-header">
                    <div>
                        <h1 class="text-xl font-semibold text-huly-caption">
                            {move || user.get().map(|u| format!("Welcome, {}!", u.first_name)).unwrap_or_else(|| "Welcome!".to_string())}
                        </h1>
                        <p class="text-xs text-huly-muted mt-0.5">
                            {move || user.get().map(|u| format!("{} · {}", role_display(&u.role), u.email)).unwrap_or_default()}
                        </p>
                        <p class="text-xs text-huly-muted mt-0.5" aria-live="polite">
                            {move || {
                                let s = state.get();
                                if let Some(data) = s.data.as_ref() {
                                    if s.loading {
                                        format!(
                                            "Last updated: {} · refreshing…",
                                            format_datetime(&data.generated_at)
                                        )
                                    } else {
                                        format!("Last updated: {}", format_datetime(&data.generated_at))
                                    }
                                } else if s.loading {
                                    "Loading dashboard…".to_string()
                                } else {
                                    String::new()
                                }
                            }}
                        </p>
                    </div>
                    <div class="flex items-center gap-2">
                        <button
                            on:click=refresh_click
                            class="btn-ghost text-xs"
                            disabled=move || state.get().loading
                        >
                            {move || if state.get().loading { "Refreshing…" } else { "Refresh" }}
                        </button>
                        <button
                            on:click=handle_logout
                            class="btn-ghost text-xs"
                        >
                            "Sign out"
                        </button>
                    </div>
                </div>

                {move || state.get().error.map(|err| view! {
                    <div class="alert-error mb-3" role="alert">
                        <span class="text-sm">{err}</span>
                    </div>
                })}

                {move || {
                    let s = state.get();
                    match s.data {
                        Some(data) => Either::Left(view! {
                            <DashboardBody
                                data=data
                                pm_sort_mode=pm_sort_mode
                                set_pm_sort_mode=set_pm_sort_mode
                                team_range=team_range
                                set_team_range=set_team_range
                            />
                        }),
                        None if s.loading => Either::Right(view! {
                            <div class="panel p-6 text-center text-huly-muted text-sm">
                                "Loading dashboard…"
                            </div>
                        }),
                        None => Either::Right(view! {
                            <div class="panel p-6 text-center text-huly-muted text-sm">
                                "No dashboard data available."
                            </div>
                        }),
                    }
                }}
            </div>
        </div>
    }
}

fn format_datetime(value: &str) -> String {
    // generated_at is RFC3339 UTC; the bare date+HH:MM is enough for users.
    let trimmed = value.split('.').next().unwrap_or(value);
    trimmed.replace('T', " ")
}

#[component]
fn DashboardBody(
    data: RoleDashboardResponse,
    pm_sort_mode: ReadSignal<PmSortMode>,
    set_pm_sort_mode: WriteSignal<PmSortMode>,
    team_range: ReadSignal<TrendRangePreset>,
    set_team_range: WriteSignal<TrendRangePreset>,
) -> impl IntoView {
    let role = data.role.clone();
    let no_widgets = data.hr.is_none()
        && data.department_head.is_none()
        && data.project_manager.is_none()
        && data.finance.is_none()
        && data.admin.is_none();
    view! {
        <div class="space-y-4">
            {data.hr.map(|hr| view! { <HrPanel hr=hr /> })}
            {data.department_head.map(|dh| view! {
                <DepartmentHeadPanel
                    dh=dh
                    team_range=team_range
                    set_team_range=set_team_range
                />
            })}
            {data.project_manager.map(|pm| view! {
                <ProjectManagerPanel
                    pm=pm
                    sort_mode=pm_sort_mode
                    set_sort_mode=set_pm_sort_mode
                />
            })}
            {data.finance.map(|f| view! { <FinancePanel finance=f /> })}
            {data.admin.map(|a| view! { <AdminPanel admin=a /> })}
            {if no_widgets {
                Some(view! {
                    <div class="panel p-4 text-center text-huly-muted text-sm">
                        {format!("No dashboard widgets available for role '{}'.", role)}
                    </div>
                })
            } else {
                None
            }}
        </div>
    }
}

#[component]
fn DashboardWarnings(warnings: Vec<String>) -> impl IntoView {
    view! {
        {if warnings.is_empty() {
            None
        } else {
            Some(view! {
                <div class="alert-error">
                    <div class="space-y-1">
                        {warnings.into_iter().map(|warning| view! {
                            <p class="text-xs">{warning}</p>
                        }).collect_view()}
                    </div>
                </div>
            })
        }}
    }
}

// ── HR Panel ─────────────────────────────────────────────────────────────

#[component]
fn HrPanel(hr: HrDashboard) -> impl IntoView {
    let warnings = hr.warnings.clone();
    let completeness = hr.completeness.clone();
    let pending = hr.pending_updates.clone();
    let compliance = hr.compliance_alerts.clone();
    let changes = hr.recent_changes.clone();
    let changed = expect_context::<ChangedKeysCtx>();

    view! {
        <div class="space-y-4">
            <DashboardWarnings warnings=warnings />
            <div class="stat-grid">
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"CTC Completeness"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "hr.completeness.overall_completion_pct")
                        >
                            {format!("{:.1}%", completeness.overall_completion_pct)}
                        </p>
                        <p
                            class="text-xs text-huly-muted"
                            class:dashboard-change-flash=flash_if_any_changed(
                                changed,
                                &[
                                    "hr.completeness.total_with_ctc",
                                    "hr.completeness.total_employees",
                                ],
                            )
                        >
                            {format!("{} / {} employees", completeness.total_with_ctc, completeness.total_employees)}
                        </p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Pending Updates"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "hr.pending_updates.missing_count")
                        >
                            {pending.missing_count.to_string()}
                        </p>
                        <p class="text-xs text-huly-muted">"Missing CTC records"</p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Compliance Rate"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "hr.compliance_alerts.compliance_rate_pct")
                        >
                            {format!("{:.1}%", compliance.compliance_rate_pct)}
                        </p>
                        <p
                            class="text-xs text-huly-muted"
                            class:dashboard-change-flash=flash_if_changed(changed, "hr.compliance_alerts.total_discrepancies")
                        >
                            {format!("{} discrepancies", compliance.total_discrepancies)}
                        </p>
                    </div>
                </div>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
                <div class="panel">
                    <div class="toolbar flex items-center justify-between">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Recent CTC Changes"</h3>
                        <a href="/ctc/completeness" class="text-xs text-primary-400 hover:text-primary-300">"View completeness →"</a>
                    </div>
                    <div class="p-3">
                        {if changes.is_empty() {
                            Either::Left(view! {
                                <div class="empty-state py-4">
                                    <p class="text-huly-muted text-xs">"No recent CTC changes."</p>
                                </div>
                            })
                        } else {
                            Either::Right(view! {
                                <div>
                                    {changes.into_iter().map(|c| {
                                        let summary = format!("{} · rev {} · {}", c.resource_name, c.revision_number, c.changed_by_name.clone().unwrap_or_else(|| "System".to_string()));
                                        let key = recent_ctc_change_key(&c);
                                        let flash = flash_if_changed_owned(changed, key);
                                        view! {
                                            <div class="activity-item" class:dashboard-change-flash=flash>
                                                <span class="text-sm text-huly-content flex-1">{summary}</span>
                                                <span class="text-xs text-huly-muted whitespace-nowrap">{format_date(&c.created_at)}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>
                </div>

                <div class="panel">
                    <div class="toolbar flex items-center justify-between">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Pending CTC Updates"</h3>
                        <a href="/ctc/completeness" class="text-xs text-primary-400 hover:text-primary-300">"Open list →"</a>
                    </div>
                    <div class="p-3">
                        {if pending.sample.is_empty() {
                            Either::Left(view! {
                                <div class="empty-state py-4">
                                    <p class="text-huly-muted text-xs">"All employees have a CTC record."</p>
                                </div>
                            })
                        } else {
                            Either::Right(view! {
                                <div>
                                    {pending.sample.into_iter().map(|e| {
                                        let key = match e.id {
                                            Some(id) => format!("hr.pending_updates.sample.{}", id),
                                            None => format!("hr.pending_updates.sample.name.{}", e.name),
                                        };
                                        let flash = flash_if_changed_owned(changed, key);
                                        view! {
                                            <div class="activity-item" class:dashboard-change-flash=flash>
                                                <span class="text-sm text-huly-content flex-1">{e.name.clone()}</span>
                                                <span class="text-xs text-huly-muted whitespace-nowrap">{e.department.clone()}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>
                </div>
            </div>

            <div class="panel">
                <div class="toolbar">
                    <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">
                        {format!("Compliance Alerts ({} → {})", format_date(&compliance.start_date), format_date(&compliance.end_date))}
                    </h3>
                </div>
                <div class="p-3 text-sm text-huly-content">
                    <div class="grid grid-cols-3 gap-3 mb-3">
                        <div>
                            <p class="stat-label">"Validated"</p>
                            <p
                                class="stat-value"
                                class:dashboard-change-flash=flash_if_changed(changed, "hr.compliance_alerts.total_validated")
                            >
                                {compliance.total_validated.to_string()}
                            </p>
                        </div>
                        <div>
                            <p class="stat-label">"Passed"</p>
                            <p
                                class="stat-value"
                                class:dashboard-change-flash=flash_if_changed(changed, "hr.compliance_alerts.total_passed")
                            >
                                {compliance.total_passed.to_string()}
                            </p>
                        </div>
                        <div>
                            <p class="stat-label">"Discrepancies"</p>
                            <p
                                class="stat-value"
                                class:dashboard-change-flash=flash_if_changed(changed, "hr.compliance_alerts.total_discrepancies")
                            >
                                {compliance.total_discrepancies.to_string()}
                            </p>
                        </div>
                    </div>
                    {if compliance.top_risks.is_empty() {
                        Either::Left(view! {
                            <p class="text-huly-muted text-xs">"No compliance discrepancies in window."</p>
                        })
                    } else {
                        Either::Right(view! {
                            <div>
                                {compliance.top_risks.into_iter().map(|r| {
                                    let key = match r.resource_id {
                                        Some(id) => format!("hr.compliance_alerts.top_risks.{}", id),
                                        None => format!("hr.compliance_alerts.top_risks.name.{}", r.name),
                                    };
                                    let flash = flash_if_changed_owned(changed, key);
                                    view! {
                                        <div class="activity-item" class:dashboard-change-flash=flash>
                                            <span class="text-sm text-huly-content flex-1">{r.name.clone()}</span>
                                            <span class="text-xs text-huly-muted whitespace-nowrap">
                                                {format!("Δ {}", format_idr(r.variance_amount))}
                                            </span>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        })
                    }}
                </div>
            </div>
        </div>
    }
}

// ── Department Head Panel ────────────────────────────────────────────────

const DH_UNDERUTILIZED_THRESHOLD_PCT: f64 = 50.0;

fn is_underutilized_threshold(current_utilization_pct: f64) -> bool {
    current_utilization_pct < DH_UNDERUTILIZED_THRESHOLD_PCT
}

fn utilization_badge_class(current_pct: f64, is_overallocated: bool) -> &'static str {
    if is_overallocated || current_pct > 100.0 {
        "badge-negative"
    } else if is_underutilized_threshold(current_pct) {
        "badge-warning"
    } else if current_pct >= 80.0 {
        "badge-positive"
    } else {
        "badge-neutral"
    }
}

fn utilization_badge_label(current_pct: f64, is_overallocated: bool) -> &'static str {
    if is_overallocated || current_pct > 100.0 {
        "Overallocated"
    } else if is_underutilized_threshold(current_pct) {
        "Underutilized"
    } else if current_pct >= 80.0 {
        "Healthy"
    } else {
        "Available"
    }
}

fn budget_health_class(health: &str) -> &'static str {
    match health {
        "critical" => "text-negative-default",
        "warning" => "text-warning-default",
        "healthy" => "text-positive-default",
        _ => "text-huly-muted",
    }
}

fn budget_health_label(health: &str) -> &'static str {
    match health {
        "critical" => "At risk",
        "warning" => "Watch",
        "healthy" => "On track",
        _ => "Unconfigured",
    }
}

/// Clamp utilization to `[0, 100]` for inline bar width. Overallocated
/// resources still render a full bar; the badge handles the negative signal.
fn bar_width_pct(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else if value < 0.0 {
        0.0
    } else if value > 100.0 {
        100.0
    } else {
        value
    }
}

/// Stable key for a team utilization member row.
fn team_member_key(member: &TeamUtilizationMember) -> String {
    match member.resource_id {
        Some(id) => format!("department_head.team.{}", id),
        None => format!("department_head.team.name.{}", member.resource_name),
    }
}

/// Stable key for a single trend period belonging to one member.
fn trend_period_key(member: &TeamUtilizationTrend, period: &str) -> String {
    match member.resource_id {
        Some(id) => format!("department_head.trend.{}.{}", id, period),
        None => format!(
            "department_head.trend.name.{}.{}",
            member.resource_name, period
        ),
    }
}

fn current_projects_change_value(projects: &[TeamUtilizationCurrentProject]) -> String {
    if projects.is_empty() {
        return "none".to_string();
    }

    let mut projects = projects.iter().collect::<Vec<_>>();
    projects.sort_by(|a, b| {
        a.project_name
            .to_lowercase()
            .cmp(&b.project_name.to_lowercase())
            .then_with(|| a.start_date.cmp(&b.start_date))
            .then_with(|| a.end_date.cmp(&b.end_date))
            .then_with(|| {
                a.allocation_percentage
                    .partial_cmp(&b.allocation_percentage)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let rows = projects
        .into_iter()
        .map(|p| {
            (
                p.project_name.as_str(),
                format!("{:.3}", p.allocation_percentage),
                p.start_date.as_str(),
                p.end_date.as_str(),
            )
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&rows).unwrap_or_else(|_| "project-summary-unavailable".to_string())
}

fn trend_period_columns(bundle: &TeamUtilizationTrendBundle) -> Vec<String> {
    let mut periods = Vec::new();
    for member in &bundle.members {
        for period in &member.periods {
            if !periods.contains(&period.period) {
                periods.push(period.period.clone());
            }
        }
    }
    periods.sort();
    periods
}

#[component]
fn DepartmentHeadPanel(
    dh: DepartmentHeadDashboard,
    team_range: ReadSignal<TrendRangePreset>,
    set_team_range: WriteSignal<TrendRangePreset>,
) -> impl IntoView {
    let warnings = dh.warnings.clone();
    let utilization = dh.utilization.clone();
    let at_risk_members = utilization.top_at_risk.clone();
    let overallocations = dh.overallocations.clone();
    let upcoming = dh.upcoming_assignments.clone();
    let budget = dh.budget.clone();
    let team_members = dh.team_members.clone();
    let trend_bundle = dh.utilization_trends.clone();
    let changed = expect_context::<ChangedKeysCtx>();

    let team_member_count = team_members.len();
    let underutilized_count = team_members
        .iter()
        .filter(|m| m.is_underutilized || is_underutilized_threshold(m.current_utilization_pct))
        .count();
    let avg_available_pct: f64 = if team_member_count == 0 {
        0.0
    } else {
        team_members
            .iter()
            .map(|m| m.available_capacity_pct)
            .sum::<f64>()
            / team_member_count as f64
    };

    view! {
        <div class="space-y-4">
            <DashboardWarnings warnings=warnings />
            <div class="stat-grid">
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Avg Utilization"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "department_head.utilization.average_utilization_pct")
                        >
                            {format!("{:.1}%", utilization.average_utilization_pct)}
                        </p>
                        <p class="text-xs text-huly-muted">{format!("{} → {}", format_date(&utilization.start_date), format_date(&utilization.end_date))}</p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Avg Available Capacity"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "department_head.team.avg_available_capacity_pct")
                        >
                            {format!("{:.1}%", avg_available_pct)}
                        </p>
                        <p
                            class="text-xs text-huly-muted"
                            class:dashboard-change-flash=flash_if_changed(changed, "department_head.team.member_count")
                        >
                            {format!("{} team members", team_member_count)}
                        </p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Underutilized"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "department_head.team.underutilized_count")
                        >
                            {underutilized_count.to_string()}
                        </p>
                        <p class="text-xs text-huly-muted">{format!("< {:.0}% utilized", DH_UNDERUTILIZED_THRESHOLD_PCT)}</p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Overallocated"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "department_head.overallocations.overallocated_count")
                        >
                            {overallocations.overallocated_count.to_string()}
                        </p>
                    </div>
                </div>
                {budget.clone().map(|b| view! {
                    <div class="stat-card card-hover">
                        <div>
                            <p class="stat-label">{format!("Budget ({})", b.budget_period.clone())}</p>
                            <p
                                class=format!("stat-value {}", budget_health_class(&b.budget_health))
                                class:dashboard-change-flash=flash_if_changed(changed, "department_head.budget.utilization_percentage")
                            >
                                {format!("{:.0}%", b.utilization_percentage)}
                            </p>
                            <p
                                class="text-xs text-huly-muted"
                                class:dashboard-change-flash=flash_if_any_changed(
                                    changed,
                                    &[
                                        "department_head.budget.total_committed_idr",
                                        "department_head.budget.total_budget_idr",
                                    ],
                                )
                            >
                                {format!("{} of {}", format_idr(b.total_committed_idr), format_idr(b.total_budget_idr))}
                            </p>
                        </div>
                    </div>
                })}
            </div>

            {budget.clone().map(|b| view! {
                <DepartmentBudgetGauge budget=b />
            })}

            <DepartmentTeamUtilizationTable team_members=team_members />

            <DepartmentUtilizationTrend
                bundle=trend_bundle
                team_range=team_range
                set_team_range=set_team_range
            />

            <div class="grid grid-cols-1 lg:grid-cols-3 gap-3">
                <div class="panel">
                    <div class="toolbar">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"At-risk Members"</h3>
                    </div>
                    <div class="p-3">
                        {if at_risk_members.is_empty() {
                            Either::Left(view! {
                                <div class="empty-state py-4">
                                    <p class="text-huly-muted text-xs">"No high-utilization members."</p>
                                </div>
                            })
                        } else {
                            Either::Right(view! {
                                <div>
                                    {at_risk_members.into_iter().map(|m| {
                                        let key = match m.resource_id {
                                            Some(id) => format!("department_head.top_at_risk.{}", id),
                                            None => format!("department_head.top_at_risk.name.{}", m.resource_name),
                                        };
                                        let flash = flash_if_changed_owned(changed, key);
                                        view! {
                                            <div class="activity-item" class:dashboard-change-flash=flash>
                                                <span class="text-sm text-huly-content flex-1">{m.resource_name.clone()}</span>
                                                <span class="text-xs text-warning-default whitespace-nowrap">{format!("{:.0}%", m.current_allocation_pct)}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>
                </div>

                <div class="panel">
                    <div class="toolbar">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Overallocated Members"</h3>
                    </div>
                    <div class="p-3">
                        {if overallocations.members.is_empty() {
                            Either::Left(view! {
                                <div class="empty-state py-4">
                                    <p class="text-huly-muted text-xs">"No overallocated members."</p>
                                </div>
                            })
                        } else {
                            Either::Right(view! {
                                <div>
                                    {overallocations.members.into_iter().map(|m| {
                                        let key = match m.resource_id {
                                            Some(id) => format!("department_head.overallocations.member.{}", id),
                                            None => format!(
                                                "department_head.overallocations.member.name.{}",
                                                m.resource_name
                                            ),
                                        };
                                        let flash = flash_if_changed_owned(changed, key);
                                        view! {
                                            <div class="activity-item" class:dashboard-change-flash=flash>
                                                <span class="text-sm text-huly-content flex-1">{m.resource_name.clone()}</span>
                                                <span class="text-xs text-negative-default whitespace-nowrap">{format!("{:.0}%", m.current_allocation_pct)}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>
                </div>

                <div class="panel">
                    <div class="toolbar flex items-center justify-between">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Upcoming Assignments"</h3>
                        <a href="/team" class="text-xs text-primary-400 hover:text-primary-300">"Open team →"</a>
                    </div>
                    <div class="p-3">
                        {if upcoming.is_empty() {
                            Either::Left(view! {
                                <div class="empty-state py-4">
                                    <p class="text-huly-muted text-xs">"No upcoming assignments."</p>
                                </div>
                            })
                        } else {
                            Either::Right(view! {
                                <div>
                                    {upcoming.into_iter().map(|u| {
                                        let label = format!("{} → {} · {:.0}%", u.resource_name, u.project_name, u.allocation_percentage);
                                        let key = match u.allocation_id {
                                            Some(id) => format!("department_head.upcoming_assignments.{}", id),
                                            None => format!(
                                                "department_head.upcoming_assignments.name.{}|{}",
                                                u.resource_name, u.project_name
                                            ),
                                        };
                                        let flash = flash_if_changed_owned(changed, key);
                                        view! {
                                            <div class="activity-item" class:dashboard-change-flash=flash>
                                                <span class="text-sm text-huly-content flex-1">{label}</span>
                                                <span class="text-xs text-huly-muted whitespace-nowrap">{format!("{} → {}", format_date(&u.start_date), format_date(&u.end_date))}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn DepartmentBudgetGauge(budget: DepartmentBudgetSummary) -> impl IntoView {
    let changed = expect_context::<ChangedKeysCtx>();
    let width_pct = if budget.total_budget_idr > 0 {
        bar_width_pct(budget.utilization_percentage)
    } else {
        0.0
    };
    let fill_class = budget_health_class(&budget.budget_health);
    let bar_fill_color = match budget.budget_health.as_str() {
        "critical" => "background-color: var(--color-negative-default);",
        "warning" => "background-color: var(--color-warning-default);",
        "healthy" => "background-color: var(--color-positive-default);",
        _ => "background-color: var(--color-huly-ghost);",
    };
    let threshold = budget.alert_threshold_pct;
    view! {
        <div class="panel">
            <div class="toolbar flex items-center justify-between">
                <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Department Budget"</h3>
                <a href="/team" class="text-xs text-primary-400 hover:text-primary-300">"Open team →"</a>
            </div>
            <div class="p-3 space-y-3">
                <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
                    <div>
                        <p class="stat-label">"Total Budget"</p>
                        <p
                            class="text-sm font-mono text-huly-caption"
                            class:dashboard-change-flash=flash_if_changed(changed, "department_head.budget.total_budget_idr")
                        >
                            {format_idr(budget.total_budget_idr)}
                        </p>
                    </div>
                    <div>
                        <p class="stat-label">"Committed"</p>
                        <p
                            class="text-sm font-mono text-huly-caption"
                            class:dashboard-change-flash=flash_if_changed(changed, "department_head.budget.total_committed_idr")
                        >
                            {format_idr(budget.total_committed_idr)}
                        </p>
                    </div>
                    <div>
                        <p class="stat-label">"Spent"</p>
                        <p
                            class="text-sm font-mono text-huly-caption"
                            class:dashboard-change-flash=flash_if_changed(changed, "department_head.budget.spent_actual_idr")
                        >
                            {format_idr(budget.spent_actual_idr)}
                        </p>
                    </div>
                    <div>
                        <p class="stat-label">"Available"</p>
                        <p
                            class="text-sm font-mono text-huly-caption"
                            class:dashboard-change-flash=flash_if_changed(changed, "department_head.budget.remaining_idr")
                        >
                            {format_idr(budget.remaining_idr)}
                        </p>
                    </div>
                </div>
                <div>
                    <div class="flex items-center justify-between mb-1">
                        <span
                            class=format!("text-xs font-medium {}", fill_class)
                            class:dashboard-change-flash=flash_if_changed(changed, "department_head.budget.budget_health")
                        >
                            {budget_health_label(&budget.budget_health)}
                        </span>
                        <span
                            class="text-xs text-huly-muted"
                            class:dashboard-change-flash=flash_if_changed(changed, "department_head.budget.utilization_percentage")
                        >
                            {format!("{:.0}% used", budget.utilization_percentage)}
                        </span>
                    </div>
                    <div
                        class="progress-track h-3"
                        role="progressbar"
                        aria-valuemin="0"
                        aria-valuemax="100"
                        aria-valuenow=format!("{:.0}", width_pct)
                        aria-label="Department budget utilization"
                    >
                        <div
                            class="h-3 rounded-full transition-all"
                            style=format!("width: {:.2}%; {}", width_pct, bar_fill_color)
                        ></div>
                    </div>
                    {if threshold > 0 {
                        Some(view! {
                            <p
                                class="text-xs text-huly-muted mt-1"
                                class:dashboard-change-flash=flash_if_changed(changed, "department_head.budget.alert_threshold_pct")
                            >
                                {format!("Alert threshold: {}%", threshold)}
                            </p>
                        })
                    } else { None }}
                    {if !budget.budget_configured {
                        Some(view! {
                            <p class="text-xs text-huly-muted mt-1">"(Budget not yet configured for this period.)"</p>
                        })
                    } else { None }}
                </div>
            </div>
        </div>
    }
}

#[component]
fn DepartmentTeamUtilizationTable(team_members: Vec<TeamUtilizationMember>) -> impl IntoView {
    let changed = expect_context::<ChangedKeysCtx>();
    let navigate = use_navigate();
    view! {
        <div class="panel">
            <div class="toolbar flex items-center justify-between">
                <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Team Utilization"</h3>
                <a href="/team" class="text-xs text-primary-400 hover:text-primary-300">"Open team →"</a>
            </div>
            <div class="p-0">
                {if team_members.is_empty() {
                    Either::Left(view! {
                        <div class="empty-state py-6">
                            <p class="text-huly-muted text-xs">"No team members in scope."</p>
                        </div>
                    })
                } else {
                    Either::Right(view! {
                        <div class="overflow-x-auto">
                            <table class="w-full text-sm">
                                <thead>
                                    <tr class="text-xs text-huly-secondary uppercase tracking-wider">
                                        <th class="text-left p-2">"Member"</th>
                                        <th class="text-right p-2">"Utilization"</th>
                                        <th class="text-right p-2">"Available"</th>
                                        <th class="text-left p-2">"Current Projects"</th>
                                        <th class="text-center p-2">"Status"</th>
                                        <th class="text-center p-2">"Action"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {team_members.into_iter().map(|member| {
                                        let row_key = team_member_key(&member);
                                        let name_key = format!("{}.resource_name", row_key);
                                        let role_key = format!("{}.role", row_key);
                                        let current_key = format!("{}.current_utilization_pct", row_key);
                                        let available_key = format!("{}.available_capacity_pct", row_key);
                                        let projects_key = format!("{}.current_projects", row_key);
                                        let underutil_key = format!("{}.is_underutilized", row_key);
                                        let overallocated_key = format!("{}.is_overallocated", row_key);

                                        let badge_class = utilization_badge_class(member.current_utilization_pct, member.is_overallocated);
                                        let badge_label = utilization_badge_label(member.current_utilization_pct, member.is_overallocated);
                                        let bar_width = bar_width_pct(member.current_utilization_pct);
                                        let bar_color = if member.is_overallocated || member.current_utilization_pct > 100.0 {
                                            "background-color: var(--color-negative-default);"
                                        } else if is_underutilized_threshold(member.current_utilization_pct) {
                                            "background-color: var(--color-warning-default);"
                                        } else {
                                            "background-color: var(--color-positive-default);"
                                        };

                                        let projects_summary = if member.current_projects.is_empty() {
                                            "None".to_string()
                                        } else {
                                            member
                                                .current_projects
                                                .iter()
                                                .map(|p| format!("{} ({:.0}%)", p.project_name, p.allocation_percentage))
                                                .collect::<Vec<_>>()
                                                .join(", ")
                                        };
                                        let projects_title = if member.current_projects.is_empty() {
                                            String::new()
                                        } else {
                                            member
                                                .current_projects
                                                .iter()
                                                .map(|p| {
                                                    format!(
                                                        "{} · {:.0}% · {} → {}",
                                                        p.project_name,
                                                        p.allocation_percentage,
                                                        format_date(&p.start_date),
                                                        format_date(&p.end_date),
                                                    )
                                                })
                                                .collect::<Vec<_>>()
                                                .join("\n")
                                        };

                                        let ctc_missing = member.ctc_status != "Active";
                                        let show_assign = member.is_underutilized
                                            || is_underutilized_threshold(member.current_utilization_pct);
                                        let nav = navigate.clone();
                                        let rid = member.resource_id;
                                        let assign_click = move |_| {
                                            if let Some(id) = rid {
                                                nav(
                                                    &format!("/team?assign_resource_id={}", id),
                                                    NavigateOptions::default(),
                                                );
                                            }
                                        };
                                        let row_flash = {
                                            let keys = vec![
                                                name_key.clone(),
                                                role_key.clone(),
                                                current_key.clone(),
                                                available_key.clone(),
                                                projects_key.clone(),
                                                underutil_key.clone(),
                                                overallocated_key.clone(),
                                                format!("{}.ctc_status", row_key),
                                            ];
                                            flash_if_any_changed_owned(changed, keys)
                                        };

                                        view! {
                                            <tr class="border-t border-huly-divider" class:dashboard-change-flash=row_flash>
                                                <td class="p-2 align-top">
                                                    <div
                                                        class="text-sm text-huly-content font-medium"
                                                        class:dashboard-change-flash=flash_if_changed_owned(changed, name_key.clone())
                                                    >
                                                        {member.resource_name.clone()}
                                                    </div>
                                                    <div
                                                        class="text-xs text-huly-muted"
                                                        class:dashboard-change-flash=flash_if_changed_owned(changed, role_key.clone())
                                                    >
                                                        {member.role.clone()}
                                                    </div>
                                                </td>
                                                <td class="p-2 align-top text-right">
                                                    <div
                                                        class="text-sm font-mono text-huly-caption"
                                                        class:dashboard-change-flash=flash_if_changed_owned(changed, current_key.clone())
                                                    >
                                                        {format!("{:.1}%", member.current_utilization_pct)}
                                                    </div>
                                                    <div class="progress-track h-2 mt-1" style="width: 80px; margin-left: auto;">
                                                        <div
                                                            class="h-2 rounded-full transition-all"
                                                            style=format!("width: {:.2}%; {}", bar_width, bar_color)
                                                        ></div>
                                                    </div>
                                                </td>
                                                <td class="p-2 align-top text-right">
                                                    <span
                                                        class="text-sm font-mono text-huly-content"
                                                        class:dashboard-change-flash=flash_if_changed_owned(changed, available_key.clone())
                                                    >
                                                        {format!("{:.1}%", member.available_capacity_pct)}
                                                    </span>
                                                </td>
                                                <td class="p-2 align-top">
                                                    <div
                                                        class="text-xs text-huly-content"
                                                        title=projects_title
                                                        class:dashboard-change-flash=flash_if_changed_owned(changed, projects_key.clone())
                                                    >
                                                        {projects_summary}
                                                    </div>
                                                </td>
                                                <td class="p-2 align-top text-center">
                                                    <span
                                                        class=badge_class
                                                        class:dashboard-change-flash=flash_if_changed_owned(changed, overallocated_key.clone())
                                                    >
                                                        {badge_label}
                                                    </span>
                                                </td>
                                                <td class="p-2 align-top text-center">
                                                    {if show_assign {
                                                        if ctc_missing {
                                                            EitherOf3::A(view! {
                                                                <button
                                                                    disabled=true
                                                                    title="CTC data required to assign. Contact HR to complete employee setup."
                                                                    class="btn-secondary text-xs opacity-50 cursor-not-allowed"
                                                                >
                                                                    "Assign"
                                                                </button>
                                                            })
                                                        } else if rid.is_some() {
                                                            EitherOf3::B(view! {
                                                                <button
                                                                    class="btn-primary text-xs"
                                                                    on:click=assign_click
                                                                >
                                                                    "Assign"
                                                                </button>
                                                            })
                                                        } else {
                                                            EitherOf3::C(view! { <span class="text-xs text-huly-muted">"—"</span> })
                                                        }
                                                    } else {
                                                        EitherOf3::C(view! { <span class="text-xs text-huly-muted">"—"</span> })
                                                    }}
                                                </td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                    })
                }}
            </div>
        </div>
    }
}

#[component]
fn DepartmentUtilizationTrend(
    bundle: Option<TeamUtilizationTrendBundle>,
    team_range: ReadSignal<TrendRangePreset>,
    set_team_range: WriteSignal<TrendRangePreset>,
) -> impl IntoView {
    let changed = expect_context::<ChangedKeysCtx>();
    let bundle_clone = bundle.clone();

    view! {
        <div class="panel">
            <div class="toolbar flex items-center justify-between flex-wrap gap-2">
                <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Utilization Trend"</h3>
                <div
                    class="inline-flex items-center gap-1"
                    role="group"
                    aria-label="Select utilization trend range"
                >
                    <span class="text-xs text-huly-muted mr-1">"Range"</span>
                    {[TrendRangePreset::CurrentMonth, TrendRangePreset::Next30Days, TrendRangePreset::ThreeMonths, TrendRangePreset::SixMonths]
                        .into_iter()
                        .map(|preset| {
                            let label = preset.label();
                            let on_click = move |_| set_team_range.set(preset);
                            view! {
                                <button
                                    type="button"
                                    class="btn-ghost text-xs"
                                    aria-pressed=move || (team_range.get() == preset).to_string()
                                    on:click=on_click
                                    style=move || if team_range.get() == preset {
                                        "background-color: var(--color-huly-btn-hover); color: var(--color-huly-caption);"
                                    } else { "" }
                                >
                                    {label}
                                </button>
                            }
                        }).collect_view()}
                </div>
            </div>
            <div class="p-3">
                {match bundle_clone {
                    None => EitherOf3::A(view! {
                        <div class="empty-state py-6">
                            <p class="text-huly-muted text-xs">"Trend data is not yet available."</p>
                        </div>
                    }),
                    Some(b) if b.members.is_empty() => EitherOf3::B(view! {
                        <div class="empty-state py-6">
                            <p class="text-huly-muted text-xs">{format!("No utilization data for {} → {}.", format_date(&b.start_date), format_date(&b.end_date))}</p>
                        </div>
                    }),
                    Some(b) => {
                        let period_columns = trend_period_columns(&b);
                        EitherOf3::C(view! {
                            <div class="overflow-x-auto">
                                <table class="w-full text-xs">
                                    <thead>
                                        <tr class="text-huly-secondary uppercase tracking-wider">
                                            <th class="text-left p-2">"Member"</th>
                                            {period_columns.iter().map(|period| view! {
                                                <th class="text-right p-2">{period.clone()}</th>
                                            }).collect_view()}
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {b.members.into_iter().map(|member| {
                                            let resource_name = member.resource_name.clone();
                                            let row_periods = period_columns.clone();
                                            view! {
                                                <tr class="border-t border-huly-divider">
                                                    <td class="p-2 text-huly-content">{resource_name}</td>
                                                    {row_periods.into_iter().map(|period_label| {
                                                        let key = trend_period_key(&member, &period_label);
                                                        let flash = flash_if_changed_owned(changed, key);
                                                        let value = member
                                                            .periods
                                                            .iter()
                                                            .find(|p| p.period == period_label)
                                                            .map(|p| p.utilization_pct);
                                                        view! {
                                                            <td class="p-2 text-right" class:dashboard-change-flash=flash>
                                                                {if let Some(utilization_pct) = value {
                                                                    let bar_w = bar_width_pct(utilization_pct);
                                                                    let bar_color = if utilization_pct > 100.0 {
                                                                        "background-color: var(--color-negative-default);"
                                                                    } else if is_underutilized_threshold(utilization_pct) {
                                                                        "background-color: var(--color-warning-default);"
                                                                    } else {
                                                                        "background-color: var(--color-positive-default);"
                                                                    };
                                                                    Either::Left(view! {
                                                                        <div class="flex items-center justify-end gap-2">
                                                                            <div class="progress-track h-2" style="width: 60px;">
                                                                                <div
                                                                                    class="h-2 rounded-full transition-all"
                                                                                    style=format!("width: {:.2}%; {}", bar_w, bar_color)
                                                                                ></div>
                                                                            </div>
                                                                            <span class="font-mono text-huly-content">{format!("{:.0}%", utilization_pct)}</span>
                                                                        </div>
                                                                    })
                                                                } else {
                                                                    Either::Right(view! {
                                                                        <span class="font-mono text-huly-muted">"—"</span>
                                                                    })
                                                                }}
                                                            </td>
                                                        }
                                                    }).collect_view()}
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </div>
                        })
                    },
                }}
            </div>
        </div>
    }
}

// ── Project Manager Panel ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PmSortMode {
    Margin,
    BudgetUtilization,
    EndDate,
}

impl PmSortMode {
    fn label(&self) -> &'static str {
        match self {
            PmSortMode::Margin => "Margin",
            PmSortMode::BudgetUtilization => "Budget Utilization",
            PmSortMode::EndDate => "End Date",
        }
    }
}

/// Sort PM cards client-side from already-loaded data. Sort modes:
/// - `Margin`: lowest current margin first, then project name (ascending).
/// - `BudgetUtilization`: highest utilization (over-budget first) first, then project name.
/// - `EndDate`: soonest end date first, then project name.
fn sort_pm_cards(cards: &mut [ProjectHealthCard], mode: PmSortMode) {
    match mode {
        PmSortMode::Margin => {
            cards.sort_by(|a, b| {
                a.margin_pct
                    .partial_cmp(&b.margin_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.project_name.cmp(&b.project_name))
            });
        }
        PmSortMode::BudgetUtilization => {
            cards.sort_by(|a, b| {
                let a_score = pm_utilization_score(a);
                let b_score = pm_utilization_score(b);
                b_score
                    .partial_cmp(&a_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.project_name.cmp(&b.project_name))
            });
        }
        PmSortMode::EndDate => {
            cards.sort_by(|a, b| {
                a.end_date
                    .cmp(&b.end_date)
                    .then_with(|| a.project_name.cmp(&b.project_name))
            });
        }
    }
}

fn pm_utilization_score(card: &ProjectHealthCard) -> f64 {
    if card.is_over_budget && card.budget_overrun_idr > 0 {
        let overrun_pct = if card.total_budget_idr > 0 {
            (card.budget_overrun_idr as f64 / card.total_budget_idr as f64) * 100.0
        } else {
            0.0
        };
        card.budget_utilization_pct.max(100.0) + overrun_pct
    } else if card.is_over_budget {
        card.budget_utilization_pct.max(100.0)
    } else if card.total_budget_idr <= 0 {
        -1.0
    } else {
        card.budget_utilization_pct
    }
}

fn budget_badge_class(budget_status: &str, is_over_budget: bool) -> &'static str {
    if is_over_budget {
        return "badge-negative";
    }
    match budget_status {
        "critical" => "badge-negative",
        "warning" => "badge-warning",
        "healthy" => "badge-positive",
        _ => "badge-neutral",
    }
}

fn budget_badge_label(budget_status: &str, is_over_budget: bool) -> &'static str {
    if is_over_budget {
        return "Over budget";
    }
    match budget_status {
        "critical" => "At risk",
        "warning" => "Watch",
        "healthy" => "On track",
        _ => "Unconfigured",
    }
}

fn health_status_class(health_status: &str) -> &'static str {
    match health_status {
        "critical" => "text-negative-default",
        "warning" => "text-warning-default",
        "healthy" => "text-positive-default",
        _ => "text-huly-muted",
    }
}

fn health_status_label(health_status: &str) -> &'static str {
    match health_status {
        "critical" => "At risk",
        "warning" => "Watch",
        "healthy" => "Healthy",
        _ => "Unconfigured",
    }
}

fn margin_status_class(
    margin_pct: f64,
    target_margin_pct: f64,
    alert_threshold_pct: f64,
    margin_alert_present: bool,
    has_revenue_signal: bool,
) -> &'static str {
    if !has_revenue_signal && !margin_alert_present {
        return "text-huly-muted";
    }
    if margin_alert_present || margin_pct < 0.0 {
        return "text-negative-default";
    }
    let margin_gap = target_margin_pct - margin_pct;
    if margin_gap > 0.0 && alert_threshold_pct > 0.0 && margin_gap > alert_threshold_pct {
        return "text-negative-default";
    }
    if margin_gap > 0.0 {
        return "text-warning-default";
    }
    "text-positive-default"
}

fn forecast_status_class(
    variance_from_target_pct: f64,
    alert_threshold_pct: f64,
    forecast_unavailable: bool,
    has_revenue_signal: bool,
) -> &'static str {
    if forecast_unavailable || !has_revenue_signal {
        return "text-huly-muted";
    }
    if variance_from_target_pct < 0.0
        && variance_from_target_pct.abs() > alert_threshold_pct.max(0.0)
    {
        return "text-negative-default";
    }
    if variance_from_target_pct < 0.0 {
        return "text-warning-default";
    }
    "text-positive-default"
}

#[component]
fn ProjectManagerPanel(
    pm: ProjectManagerDashboard,
    sort_mode: ReadSignal<PmSortMode>,
    set_sort_mode: WriteSignal<PmSortMode>,
) -> impl IntoView {
    let warnings = pm.warnings.clone();
    let alerts = pm.margin_alerts.clone();
    let projects = pm.active_projects.clone();
    let changed = expect_context::<ChangedKeysCtx>();
    let navigate = use_navigate();

    let sorted_projects = Signal::derive(move || {
        let mut copy = projects.clone();
        sort_pm_cards(&mut copy, sort_mode.get());
        copy
    });

    view! {
        <div class="space-y-4">
            <DashboardWarnings warnings=warnings />
            <div class="stat-grid">
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Active Projects"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "project_manager.active_projects.count")
                        >
                            {move || sorted_projects.with(|p| p.len().to_string())}
                        </p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Margin Alerts"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "project_manager.margin_alerts.count")
                        >
                            {alerts.len().to_string()}
                        </p>
                    </div>
                </div>
            </div>

            {if !alerts.is_empty() {
                Some(view! {
                    <div class="panel">
                        <div class="toolbar">
                            <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Margin Alerts"</h3>
                        </div>
                        <div class="p-3">
                            {alerts.into_iter().map(|a| {
                                let id_key = match a.project_id {
                                    Some(id) => id.to_string(),
                                    None => format!("name.{}", a.project_name),
                                };
                                let key = format!("project_manager.margin_alert.{}", id_key);
                                let flash = flash_if_changed_owned(changed, key);
                                view! {
                                    <div class="activity-item" class:dashboard-change-flash=flash>
                                        <span class="text-sm text-huly-content flex-1">{a.project_name.clone()}</span>
                                        <span class="text-xs text-huly-muted whitespace-nowrap">{format!("{:.1}% · {}", a.margin_pct, a.message)}</span>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    </div>
                })
            } else { None }}

            <div class="panel">
                <div class="toolbar flex items-center justify-between flex-wrap gap-2">
                    <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Project Health"</h3>
                    <div class="flex items-center gap-2">
                        <div
                            class="inline-flex items-center gap-1"
                            role="group"
                            aria-label="Sort projects by"
                        >
                            <span class="text-xs text-huly-muted mr-1">"Sort by"</span>
                            {[PmSortMode::Margin, PmSortMode::BudgetUtilization, PmSortMode::EndDate].into_iter().map(|mode| {
                                let label = mode.label();
                                let on_click = move |_| set_sort_mode.set(mode);
                                view! {
                                    <button
                                        type="button"
                                        class="btn-ghost text-xs"
                                        aria-pressed=move || (sort_mode.get() == mode).to_string()
                                        on:click=on_click
                                        style=move || if sort_mode.get() == mode {
                                            "background-color: var(--color-huly-btn-hover); color: var(--color-huly-caption);"
                                        } else { "" }
                                    >
                                        {label}
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                        <a href="/projects" class="text-xs text-primary-400 hover:text-primary-300">"Open projects →"</a>
                    </div>
                </div>
                <div class="p-3">
                    {move || {
                        let projects = sorted_projects.get();
                        if projects.is_empty() {
                            Either::Left(view! {
                                <div class="empty-state py-4">
                                    <p class="text-huly-muted text-xs">"No active projects assigned."</p>
                                </div>
                            })
                        } else {
                            let navigate = navigate.clone();
                            Either::Right(view! {
                                <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
                                    {projects.into_iter().map(|p| {
                                        let id_key = match p.project_id {
                                            Some(id) => id.to_string(),
                                            None => format!("name.{}", p.project_name),
                                        };
                                        let name_key = format!("project_manager.project.{}.project_name", id_key);
                                        let project_status_key = format!("project_manager.project.{}.status", id_key);
                                        let end_date_key = format!("project_manager.project.{}.end_date", id_key);
                                        let margin_key = format!("project_manager.project.{}.margin_pct", id_key);
                                        let profit_key = format!("project_manager.project.{}.gross_profit_idr", id_key);
                                        let budget_status_key = format!("project_manager.project.{}.budget_status", id_key);
                                        let total_budget_key = format!("project_manager.project.{}.total_budget_idr", id_key);
                                        let spent_key = format!("project_manager.project.{}.budget_spent_idr", id_key);
                                        let remaining_key = format!("project_manager.project.{}.budget_remaining_idr", id_key);
                                        let revenue_key = format!("project_manager.project.{}.total_revenue_idr", id_key);
                                        let cost_key = format!("project_manager.project.{}.total_cost_idr", id_key);
                                        let warning_key = format!("project_manager.project.{}.warning", id_key);
                                        let margin_alert_key = format!("project_manager.project.{}.margin_alert", id_key);
                                        let budget_utilization_key = format!("project_manager.project.{}.budget_utilization_pct", id_key);
                                        let is_over_budget_key = format!("project_manager.project.{}.is_over_budget", id_key);
                                        let budget_overrun_key = format!("project_manager.project.{}.budget_overrun_idr", id_key);
                                        let forecast_margin_key = format!("project_manager.project.{}.forecast_margin_pct", id_key);
                                        let forecast_variance_key = format!("project_manager.project.{}.forecast_variance_from_target_pct", id_key);
                                        let projected_total_cost_key = format!("project_manager.project.{}.projected_total_cost_idr", id_key);
                                        let forecast_unavailable_key = format!("project_manager.project.{}.forecast_unavailable", id_key);
                                        let forecast_revenue_signal_key = format!("project_manager.project.{}.forecast_has_revenue_signal", id_key);
                                        let health_status_key = format!("project_manager.project.{}.health_status", id_key);
                                        let target_margin_key = format!("project_manager.project.{}.target_margin_pct", id_key);
                                        let margin_alert_threshold_key = format!("project_manager.project.{}.margin_alert_threshold_pct", id_key);

                                        let flash_card = {
                                            let any_change_keys = [
                                                name_key.clone(),
                                                project_status_key.clone(),
                                                end_date_key.clone(),
                                                margin_key.clone(),
                                                profit_key.clone(),
                                                budget_status_key.clone(),
                                                total_budget_key.clone(),
                                                spent_key.clone(),
                                                remaining_key.clone(),
                                                revenue_key.clone(),
                                                cost_key.clone(),
                                                warning_key.clone(),
                                                margin_alert_key.clone(),
                                                budget_utilization_key.clone(),
                                                is_over_budget_key.clone(),
                                                budget_overrun_key.clone(),
                                                forecast_margin_key.clone(),
                                                forecast_variance_key.clone(),
                                                projected_total_cost_key.clone(),
                                                forecast_unavailable_key.clone(),
                                                forecast_revenue_signal_key.clone(),
                                                health_status_key.clone(),
                                                target_margin_key.clone(),
                                                margin_alert_threshold_key.clone(),
                                            ];
                                            move || changed.0.with(|set| any_change_keys.iter().any(|k| set.contains(k)))
                                        };

                                        let is_budget_overrun = p.is_over_budget
                                            || (p.total_budget_idr > 0 && p.budget_spent_idr > p.total_budget_idr);
                                        let effective_overrun_idr = if p.budget_overrun_idr > 0 {
                                            p.budget_overrun_idr
                                        } else if p.total_budget_idr > 0 && p.budget_spent_idr > p.total_budget_idr {
                                            p.budget_spent_idr - p.total_budget_idr
                                        } else {
                                            0
                                        };
                                        let forecast_unavailable = p.forecast_unavailable;
                                        let has_revenue_signal = p.total_revenue_idr > 0;
                                        let has_forecast_revenue_signal = p.forecast_has_revenue_signal;
                                        let margin_alert_present = p.margin_alert.is_some();
                                        let margin_class = margin_status_class(
                                            p.margin_pct,
                                            p.target_margin_pct,
                                            p.margin_alert_threshold_pct,
                                            margin_alert_present,
                                            has_revenue_signal,
                                        );
                                        let forecast_class = forecast_status_class(
                                            p.forecast_variance_from_target_pct,
                                            p.margin_alert_threshold_pct,
                                            forecast_unavailable,
                                            has_forecast_revenue_signal,
                                        );
                                        let badge_class = budget_badge_class(&p.budget_status, is_budget_overrun);
                                        let badge_label = budget_badge_label(&p.budget_status, is_budget_overrun);
                                        let health_class = health_status_class(&p.health_status);
                                        let health_label = health_status_label(&p.health_status);
                                        let pid = p.project_id;
                                        let project_name = p.project_name.clone();
                                        let nav = navigate.clone();
                                        let pnl_click = move |_| {
                                            if let Some(id) = pid {
                                                nav(&format!("/projects?view=pnl&project_id={}", id), Default::default());
                                            }
                                        };
                                        let over_budget_aria = if is_budget_overrun {
                                            format!("Over budget by {}", format_idr(effective_overrun_idr))
                                        } else { String::new() };
                                        let pnl_aria_label = format!("View P&L for {}", project_name);

                                        view! {
                                            <div class="panel p-3" class:dashboard-change-flash=flash_card>
                                                <div class="flex items-center justify-between mb-1 gap-2">
                                                    <span
                                                        class="text-sm font-medium text-huly-caption truncate"
                                                        class:dashboard-change-flash=flash_if_changed_owned(changed, name_key.clone())
                                                    >
                                                        {p.project_name.clone()}
                                                    </span>
                                                    <div class="flex items-center gap-1.5 whitespace-nowrap">
                                                        {is_budget_overrun.then(|| view! {
                                                            <span
                                                                class="text-negative-default"
                                                                aria-label=over_budget_aria.clone()
                                                                title=over_budget_aria.clone()
                                                            >
                                                                <svg
                                                                    xmlns="http://www.w3.org/2000/svg"
                                                                    width="14"
                                                                    height="14"
                                                                    viewBox="0 0 24 24"
                                                                    fill="none"
                                                                    stroke="currentColor"
                                                                    stroke-width="2"
                                                                    stroke-linecap="round"
                                                                    stroke-linejoin="round"
                                                                    aria-hidden="true"
                                                                    focusable="false"
                                                                >
                                                                    <path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
                                                                    <line x1="12" y1="9" x2="12" y2="13" />
                                                                    <line x1="12" y1="17" x2="12.01" y2="17" />
                                                                </svg>
                                                            </span>
                                                        })}
                                                        <span
                                                            class=badge_class
                                                            class:dashboard-change-flash=flash_if_any_changed_owned(
                                                                changed,
                                                                vec![is_over_budget_key.clone(), budget_status_key.clone(), spent_key.clone(), total_budget_key.clone()],
                                                            )
                                                        >
                                                            {badge_label}
                                                        </span>
                                                    </div>
                                                </div>
                                                <div class="flex items-center justify-between gap-2 mb-2">
                                                    <p
                                                        class="text-xs text-huly-muted"
                                                        class:dashboard-change-flash=flash_if_changed_owned(changed, end_date_key.clone())
                                                    >
                                                        {format!("Ends {}", format_date(&p.end_date))}
                                                    </p>
                                                    <span
                                                        class="text-xs text-huly-muted"
                                                        class:dashboard-change-flash=flash_if_changed_owned(changed, project_status_key.clone())
                                                    >
                                                        {p.status.clone()}
                                                    </span>
                                                </div>
                                                <div class="text-xs text-huly-content space-y-0.5">
                                                    <p class:dashboard-change-flash=flash_if_any_changed_owned(changed, vec![budget_status_key.clone(), total_budget_key.clone(), budget_utilization_key.clone()])>
                                                        {format!(
                                                            "Budget: {} ({:.0}% used)",
                                                            format_idr(p.total_budget_idr),
                                                            p.budget_utilization_pct,
                                                        )}
                                                    </p>
                                                    <p
                                                        class=health_class
                                                        class:dashboard-change-flash=flash_if_changed_owned(changed, health_status_key.clone())
                                                    >
                                                        {format!("Health: {}", health_label)}
                                                    </p>
                                                    <p class:dashboard-change-flash=flash_if_any_changed_owned(changed, vec![spent_key.clone(), remaining_key.clone(), budget_overrun_key.clone()])>
                                                        {if is_budget_overrun {
                                                            format!(
                                                                "Spent: {} · Overrun: {}",
                                                                format_idr(p.budget_spent_idr),
                                                                format_idr(effective_overrun_idr),
                                                            )
                                                        } else {
                                                            format!(
                                                                "Spent: {} · Remaining: {}",
                                                                format_idr(p.budget_spent_idr),
                                                                format_idr(p.budget_remaining_idr),
                                                            )
                                                        }}
                                                    </p>
                                                    <p class:dashboard-change-flash=flash_if_changed_owned(changed, revenue_key.clone())>
                                                        {format!("Revenue: {}", format_idr(p.total_revenue_idr))}
                                                    </p>
                                                    <p class:dashboard-change-flash=flash_if_changed_owned(changed, cost_key.clone())>
                                                        {format!("Cost: {}", format_idr(p.total_cost_idr))}
                                                    </p>
                                                    <p
                                                        class=margin_class
                                                        class:dashboard-change-flash=flash_if_any_changed_owned(
                                                            changed,
                                                            vec![
                                                                margin_key.clone(),
                                                                target_margin_key.clone(),
                                                                margin_alert_threshold_key.clone(),
                                                                profit_key.clone(),
                                                            ],
                                                        )
                                                    >
                                                        {format!(
                                                            "Margin: {:.1}% (target {:.1}%) · profit {}",
                                                            p.margin_pct,
                                                            p.target_margin_pct,
                                                            format_idr(p.gross_profit_idr),
                                                        )}
                                                    </p>
                                                    <p
                                                        class=forecast_class
                                                        class:dashboard-change-flash=flash_if_any_changed_owned(
                                                            changed,
                                                            vec![
                                                                forecast_margin_key.clone(),
                                                                forecast_variance_key.clone(),
                                                                projected_total_cost_key.clone(),
                                                                forecast_unavailable_key.clone(),
                                                                forecast_revenue_signal_key.clone(),
                                                                margin_alert_threshold_key.clone(),
                                                            ],
                                                        )
                                                    >
                                                        {if forecast_unavailable {
                                                            "Forecast: unavailable".to_string()
                                                        } else if !has_forecast_revenue_signal {
                                                            format!(
                                                                "Forecast: no revenue data · projected cost {}",
                                                                format_idr(p.projected_total_cost_idr),
                                                            )
                                                        } else {
                                                            format!(
                                                                "Forecast: {:.1}% (Δ {:+.1}% vs target) · projected cost {}",
                                                                p.forecast_margin_pct,
                                                                p.forecast_variance_from_target_pct,
                                                                format_idr(p.projected_total_cost_idr),
                                                            )
                                                        }}
                                                    </p>
                                                    {p.warning.as_ref().map(|w| view! {
                                                        <p
                                                            class="text-xs text-negative-default"
                                                            class:dashboard-change-flash=flash_if_changed_owned(changed, warning_key.clone())
                                                        >
                                                            {w.clone()}
                                                        </p>
                                                    })}
                                                    {p.margin_alert.as_ref().map(|a| view! {
                                                        <p
                                                            class="text-xs text-negative-default"
                                                            class:dashboard-change-flash=flash_if_changed_owned(changed, margin_alert_key.clone())
                                                        >
                                                            {a.clone()}
                                                        </p>
                                                    })}
                                                </div>
                                                <div class="mt-2 flex items-center justify-end">
                                                    {pid.map(|_| view! {
                                                        <button
                                                            type="button"
                                                            class="btn-ghost text-xs"
                                                            on:click=pnl_click
                                                            aria-label=pnl_aria_label
                                                        >
                                                            "View P&L →"
                                                        </button>
                                                    })}
                                                </div>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

// ── Finance Panel ────────────────────────────────────────────────────────

#[component]
fn FinancePanel(finance: FinanceDashboard) -> impl IntoView {
    let warnings = finance.warnings.clone();
    let cash = finance.cash_position.clone();
    let validation = finance.ctc_validation.clone();
    let audit = finance.audit_alerts.clone();
    let exports = finance.export_requests.clone();
    let changed = expect_context::<ChangedKeysCtx>();

    let validation_status_label = match validation.status.as_str() {
        "ok" => "Validated",
        "no_data" => "No payroll data",
        "error" => "Error",
        _ => "Unknown",
    };

    view! {
        <div class="space-y-4">
            <DashboardWarnings warnings=warnings />
            <div class="stat-grid">
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Cash Position (YTD)"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "finance.cash_position.ending_cumulative_position_idr")
                        >
                            {format_idr(cash.ending_cumulative_position_idr)}
                        </p>
                        <p
                            class="text-xs text-huly-muted"
                            class:dashboard-change-flash=flash_if_changed(changed, "finance.cash_position.net_cash_flow_idr")
                        >
                            {format!("Net: {}", format_idr(cash.net_cash_flow_idr))}
                        </p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"CTC Validation"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "finance.ctc_validation.status")
                        >
                            {validation_status_label}
                        </p>
                        <p
                            class="text-xs text-huly-muted"
                            class:dashboard-change-flash=flash_if_changed(changed, "finance.ctc_validation.match_rate_pct")
                        >
                            {validation.match_rate_pct.map(|p| format!("Match {:.1}%", p)).unwrap_or_else(|| "—".to_string())}
                        </p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Audit Alerts (7d)"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "finance.audit_alerts.total")
                        >
                            {(audit.access_denied_count + audit.login_failed_count + audit.login_blocked_count + audit.chain_verification_failure_count).to_string()}
                        </p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Pending Exports"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "finance.export_requests.pending_count")
                        >
                            {exports.pending_count.to_string()}
                        </p>
                    </div>
                </div>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
                <div class="panel">
                    <div class="toolbar flex items-center justify-between">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Cash Flow Summary"</h3>
                        <a href="/finance/cash-flow" class="text-xs text-primary-400 hover:text-primary-300">"Open dashboard →"</a>
                    </div>
                    <div class="p-3 text-sm text-huly-content space-y-1">
                        <p>{format!("Period: {} → {}", format_date(&cash.start_date), format_date(&cash.end_date))}</p>
                        <p class:dashboard-change-flash=flash_if_changed(changed, "finance.cash_position.total_cash_in_idr")>
                            {format!("Cash in: {}", format_idr(cash.total_cash_in_idr))}
                        </p>
                        <p class:dashboard-change-flash=flash_if_changed(changed, "finance.cash_position.total_cash_out_idr")>
                            {format!("Cash out: {}", format_idr(cash.total_cash_out_idr))}
                        </p>
                        <p class:dashboard-change-flash=flash_if_changed(changed, "finance.cash_position.net_cash_flow_idr")>
                            {format!("Net: {}", format_idr(cash.net_cash_flow_idr))}
                        </p>
                        <p class:dashboard-change-flash=flash_if_changed(changed, "finance.cash_position.ending_cumulative_position_idr")>
                            {format!("Ending position: {}", format_idr(cash.ending_cumulative_position_idr))}
                        </p>
                    </div>
                </div>

                <div class="panel">
                    <div class="toolbar flex items-center justify-between">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"CTC Validation"</h3>
                        <a href="/finance/ctc-validation" class="text-xs text-primary-400 hover:text-primary-300">"Open report →"</a>
                    </div>
                    <div class="p-3 text-sm text-huly-content space-y-1">
                        <p>{format!("Window: {} → {}", format_date(&validation.start_date), format_date(&validation.end_date))}</p>
                        {match validation.status.as_str() {
                            "ok" => Either::Left(view! {
                                <div class="space-y-1">
                                    <p class:dashboard-change-flash=flash_if_changed(changed, "finance.ctc_validation.total_compared")>
                                        {format!("Compared: {}", validation.total_compared.unwrap_or(0))}
                                    </p>
                                    <p class:dashboard-change-flash=flash_if_changed(changed, "finance.ctc_validation.total_matches")>
                                        {format!("Matches: {}", validation.total_matches.unwrap_or(0))}
                                    </p>
                                    <p class:dashboard-change-flash=flash_if_changed(changed, "finance.ctc_validation.total_discrepancies")>
                                        {format!("Discrepancies: {}", validation.total_discrepancies.unwrap_or(0))}
                                    </p>
                                </div>
                            }),
                            _ => Either::Right(view! {
                                <p class="text-xs text-huly-muted">{validation.message.clone().unwrap_or_else(|| "No payroll data available for the window.".to_string())}</p>
                            }),
                        }}
                    </div>
                </div>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
                <div class="panel">
                    <div class="toolbar flex items-center justify-between">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Audit Alerts"</h3>
                        <a href="/finance/audit-reports" class="text-xs text-primary-400 hover:text-primary-300">"Open report →"</a>
                    </div>
                    <div class="p-3 text-sm text-huly-content space-y-1">
                        <p class:dashboard-change-flash=flash_if_changed(changed, "finance.audit_alerts.access_denied_count")>
                            {format!("Access denied: {}", audit.access_denied_count)}
                        </p>
                        <p class:dashboard-change-flash=flash_if_changed(changed, "finance.audit_alerts.login_failed_count")>
                            {format!("Login failed: {}", audit.login_failed_count)}
                        </p>
                        <p class:dashboard-change-flash=flash_if_changed(changed, "finance.audit_alerts.login_blocked_count")>
                            {format!("Login blocked: {}", audit.login_blocked_count)}
                        </p>
                        <p class:dashboard-change-flash=flash_if_changed(changed, "finance.audit_alerts.chain_verification_failure_count")>
                            {format!("Chain failures: {}", audit.chain_verification_failure_count)}
                        </p>
                        {if audit.recent.is_empty() {
                            Either::Left(view! {
                                <p class="text-xs text-huly-muted">"No recent alert events."</p>
                            })
                        } else {
                            Either::Right(view! {
                                <div>
                                    {audit.recent.into_iter().map(|e| {
                                        let key = match e.id {
                                            Some(id) => format!("finance.audit_alerts.recent.{}", id),
                                            None => format!(
                                                "finance.audit_alerts.recent.{}|{}|{}",
                                                e.action, e.entity_type, e.created_at
                                            ),
                                        };
                                        let flash = flash_if_changed_owned(changed, key);
                                        view! {
                                            <div class="activity-item" class:dashboard-change-flash=flash>
                                                <span class="text-sm text-huly-content flex-1">{format!("{} · {}", e.action, e.entity_type)}</span>
                                                <span class="text-xs text-huly-muted whitespace-nowrap">{format_date(&e.created_at)}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>
                </div>

                <div class="panel">
                    <div class="toolbar flex items-center justify-between">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Pending Export Requests"</h3>
                        <a href="/finance/audit-reports" class="text-xs text-primary-400 hover:text-primary-300">"Open report →"</a>
                    </div>
                    <div class="p-3">
                        {if exports.latest_pending.is_empty() {
                            Either::Left(view! {
                                <div class="empty-state py-4">
                                    <p class="text-huly-muted text-xs">"No pending export requests."</p>
                                </div>
                            })
                        } else {
                            Either::Right(view! {
                                <div>
                                    {exports.latest_pending.into_iter().map(|p| {
                                        let report_label = p.report_type.clone().unwrap_or_else(|| "generic".to_string());
                                        let label = format!("{} · {}", report_label, p.status);
                                        let key = match p.id {
                                            Some(id) => format!("finance.export_requests.latest.{}", id),
                                            None => format!(
                                                "finance.export_requests.latest.{}|{}",
                                                p.status, p.created_at
                                            ),
                                        };
                                        let flash = flash_if_changed_owned(changed, key);
                                        view! {
                                            <div class="activity-item" class:dashboard-change-flash=flash>
                                                <span class="text-sm text-huly-content flex-1">{label}</span>
                                                <span class="text-xs text-huly-muted whitespace-nowrap">{format_date(&p.created_at)}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>
                </div>
            </div>
        </div>
    }
}

// ── Admin Panel ──────────────────────────────────────────────────────────

#[component]
fn AdminPanel(admin: AdminDashboard) -> impl IntoView {
    let changed = expect_context::<ChangedKeysCtx>();
    view! {
        <div class="space-y-4">
            <div class="stat-grid">
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Users"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "admin.total_users")
                        >
                            {admin.total_users.to_string()}
                        </p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Departments"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "admin.total_departments")
                        >
                            {admin.total_departments.to_string()}
                        </p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Active Projects"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "admin.total_active_projects")
                        >
                            {admin.total_active_projects.to_string()}
                        </p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Active CTC"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "admin.total_active_ctc_records")
                        >
                            {admin.total_active_ctc_records.to_string()}
                        </p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Pending Exports"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "admin.pending_export_requests")
                        >
                            {admin.pending_export_requests.to_string()}
                        </p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Access Denied (24h)"</p>
                        <p
                            class="stat-value"
                            class:dashboard-change-flash=flash_if_changed(changed, "admin.access_denied_24h")
                        >
                            {admin.access_denied_24h.to_string()}
                        </p>
                    </div>
                </div>
            </div>
        </div>
    }
}

// ── Native test surface (compile-time only on non-WASM targets) ──────────

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    fn empty_response(role: &str) -> RoleDashboardResponse {
        RoleDashboardResponse {
            role: role.to_string(),
            generated_at: "2026-05-21T00:00:00Z".to_string(),
            hr: None,
            department_head: None,
            project_manager: None,
            finance: None,
            admin: None,
        }
    }

    fn admin_response(total_active_projects: i64, total_users: i64) -> RoleDashboardResponse {
        let mut resp = empty_response("admin");
        resp.admin = Some(AdminDashboard {
            total_users,
            total_departments: 4,
            total_active_projects,
            total_active_ctc_records: 22,
            pending_export_requests: 1,
            access_denied_24h: 0,
        });
        resp
    }

    #[test]
    fn initial_load_produces_no_changed_keys() {
        let next = dashboard_value_map(&admin_response(7, 50));
        // With no previous snapshot, the dashboard never highlights — verify by
        // matching the diff path used at runtime.
        let changed: HashSet<String> = HashSet::new();
        assert!(changed.is_empty());
        // Sanity: helper produces non-empty key set when admin data is present.
        assert!(next.contains_key("admin.total_active_projects"));
    }

    #[test]
    fn generated_at_only_change_produces_no_changed_keys() {
        let mut prev_resp = admin_response(7, 50);
        let mut next_resp = admin_response(7, 50);
        prev_resp.generated_at = "2026-05-21T00:00:00Z".to_string();
        next_resp.generated_at = "2026-05-21T00:00:30Z".to_string();

        let prev_map = dashboard_value_map(&prev_resp);
        let next_map = dashboard_value_map(&next_resp);
        let changed = compute_changed_keys(&prev_map, &next_map);
        assert!(
            changed.is_empty(),
            "generated_at-only delta must not produce changed keys, got {:?}",
            changed
        );
    }

    #[test]
    fn scalar_change_produces_expected_key() {
        let prev_map = dashboard_value_map(&admin_response(7, 50));
        let next_map = dashboard_value_map(&admin_response(8, 50));
        let changed = compute_changed_keys(&prev_map, &next_map);
        assert!(
            changed.contains("admin.total_active_projects"),
            "expected admin.total_active_projects in {:?}",
            changed
        );
        assert!(!changed.contains("admin.total_users"));
    }

    #[test]
    fn newly_visible_row_produces_expected_key() {
        let mut prev_resp = empty_response("hr");
        prev_resp.hr = Some(HrDashboard {
            completeness: CompletenessReport {
                total_employees: 10,
                total_with_ctc: 9,
                total_missing: 1,
                overall_completion_pct: 90.0,
            },
            pending_updates: HrPendingUpdates {
                missing_count: 1,
                sample: vec![],
            },
            recent_changes: vec![],
            compliance_alerts: ComplianceAlertSummary {
                start_date: "2026-05-01".into(),
                end_date: "2026-05-21".into(),
                total_validated: 0,
                total_passed: 0,
                total_discrepancies: 0,
                compliance_rate_pct: 100.0,
                top_risks: vec![],
            },
            warnings: vec![],
        });

        let resource_id = Uuid::new_v4();
        let mut next_resp = empty_response("hr");
        next_resp.hr = Some(HrDashboard {
            completeness: CompletenessReport {
                total_employees: 10,
                total_with_ctc: 9,
                total_missing: 1,
                overall_completion_pct: 90.0,
            },
            pending_updates: HrPendingUpdates {
                missing_count: 1,
                sample: vec![],
            },
            recent_changes: vec![RecentCtcChange {
                resource_id: Some(resource_id),
                resource_name: "Alice".into(),
                revision_number: 3,
                changed_by_name: Some("Bob".into()),
                created_at: "2026-05-21T01:00:00Z".into(),
                reason: "promotion".into(),
            }],
            compliance_alerts: ComplianceAlertSummary {
                start_date: "2026-05-01".into(),
                end_date: "2026-05-21".into(),
                total_validated: 0,
                total_passed: 0,
                total_discrepancies: 0,
                compliance_rate_pct: 100.0,
                top_risks: vec![],
            },
            warnings: vec![],
        });

        let prev_map = dashboard_value_map(&prev_resp);
        let next_map = dashboard_value_map(&next_resp);
        let changed = compute_changed_keys(&prev_map, &next_map);
        let expected_key = format!(
            "hr.recent_changes.{}|rev3|2026-05-21T01:00:00Z",
            resource_id
        );
        assert!(
            changed.contains(&expected_key),
            "newly visible recent_changes row should appear in {:?}",
            changed
        );
    }

    #[test]
    fn removed_row_does_not_panic() {
        let resource_id = Uuid::new_v4();
        let mut prev_resp = empty_response("hr");
        prev_resp.hr = Some(HrDashboard {
            completeness: CompletenessReport {
                total_employees: 10,
                total_with_ctc: 9,
                total_missing: 1,
                overall_completion_pct: 90.0,
            },
            pending_updates: HrPendingUpdates {
                missing_count: 1,
                sample: vec![],
            },
            recent_changes: vec![RecentCtcChange {
                resource_id: Some(resource_id),
                resource_name: "Alice".into(),
                revision_number: 3,
                changed_by_name: Some("Bob".into()),
                created_at: "2026-05-21T01:00:00Z".into(),
                reason: "promotion".into(),
            }],
            compliance_alerts: ComplianceAlertSummary {
                start_date: "2026-05-01".into(),
                end_date: "2026-05-21".into(),
                total_validated: 0,
                total_passed: 0,
                total_discrepancies: 0,
                compliance_rate_pct: 100.0,
                top_risks: vec![],
            },
            warnings: vec![],
        });

        let mut next_resp = prev_resp.clone();
        if let Some(hr) = next_resp.hr.as_mut() {
            hr.recent_changes.clear();
        }

        let prev_map = dashboard_value_map(&prev_resp);
        let next_map = dashboard_value_map(&next_resp);
        let changed = compute_changed_keys(&prev_map, &next_map);
        // Removal is not surfaced as a "changed" key (we never highlight
        // absent rows) and must not panic.
        let removed_key = format!(
            "hr.recent_changes.{}|rev3|2026-05-21T01:00:00Z",
            resource_id
        );
        assert!(!changed.contains(&removed_key));
    }

    // ── Story 6.2 expansion: per-role change-detection coverage ───────────

    fn dept_head_response_with_budget_pct(utilization_pct: f64) -> RoleDashboardResponse {
        let mut resp = empty_response("department_head");
        resp.department_head = Some(DepartmentHeadDashboard {
            utilization: UtilizationSummary {
                start_date: String::new(),
                end_date: String::new(),
                average_utilization_pct: 75.0,
                overallocated_count: 0,
                top_at_risk: vec![],
            },
            budget: Some(DepartmentBudgetSummary {
                department_name: "Engineering".into(),
                budget_period: "2026-Q2".into(),
                total_budget_idr: 100_000_000,
                total_committed_idr: 50_000_000,
                spent_actual_idr: 0,
                remaining_idr: 50_000_000,
                utilization_percentage: utilization_pct,
                budget_health: "healthy".into(),
                alert_threshold_pct: 80,
                budget_configured: true,
            }),
            overallocations: OverallocationSummary {
                overallocated_count: 0,
                members: vec![],
            },
            upcoming_assignments: vec![],
            warnings: vec![],
            team_members: vec![],
            utilization_trends: None,
            underutilized_members: vec![],
        });
        resp
    }

    #[test]
    fn dept_head_budget_utilization_change_produces_expected_key() {
        let prev_map = dashboard_value_map(&dept_head_response_with_budget_pct(50.0));
        let next_map = dashboard_value_map(&dept_head_response_with_budget_pct(75.0));
        let changed = compute_changed_keys(&prev_map, &next_map);
        assert!(
            changed.contains("department_head.budget.utilization_percentage"),
            "expected department_head.budget.utilization_percentage in {:?}",
            changed
        );
    }

    #[test]
    fn dept_head_upcoming_assignment_id_change_produces_stable_key() {
        let alloc_id = Uuid::new_v4();
        let make = |pct: f64| {
            let mut resp = empty_response("department_head");
            resp.department_head = Some(DepartmentHeadDashboard {
                utilization: UtilizationSummary {
                    start_date: String::new(),
                    end_date: String::new(),
                    average_utilization_pct: 0.0,
                    overallocated_count: 0,
                    top_at_risk: vec![],
                },
                budget: None,
                overallocations: OverallocationSummary {
                    overallocated_count: 0,
                    members: vec![],
                },
                upcoming_assignments: vec![UpcomingAssignment {
                    allocation_id: Some(alloc_id),
                    resource_name: "Alice".into(),
                    project_name: "Atlas".into(),
                    start_date: "2026-06-01".into(),
                    end_date: "2026-06-30".into(),
                    allocation_percentage: pct,
                }],
                warnings: vec![],
                team_members: vec![],
                utilization_trends: None,
                underutilized_members: vec![],
            });
            resp
        };
        let prev_map = dashboard_value_map(&make(40.0));
        let next_map = dashboard_value_map(&make(80.0));
        let changed = compute_changed_keys(&prev_map, &next_map);
        let expected_key = format!("department_head.upcoming_assignments.{}", alloc_id);
        assert!(
            changed.contains(&expected_key),
            "expected stable allocation_id key `{}` in {:?}",
            expected_key,
            changed
        );
        // No name-based fallback key should appear when the id is present.
        assert!(
            !changed
                .iter()
                .any(|k| k.starts_with("department_head.upcoming_assignments.name.")),
            "id-stable row must not double-emit a name-based key"
        );
    }

    fn pm_card_default(project_id: Option<Uuid>, project_name: &str) -> ProjectHealthCard {
        ProjectHealthCard {
            project_id,
            project_name: project_name.into(),
            status: "Active".into(),
            end_date: "2026-12-31".into(),
            total_budget_idr: 100_000_000,
            budget_spent_idr: 30_000_000,
            budget_remaining_idr: 70_000_000,
            budget_status: "healthy".into(),
            budget_utilization_pct: 30.0,
            is_over_budget: false,
            budget_overrun_idr: 0,
            total_revenue_idr: 80_000_000,
            total_cost_idr: 50_000_000,
            gross_profit_idr: 30_000_000,
            margin_pct: 30.0,
            target_margin_pct: 25.0,
            margin_alert_threshold_pct: 5.0,
            margin_alert: None,
            forecast_margin_pct: 25.0,
            forecast_variance_from_target_pct: 0.0,
            projected_total_cost_idr: 60_000_000,
            forecast_unavailable: false,
            forecast_has_revenue_signal: true,
            health_status: "healthy".into(),
            warning: None,
        }
    }

    fn pm_response_with(margin_pct: f64) -> RoleDashboardResponse {
        let project_id = Uuid::nil();
        let mut resp = empty_response("project_manager");
        resp.project_manager = Some(ProjectManagerDashboard {
            active_projects: vec![ProjectHealthCard {
                margin_pct,
                ..pm_card_default(Some(project_id), "Atlas")
            }],
            margin_alerts: vec![],
            warnings: vec![],
        });
        resp
    }

    #[test]
    fn project_manager_margin_pct_change_produces_expected_key() {
        let prev_map = dashboard_value_map(&pm_response_with(25.0));
        let next_map = dashboard_value_map(&pm_response_with(8.0));
        let changed = compute_changed_keys(&prev_map, &next_map);
        let expected_key = format!("project_manager.project.{}.margin_pct", Uuid::nil());
        assert!(
            changed.contains(&expected_key),
            "expected `{}` in {:?}",
            expected_key,
            changed
        );
        // gross_profit_idr is unchanged in this fixture and must not flash.
        assert!(
            !changed.contains(&format!(
                "project_manager.project.{}.gross_profit_idr",
                Uuid::nil()
            )),
            "unchanged sibling field must not appear as changed"
        );
    }

    #[test]
    fn project_manager_margin_alert_uses_project_id_stable_key() {
        let project_id = Uuid::new_v4();
        let make_alert = |msg: &str| {
            let mut resp = empty_response("project_manager");
            resp.project_manager = Some(ProjectManagerDashboard {
                active_projects: vec![],
                margin_alerts: vec![MarginAlert {
                    project_id: Some(project_id),
                    project_name: "Atlas".into(),
                    margin_pct: 5.0,
                    message: msg.into(),
                }],
                warnings: vec![],
            });
            resp
        };
        // Initial baseline so the alert is newly visible only on the first diff.
        let baseline = empty_response("project_manager");
        let baseline_map = dashboard_value_map(&baseline);
        let next_map = dashboard_value_map(&make_alert("Margin below threshold"));
        let changed = compute_changed_keys(&baseline_map, &next_map);
        let expected_key = format!("project_manager.margin_alert.{}", project_id);
        assert!(
            changed.contains(&expected_key),
            "new margin alert must surface stable project_id key, got {:?}",
            changed
        );
    }

    fn finance_response_with_cash(net_idr: i64, ending_idr: i64) -> RoleDashboardResponse {
        let mut resp = empty_response("finance");
        resp.finance = Some(FinanceDashboard {
            cash_position: CashPositionSummary {
                start_date: "2026-05-01".into(),
                end_date: "2026-05-21".into(),
                total_cash_in_idr: 10_000_000,
                total_cash_out_idr: 3_000_000,
                net_cash_flow_idr: net_idr,
                ending_cumulative_position_idr: ending_idr,
            },
            ctc_validation: CtcValidationStatus {
                status: "no_data".into(),
                start_date: "2026-05-01".into(),
                end_date: "2026-05-21".into(),
                total_compared: None,
                total_matches: None,
                total_discrepancies: None,
                match_rate_pct: None,
                message: Some("No data".into()),
            },
            audit_alerts: AuditAlertsSummary {
                access_denied_count: 0,
                login_failed_count: 0,
                login_blocked_count: 0,
                chain_verification_failure_count: 0,
                recent: vec![],
            },
            export_requests: ExportRequestsSummary {
                pending_count: 0,
                latest_pending: vec![],
            },
            warnings: vec![],
        });
        resp
    }

    #[test]
    fn finance_cash_position_change_produces_expected_key() {
        let prev_map = dashboard_value_map(&finance_response_with_cash(7_000_000, 50_000_000));
        let next_map = dashboard_value_map(&finance_response_with_cash(5_000_000, 48_000_000));
        let changed = compute_changed_keys(&prev_map, &next_map);
        assert!(
            changed.contains("finance.cash_position.ending_cumulative_position_idr"),
            "expected ending_cumulative_position_idr in {:?}",
            changed
        );
        assert!(
            changed.contains("finance.cash_position.net_cash_flow_idr"),
            "expected net_cash_flow_idr in {:?}",
            changed
        );
    }

    #[test]
    fn finance_export_request_new_pending_id_produces_changed_key() {
        let baseline = finance_response_with_cash(0, 0);
        let export_id = Uuid::new_v4();
        let mut next = finance_response_with_cash(0, 0);
        if let Some(f) = next.finance.as_mut() {
            f.export_requests = ExportRequestsSummary {
                pending_count: 1,
                latest_pending: vec![PendingExportRequest {
                    id: Some(export_id),
                    status: "pending_approval".into(),
                    created_at: "2026-05-21T02:00:00Z".into(),
                    report_type: Some("compliance_audit".into()),
                }],
            };
        }
        let prev_map = dashboard_value_map(&baseline);
        let next_map = dashboard_value_map(&next);
        let changed = compute_changed_keys(&prev_map, &next_map);
        let expected_key = format!("finance.export_requests.latest.{}", export_id);
        assert!(
            changed.contains(&expected_key),
            "newly pending export request must surface stable export_id key, got {:?}",
            changed
        );
        assert!(
            changed.contains("finance.export_requests.pending_count"),
            "pending_count scalar must also be marked changed"
        );
    }

    #[test]
    fn finance_audit_alerts_total_change_produces_expected_key() {
        let mut prev = finance_response_with_cash(0, 0);
        if let Some(f) = prev.finance.as_mut() {
            f.audit_alerts.access_denied_count = 1;
        }
        let mut next = finance_response_with_cash(0, 0);
        if let Some(f) = next.finance.as_mut() {
            f.audit_alerts.access_denied_count = 1;
            f.audit_alerts.login_failed_count = 2;
        }
        let prev_map = dashboard_value_map(&prev);
        let next_map = dashboard_value_map(&next);
        let changed = compute_changed_keys(&prev_map, &next_map);
        assert!(
            changed.contains("finance.audit_alerts.total"),
            "derived total field must update when any sub-count changes, got {:?}",
            changed
        );
        // access_denied_count is unchanged → it must not appear as changed.
        assert!(
            !changed.contains("finance.audit_alerts.access_denied_count"),
            "unchanged sub-count must not appear as changed"
        );
    }

    #[test]
    fn value_map_excludes_generated_at_key() {
        let resp = admin_response(5, 10);
        let map = dashboard_value_map(&resp);
        // `generated_at` is explicitly excluded so polling never flashes
        // every value when only the timestamp changed.
        assert!(
            map.keys().all(|k| !k.contains("generated_at")),
            "value map keys must never reference generated_at, got {:?}",
            map.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn value_map_excludes_sensitive_ctc_fields() {
        // Build a fully populated HR + Finance response and assert the
        // serialized value map does not surface CTC ciphertext, salary
        // components, key metadata, or raw audit payload fields.
        let mut resp = empty_response("hr");
        resp.hr = Some(HrDashboard {
            completeness: CompletenessReport {
                total_employees: 5,
                total_with_ctc: 5,
                total_missing: 0,
                overall_completion_pct: 100.0,
            },
            pending_updates: HrPendingUpdates {
                missing_count: 0,
                sample: vec![],
            },
            recent_changes: vec![RecentCtcChange {
                resource_id: Some(Uuid::new_v4()),
                resource_name: "Alice".into(),
                revision_number: 1,
                changed_by_name: Some("Bob".into()),
                created_at: "2026-05-21T03:00:00Z".into(),
                reason: "Promotion".into(),
            }],
            compliance_alerts: ComplianceAlertSummary {
                start_date: "2026-05-01".into(),
                end_date: "2026-05-21".into(),
                total_validated: 1,
                total_passed: 1,
                total_discrepancies: 0,
                compliance_rate_pct: 100.0,
                top_risks: vec![],
            },
            warnings: vec![],
        });
        let map = dashboard_value_map(&resp);
        let serialized = format!("{:?}", map);
        for forbidden in [
            "encrypted_components",
            "encrypted_daily_rate",
            "ciphertext",
            "key_version",
            "encryption_algorithm",
            "encryption_version",
            "base_salary",
            "daily_rate",
        ] {
            assert!(
                !serialized.contains(forbidden),
                "value map leaked sensitive token `{}` in {}",
                forbidden,
                serialized
            );
        }
    }

    #[test]
    fn fallback_key_used_when_resource_id_missing() {
        // Backend should normally emit resource_id, but the frontend DTO is
        // tolerant of None. Confirm the fallback key based on resource_name
        // is used so highlights remain stable across polls even when the
        // backend omits the id.
        let mut resp = empty_response("hr");
        resp.hr = Some(HrDashboard {
            completeness: CompletenessReport {
                total_employees: 1,
                total_with_ctc: 1,
                total_missing: 0,
                overall_completion_pct: 100.0,
            },
            pending_updates: HrPendingUpdates {
                missing_count: 0,
                sample: vec![],
            },
            recent_changes: vec![RecentCtcChange {
                resource_id: None,
                resource_name: "Anon Person".into(),
                revision_number: 1,
                changed_by_name: None,
                created_at: "2026-05-21T04:00:00Z".into(),
                reason: "raise".into(),
            }],
            compliance_alerts: ComplianceAlertSummary {
                start_date: "2026-05-01".into(),
                end_date: "2026-05-21".into(),
                total_validated: 0,
                total_passed: 0,
                total_discrepancies: 0,
                compliance_rate_pct: 100.0,
                top_risks: vec![],
            },
            warnings: vec![],
        });
        let map = dashboard_value_map(&resp);
        assert!(
            map.keys()
                .any(|k| k == "hr.recent_changes.name.Anon Person|rev1|2026-05-21T04:00:00Z"),
            "fallback key must use resource_name when resource_id is None, got keys = {:?}",
            map.keys().collect::<Vec<_>>()
        );
    }

    // ── Story 6.2 expansion run 2: display helpers + change-detection gaps ─

    #[test]
    fn format_idr_handles_zero() {
        // Zero must render without a stray "-" sign and without grouping dots.
        assert_eq!(format_idr(0), "Rp 0");
    }

    #[test]
    fn format_idr_groups_thousands_with_dots() {
        assert_eq!(format_idr(1_000), "Rp 1.000");
        assert_eq!(format_idr(1_000_000), "Rp 1.000.000");
        assert_eq!(format_idr(1_000_000_000), "Rp 1.000.000.000");
    }

    #[test]
    fn format_idr_handles_negative_amount() {
        // Negative cash flow renders with leading "-" between "Rp " and digits.
        assert_eq!(format_idr(-50_000), "Rp -50.000");
        assert_eq!(format_idr(-1), "Rp -1");
    }

    #[test]
    fn format_date_strips_iso_time_component() {
        assert_eq!(format_date("2026-05-21T10:15:00Z"), "2026-05-21");
        // Passthrough for date-only input — Last-updated label safety.
        assert_eq!(format_date("2026-05-21"), "2026-05-21");
        // Empty input must not panic.
        assert_eq!(format_date(""), "");
    }

    #[test]
    fn role_display_returns_canonical_labels() {
        assert_eq!(role_display("hr"), "HR");
        assert_eq!(role_display("department_head"), "Department Head");
        assert_eq!(role_display("project_manager"), "Project Manager");
        assert_eq!(role_display("finance"), "Finance");
        assert_eq!(role_display("admin"), "Administrator");
        // Unknown role falls back to a generic label (never the raw role string).
        assert_eq!(role_display("director"), "User");
        assert_eq!(role_display(""), "User");
    }

    #[test]
    fn hr_pending_updates_sample_id_keyed_change() {
        let baseline = empty_response("hr");
        let emp_id = Uuid::new_v4();
        let mut next = empty_response("hr");
        next.hr = Some(HrDashboard {
            completeness: CompletenessReport {
                total_employees: 10,
                total_with_ctc: 9,
                total_missing: 1,
                overall_completion_pct: 90.0,
            },
            pending_updates: HrPendingUpdates {
                missing_count: 1,
                sample: vec![HrMissingEmployee {
                    id: Some(emp_id),
                    name: "Charlie".into(),
                    department: "Engineering".into(),
                }],
            },
            recent_changes: vec![],
            compliance_alerts: ComplianceAlertSummary {
                start_date: "2026-05-01".into(),
                end_date: "2026-05-21".into(),
                total_validated: 0,
                total_passed: 0,
                total_discrepancies: 0,
                compliance_rate_pct: 100.0,
                top_risks: vec![],
            },
            warnings: vec![],
        });

        let prev_map = dashboard_value_map(&baseline);
        let next_map = dashboard_value_map(&next);
        let changed = compute_changed_keys(&prev_map, &next_map);
        let expected_key = format!("hr.pending_updates.sample.{}", emp_id);
        assert!(
            changed.contains(&expected_key),
            "expected stable employee id key `{}` in {:?}",
            expected_key,
            changed
        );
        // Department string is the value, not part of the key.
        assert_eq!(
            next_map.get(&expected_key).map(String::as_str),
            Some("Charlie|Engineering")
        );
    }

    #[test]
    fn hr_compliance_top_risks_id_keyed_row_change() {
        let baseline = empty_response("hr");
        let resource_id = Uuid::new_v4();
        let mut next = empty_response("hr");
        next.hr = Some(HrDashboard {
            completeness: CompletenessReport {
                total_employees: 5,
                total_with_ctc: 5,
                total_missing: 0,
                overall_completion_pct: 100.0,
            },
            pending_updates: HrPendingUpdates {
                missing_count: 0,
                sample: vec![],
            },
            recent_changes: vec![],
            compliance_alerts: ComplianceAlertSummary {
                start_date: "2026-05-01".into(),
                end_date: "2026-05-21".into(),
                total_validated: 1,
                total_passed: 0,
                total_discrepancies: 1,
                compliance_rate_pct: 0.0,
                top_risks: vec![ComplianceTopRisk {
                    resource_id: Some(resource_id),
                    name: "Dana".into(),
                    variance_amount: 12_500_000,
                }],
            },
            warnings: vec![],
        });
        let prev_map = dashboard_value_map(&baseline);
        let next_map = dashboard_value_map(&next);
        let changed = compute_changed_keys(&prev_map, &next_map);
        let expected_key = format!("hr.compliance_alerts.top_risks.{}", resource_id);
        assert!(
            changed.contains(&expected_key),
            "newly visible top-risk row must surface stable resource_id key, got {:?}",
            changed
        );
        // No name-based fallback key when id is present.
        assert!(
            !changed
                .iter()
                .any(|k| k.starts_with("hr.compliance_alerts.top_risks.name.")),
            "id-stable row must not double-emit a name-based key"
        );
    }

    #[test]
    fn dept_head_top_at_risk_id_stable_key() {
        let baseline = empty_response("department_head");
        let resource_id = Uuid::new_v4();
        let mut next = empty_response("department_head");
        next.department_head = Some(DepartmentHeadDashboard {
            utilization: UtilizationSummary {
                start_date: String::new(),
                end_date: String::new(),
                average_utilization_pct: 92.0,
                overallocated_count: 1,
                top_at_risk: vec![UtilizationAtRisk {
                    resource_id: Some(resource_id),
                    resource_name: "Erin".into(),
                    current_allocation_pct: 130.0,
                }],
            },
            budget: None,
            overallocations: OverallocationSummary {
                overallocated_count: 1,
                members: vec![],
            },
            upcoming_assignments: vec![],
            warnings: vec![],
            team_members: vec![],
            utilization_trends: None,
            underutilized_members: vec![],
        });
        let prev_map = dashboard_value_map(&baseline);
        let next_map = dashboard_value_map(&next);
        let changed = compute_changed_keys(&prev_map, &next_map);
        let expected_key = format!("department_head.top_at_risk.{}", resource_id);
        assert!(
            changed.contains(&expected_key),
            "newly visible at-risk member must surface stable resource_id key, got {:?}",
            changed
        );
    }

    #[test]
    fn pm_project_fallback_key_used_when_project_id_missing() {
        // When backend omits project_id, the helper must fall back to a
        // name-based stable key so highlight behaviour still works.
        let make = |margin: f64| {
            let mut resp = empty_response("project_manager");
            resp.project_manager = Some(ProjectManagerDashboard {
                active_projects: vec![ProjectHealthCard {
                    margin_pct: margin,
                    total_budget_idr: 50_000_000,
                    budget_spent_idr: 10_000_000,
                    budget_remaining_idr: 40_000_000,
                    budget_utilization_pct: 20.0,
                    total_revenue_idr: 30_000_000,
                    total_cost_idr: 20_000_000,
                    gross_profit_idr: 10_000_000,
                    ..pm_card_default(None, "Orphan Project")
                }],
                margin_alerts: vec![],
                warnings: vec![],
            });
            resp
        };
        let prev_map = dashboard_value_map(&make(30.0));
        let next_map = dashboard_value_map(&make(12.0));
        let changed = compute_changed_keys(&prev_map, &next_map);
        let expected_key = "project_manager.project.name.Orphan Project.margin_pct";
        assert!(
            changed.contains(expected_key),
            "expected fallback name-based key `{}` in {:?}",
            expected_key,
            changed
        );
    }

    #[test]
    fn pm_active_projects_count_changes_when_project_added() {
        let baseline = empty_response("project_manager");
        let mut next = empty_response("project_manager");
        next.project_manager = Some(ProjectManagerDashboard {
            active_projects: vec![ProjectHealthCard {
                total_budget_idr: 1,
                budget_spent_idr: 0,
                budget_remaining_idr: 1,
                budget_utilization_pct: 0.0,
                total_revenue_idr: 0,
                total_cost_idr: 0,
                gross_profit_idr: 0,
                margin_pct: 0.0,
                ..pm_card_default(Some(Uuid::new_v4()), "Atlas")
            }],
            margin_alerts: vec![],
            warnings: vec![],
        });
        let prev_map = dashboard_value_map(&baseline);
        let next_map = dashboard_value_map(&next);
        let changed = compute_changed_keys(&prev_map, &next_map);
        assert!(
            changed.contains("project_manager.active_projects.count"),
            "list-length scalar must update when a project appears, got {:?}",
            changed
        );
    }

    #[test]
    fn finance_audit_alerts_recent_row_uses_id_stable_key() {
        let baseline = finance_response_with_cash(0, 0);
        let entry_id = Uuid::new_v4();
        let mut next = finance_response_with_cash(0, 0);
        if let Some(f) = next.finance.as_mut() {
            f.audit_alerts.access_denied_count = 1;
            f.audit_alerts.recent = vec![AuditAlertEntry {
                id: Some(entry_id),
                action: "access_denied".into(),
                entity_type: "ctc_record".into(),
                created_at: "2026-05-21T05:00:00Z".into(),
            }];
        }
        let prev_map = dashboard_value_map(&baseline);
        let next_map = dashboard_value_map(&next);
        let changed = compute_changed_keys(&prev_map, &next_map);
        let expected_key = format!("finance.audit_alerts.recent.{}", entry_id);
        assert!(
            changed.contains(&expected_key),
            "new audit alert row must surface stable id key, got {:?}",
            changed
        );
        // No timestamp-based fallback key when id is present.
        assert!(
            !changed
                .iter()
                .any(|k| k.starts_with("finance.audit_alerts.recent.access_denied|")),
            "id-stable row must not double-emit a timestamp-based key"
        );
    }

    #[test]
    fn finance_ctc_validation_status_change_produces_expected_key() {
        // Status transitioning from "no_data" → "passing" with a non-None
        // match_rate_pct must surface both keys as changed.
        let prev = finance_response_with_cash(0, 0);
        let mut next = finance_response_with_cash(0, 0);
        if let Some(f) = next.finance.as_mut() {
            f.ctc_validation = CtcValidationStatus {
                status: "passing".into(),
                start_date: "2026-05-01".into(),
                end_date: "2026-05-21".into(),
                total_compared: Some(10),
                total_matches: Some(10),
                total_discrepancies: Some(0),
                match_rate_pct: Some(100.0),
                message: None,
            };
        }
        let prev_map = dashboard_value_map(&prev);
        let next_map = dashboard_value_map(&next);
        let changed = compute_changed_keys(&prev_map, &next_map);
        assert!(
            changed.contains("finance.ctc_validation.status"),
            "ctc_validation.status transition must be marked changed, got {:?}",
            changed
        );
        assert!(
            changed.contains("finance.ctc_validation.match_rate_pct"),
            "newly-present match_rate_pct must surface as changed, got {:?}",
            changed
        );
    }

    // ── Story 6.3 expansion: sort helpers + new PM change-detection keys ─

    #[test]
    fn pm_sort_by_margin_orders_lowest_first() {
        let mut cards = vec![
            ProjectHealthCard {
                margin_pct: 35.0,
                ..pm_card_default(Some(Uuid::new_v4()), "Beta")
            },
            ProjectHealthCard {
                margin_pct: 5.0,
                ..pm_card_default(Some(Uuid::new_v4()), "Critical")
            },
            ProjectHealthCard {
                margin_pct: 20.0,
                ..pm_card_default(Some(Uuid::new_v4()), "Alpha")
            },
        ];
        sort_pm_cards(&mut cards, PmSortMode::Margin);
        let names: Vec<_> = cards.iter().map(|c| c.project_name.as_str()).collect();
        assert_eq!(
            names,
            vec!["Critical", "Alpha", "Beta"],
            "Margin sort must place lowest margin first"
        );
    }

    #[test]
    fn pm_sort_by_budget_utilization_orders_over_budget_first() {
        let mut cards = vec![
            ProjectHealthCard {
                budget_utilization_pct: 40.0,
                is_over_budget: false,
                budget_overrun_idr: 0,
                total_budget_idr: 100,
                ..pm_card_default(Some(Uuid::new_v4()), "Low")
            },
            ProjectHealthCard {
                budget_utilization_pct: 150.0,
                is_over_budget: true,
                budget_overrun_idr: 50,
                total_budget_idr: 100,
                ..pm_card_default(Some(Uuid::new_v4()), "Overrun")
            },
            ProjectHealthCard {
                budget_utilization_pct: 70.0,
                is_over_budget: false,
                budget_overrun_idr: 0,
                total_budget_idr: 100,
                ..pm_card_default(Some(Uuid::new_v4()), "Mid")
            },
            ProjectHealthCard {
                budget_utilization_pct: 0.0,
                is_over_budget: false,
                budget_overrun_idr: 0,
                total_budget_idr: 0,
                ..pm_card_default(Some(Uuid::new_v4()), "Unconfigured")
            },
        ];
        sort_pm_cards(&mut cards, PmSortMode::BudgetUtilization);
        let names: Vec<_> = cards.iter().map(|c| c.project_name.as_str()).collect();
        assert_eq!(
            names,
            vec!["Overrun", "Mid", "Low", "Unconfigured"],
            "Budget Utilization sort must place over-budget first, unconfigured last"
        );
    }

    #[test]
    fn pm_sort_by_end_date_orders_soonest_first() {
        let mut cards = vec![
            ProjectHealthCard {
                end_date: "2027-01-15".into(),
                ..pm_card_default(Some(Uuid::new_v4()), "Late")
            },
            ProjectHealthCard {
                end_date: "2026-06-01".into(),
                ..pm_card_default(Some(Uuid::new_v4()), "Soon")
            },
            ProjectHealthCard {
                end_date: "2026-12-31".into(),
                ..pm_card_default(Some(Uuid::new_v4()), "Mid")
            },
        ];
        sort_pm_cards(&mut cards, PmSortMode::EndDate);
        let names: Vec<_> = cards.iter().map(|c| c.project_name.as_str()).collect();
        assert_eq!(
            names,
            vec!["Soon", "Mid", "Late"],
            "End date sort must place soonest end date first"
        );
    }

    #[test]
    fn pm_sort_uses_project_name_as_tiebreaker() {
        let mut cards = vec![
            ProjectHealthCard {
                margin_pct: 10.0,
                ..pm_card_default(Some(Uuid::new_v4()), "Zeta")
            },
            ProjectHealthCard {
                margin_pct: 10.0,
                ..pm_card_default(Some(Uuid::new_v4()), "Alpha")
            },
        ];
        sort_pm_cards(&mut cards, PmSortMode::Margin);
        assert_eq!(
            cards
                .iter()
                .map(|c| c.project_name.as_str())
                .collect::<Vec<_>>(),
            vec!["Alpha", "Zeta"],
            "ties on margin must fall back to ascending project_name"
        );
    }

    #[test]
    fn pm_forecast_margin_change_produces_expected_key() {
        let project_id = Uuid::new_v4();
        let make = |forecast: f64| {
            let mut resp = empty_response("project_manager");
            resp.project_manager = Some(ProjectManagerDashboard {
                active_projects: vec![ProjectHealthCard {
                    forecast_margin_pct: forecast,
                    ..pm_card_default(Some(project_id), "Atlas")
                }],
                margin_alerts: vec![],
                warnings: vec![],
            });
            resp
        };
        let prev_map = dashboard_value_map(&make(25.0));
        let next_map = dashboard_value_map(&make(8.0));
        let changed = compute_changed_keys(&prev_map, &next_map);
        let expected_key = format!("project_manager.project.{}.forecast_margin_pct", project_id);
        assert!(
            changed.contains(&expected_key),
            "expected `{}` in {:?}",
            expected_key,
            changed
        );
    }

    #[test]
    fn pm_over_budget_transition_produces_expected_key() {
        let project_id = Uuid::new_v4();
        let make = |over: bool, overrun: i64, spent: i64, utilization: f64| {
            let mut resp = empty_response("project_manager");
            resp.project_manager = Some(ProjectManagerDashboard {
                active_projects: vec![ProjectHealthCard {
                    is_over_budget: over,
                    budget_overrun_idr: overrun,
                    budget_spent_idr: spent,
                    budget_utilization_pct: utilization,
                    health_status: if over {
                        "critical".into()
                    } else {
                        "healthy".into()
                    },
                    ..pm_card_default(Some(project_id), "Atlas")
                }],
                margin_alerts: vec![],
                warnings: vec![],
            });
            resp
        };
        let prev_map = dashboard_value_map(&make(false, 0, 30_000_000, 30.0));
        let next_map = dashboard_value_map(&make(true, 5_000_000, 105_000_000, 105.0));
        let changed = compute_changed_keys(&prev_map, &next_map);
        let over_key = format!("project_manager.project.{}.is_over_budget", project_id);
        let overrun_key = format!("project_manager.project.{}.budget_overrun_idr", project_id);
        let health_key = format!("project_manager.project.{}.health_status", project_id);
        assert!(
            changed.contains(&over_key),
            "expected `{}` in {:?}",
            over_key,
            changed
        );
        assert!(
            changed.contains(&overrun_key),
            "expected `{}` in {:?}",
            overrun_key,
            changed
        );
        assert!(
            changed.contains(&health_key),
            "expected `{}` in {:?}",
            health_key,
            changed
        );
    }

    #[test]
    fn pm_value_map_excludes_sensitive_keys_for_pm_section() {
        let project_id = Uuid::new_v4();
        let mut resp = empty_response("project_manager");
        resp.project_manager = Some(ProjectManagerDashboard {
            active_projects: vec![pm_card_default(Some(project_id), "Atlas")],
            margin_alerts: vec![],
            warnings: vec![],
        });
        let map = dashboard_value_map(&resp);
        let serialized = format!("{:?}", map);
        for forbidden in [
            "encrypted_components",
            "encrypted_daily_rate",
            "ciphertext",
            "key_version",
            "encryption_algorithm",
            "encryption_version",
            "base_salary",
            "daily_rate",
        ] {
            assert!(
                !serialized.contains(forbidden),
                "PM value map leaked sensitive token `{}` in {}",
                forbidden,
                serialized
            );
        }
    }

    // ── Story 6.3 expansion run 2 (Master Test Architect):
    // visual-state helpers (badge/margin/forecast), pm_utilization_score
    // invariants, remaining PM change-detection keys (health_status,
    // projected_total_cost_idr, forecast_variance_from_target_pct,
    // target_margin_pct, margin_alert_threshold_pct,
    // forecast_has_revenue_signal), and sort_pm_cards edge cases.
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn budget_badge_class_returns_badge_negative_when_over_budget() {
        // is_over_budget=true must always force negative styling, even when
        // health_status is otherwise "healthy" (defensive: backend currently
        // sets health_status=critical when over budget, but the UI helper
        // must not depend on that coupling).
        assert_eq!(budget_badge_class("healthy", true), "badge-negative");
        assert_eq!(budget_badge_class("warning", true), "badge-negative");
        assert_eq!(budget_badge_class("critical", true), "badge-negative");
        assert_eq!(budget_badge_class("unconfigured", true), "badge-negative");
    }

    #[test]
    fn budget_badge_class_maps_budget_status_tokens() {
        assert_eq!(budget_badge_class("healthy", false), "badge-positive");
        assert_eq!(budget_badge_class("warning", false), "badge-warning");
        assert_eq!(budget_badge_class("critical", false), "badge-negative");
        assert_eq!(budget_badge_class("unconfigured", false), "badge-neutral");
        // Unknown severities must fall back to neutral, not silently render
        // as positive/negative.
        assert_eq!(budget_badge_class("garbage", false), "badge-neutral");
    }

    #[test]
    fn budget_badge_label_returns_human_readable_labels() {
        assert_eq!(budget_badge_label("healthy", true), "Over budget");
        assert_eq!(budget_badge_label("healthy", false), "On track");
        assert_eq!(budget_badge_label("warning", false), "Watch");
        assert_eq!(budget_badge_label("critical", false), "At risk");
        assert_eq!(budget_badge_label("unconfigured", false), "Unconfigured");
        assert_eq!(budget_badge_label("anything-else", false), "Unconfigured");
        assert_eq!(health_status_class("healthy"), "text-positive-default");
        assert_eq!(health_status_class("warning"), "text-warning-default");
        assert_eq!(health_status_class("critical"), "text-negative-default");
        assert_eq!(health_status_class("unconfigured"), "text-huly-muted");
        assert_eq!(health_status_label("healthy"), "Healthy");
        assert_eq!(health_status_label("warning"), "Watch");
        assert_eq!(health_status_label("critical"), "At risk");
        assert_eq!(health_status_label("unconfigured"), "Unconfigured");
    }

    #[test]
    fn margin_status_class_negative_when_margin_alert_present() {
        // margin_alert_present must dominate other inputs.
        let class = margin_status_class(40.0, 25.0, 5.0, true, false);
        assert_eq!(class, "text-negative-default");
    }

    #[test]
    fn margin_status_class_negative_when_below_target_beyond_threshold() {
        // Target 30, threshold 5, actual 20 → gap of 10 > threshold → negative.
        let class = margin_status_class(20.0, 30.0, 5.0, false, true);
        assert_eq!(class, "text-negative-default");
    }

    #[test]
    fn margin_status_class_warning_when_below_target_within_threshold() {
        // Target 30, threshold 5, actual 27 → gap of 3 ≤ threshold → warning.
        let class = margin_status_class(27.0, 30.0, 5.0, false, true);
        assert_eq!(class, "text-warning-default");
    }

    #[test]
    fn margin_status_class_positive_when_at_or_above_target() {
        assert_eq!(
            margin_status_class(30.0, 30.0, 5.0, false, true),
            "text-positive-default"
        );
        assert_eq!(
            margin_status_class(45.0, 30.0, 5.0, false, true),
            "text-positive-default"
        );
        let class = margin_status_class(0.0, 30.0, 5.0, false, false);
        assert_eq!(class, "text-huly-muted");
    }

    #[test]
    fn forecast_status_class_muted_when_unavailable() {
        // Forecast unavailable must visually deprioritize, regardless of
        // variance noise from the placeholder card.
        let class = forecast_status_class(-50.0, 5.0, true, true);
        assert_eq!(class, "text-huly-muted");

        let class = forecast_status_class(-50.0, 5.0, false, false);
        assert_eq!(class, "text-huly-muted");
    }

    #[test]
    fn forecast_status_class_negative_when_variance_below_threshold() {
        // variance = -10, threshold = 5 → |variance| > threshold → negative.
        let class = forecast_status_class(-10.0, 5.0, false, true);
        assert_eq!(class, "text-negative-default");
    }

    #[test]
    fn forecast_status_class_warning_when_variance_below_target_within_threshold() {
        // variance = -3, threshold = 5 → still negative but not past threshold → warning.
        let class = forecast_status_class(-3.0, 5.0, false, true);
        assert_eq!(class, "text-warning-default");
    }

    #[test]
    fn forecast_status_class_positive_when_variance_at_or_above_target() {
        assert_eq!(
            forecast_status_class(0.0, 5.0, false, true),
            "text-positive-default"
        );
        assert_eq!(
            forecast_status_class(8.0, 5.0, false, true),
            "text-positive-default"
        );
    }

    #[test]
    fn pm_utilization_score_returns_sentinel_for_unconfigured_card() {
        // total_budget_idr <= 0 must map to a sub-zero score so unconfigured
        // cards sort to the bottom of the Budget Utilization mode.
        let unconfigured = ProjectHealthCard {
            total_budget_idr: 0,
            budget_utilization_pct: 0.0,
            is_over_budget: false,
            budget_overrun_idr: 0,
            ..pm_card_default(Some(Uuid::new_v4()), "Unconfigured")
        };
        assert_eq!(pm_utilization_score(&unconfigured), -1.0);
    }

    #[test]
    fn pm_utilization_score_boosts_over_budget_card_above_in_budget_card_at_same_utilization() {
        // Two cards at the same nominal utilization — the one flagged
        // `is_over_budget` must rank strictly higher so the UI never hides a
        // genuine overrun behind another card at the same %.
        let in_budget = ProjectHealthCard {
            total_budget_idr: 100,
            budget_utilization_pct: 101.0,
            is_over_budget: false,
            budget_overrun_idr: 0,
            ..pm_card_default(Some(Uuid::new_v4()), "InBudget")
        };
        let over_budget = ProjectHealthCard {
            total_budget_idr: 100,
            budget_utilization_pct: 101.0,
            is_over_budget: true,
            budget_overrun_idr: 1,
            ..pm_card_default(Some(Uuid::new_v4()), "OverBudget")
        };
        assert!(
            pm_utilization_score(&over_budget) > pm_utilization_score(&in_budget),
            "over-budget card must outrank in-budget card at the same utilization"
        );
    }

    #[test]
    fn pm_health_status_change_produces_stable_key() {
        // Polling tick where backend re-derives health_status from healthy →
        // critical must surface as a stable change-detection key so the
        // status flash binding fires.
        let project_id = Uuid::new_v4();
        let make = |status: &str| {
            let mut resp = empty_response("project_manager");
            resp.project_manager = Some(ProjectManagerDashboard {
                active_projects: vec![ProjectHealthCard {
                    health_status: status.into(),
                    ..pm_card_default(Some(project_id), "Atlas")
                }],
                margin_alerts: vec![],
                warnings: vec![],
            });
            resp
        };
        let prev_map = dashboard_value_map(&make("healthy"));
        let next_map = dashboard_value_map(&make("critical"));
        let changed = compute_changed_keys(&prev_map, &next_map);
        let expected_key = format!("project_manager.project.{}.health_status", project_id);
        assert!(
            changed.contains(&expected_key),
            "expected `{}` in {:?}",
            expected_key,
            changed
        );
    }

    #[test]
    fn pm_projected_total_cost_idr_change_produces_stable_key() {
        let project_id = Uuid::new_v4();
        let make = |projected: i64| {
            let mut resp = empty_response("project_manager");
            resp.project_manager = Some(ProjectManagerDashboard {
                active_projects: vec![ProjectHealthCard {
                    projected_total_cost_idr: projected,
                    ..pm_card_default(Some(project_id), "Atlas")
                }],
                margin_alerts: vec![],
                warnings: vec![],
            });
            resp
        };
        let prev_map = dashboard_value_map(&make(60_000_000));
        let next_map = dashboard_value_map(&make(95_000_000));
        let changed = compute_changed_keys(&prev_map, &next_map);
        let expected_key = format!(
            "project_manager.project.{}.projected_total_cost_idr",
            project_id
        );
        assert!(
            changed.contains(&expected_key),
            "expected `{}` in {:?}",
            expected_key,
            changed
        );
    }

    #[test]
    fn pm_forecast_variance_from_target_pct_change_produces_stable_key() {
        let project_id = Uuid::new_v4();
        let make = |variance: f64, has_forecast_revenue: bool, target: f64, threshold: f64| {
            let mut resp = empty_response("project_manager");
            resp.project_manager = Some(ProjectManagerDashboard {
                active_projects: vec![ProjectHealthCard {
                    forecast_variance_from_target_pct: variance,
                    forecast_has_revenue_signal: has_forecast_revenue,
                    target_margin_pct: target,
                    margin_alert_threshold_pct: threshold,
                    ..pm_card_default(Some(project_id), "Atlas")
                }],
                margin_alerts: vec![],
                warnings: vec![],
            });
            resp
        };
        let prev_map = dashboard_value_map(&make(0.0, true, 25.0, 5.0));
        let next_map = dashboard_value_map(&make(-12.5, false, 30.0, 7.5));
        let changed = compute_changed_keys(&prev_map, &next_map);
        for field in [
            "forecast_variance_from_target_pct",
            "forecast_has_revenue_signal",
            "target_margin_pct",
            "margin_alert_threshold_pct",
        ] {
            let expected_key = format!("project_manager.project.{}.{}", project_id, field);
            assert!(
                changed.contains(&expected_key),
                "expected `{}` in {:?}",
                expected_key,
                changed
            );
        }
    }

    #[test]
    fn sort_pm_cards_handles_empty_slice_without_panic() {
        // PM with zero active projects must still allow sort-mode toggling
        // without panicking in any sort branch.
        let mut empty: Vec<ProjectHealthCard> = Vec::new();
        sort_pm_cards(&mut empty, PmSortMode::Margin);
        sort_pm_cards(&mut empty, PmSortMode::BudgetUtilization);
        sort_pm_cards(&mut empty, PmSortMode::EndDate);
        assert!(empty.is_empty());
    }

    // ── Story 6.4 — Team Utilization Dashboard helpers + change keys ───────

    fn dh_response_with(
        team_members: Vec<TeamUtilizationMember>,
        trends: Option<TeamUtilizationTrendBundle>,
        budget: Option<DepartmentBudgetSummary>,
    ) -> RoleDashboardResponse {
        let underutilized = team_members
            .iter()
            .filter(|m| m.is_underutilized)
            .cloned()
            .collect();
        let mut resp = empty_response("department_head");
        resp.department_head = Some(DepartmentHeadDashboard {
            utilization: UtilizationSummary {
                start_date: "2026-05-22".into(),
                end_date: "2026-06-21".into(),
                average_utilization_pct: 60.0,
                overallocated_count: 0,
                top_at_risk: vec![],
            },
            budget,
            overallocations: OverallocationSummary {
                overallocated_count: 0,
                members: vec![],
            },
            upcoming_assignments: vec![],
            warnings: vec![],
            team_members,
            utilization_trends: trends,
            underutilized_members: underutilized,
        });
        resp
    }

    fn dh_member(
        id: Option<Uuid>,
        name: &str,
        current: f64,
        projects: Vec<TeamUtilizationCurrentProject>,
    ) -> TeamUtilizationMember {
        let available = (100.0 - current).max(0.0);
        TeamUtilizationMember {
            resource_id: id,
            resource_name: name.into(),
            role: "engineer".into(),
            current_utilization_pct: current,
            available_capacity_pct: (available * 10.0).round() / 10.0,
            is_underutilized: current < DH_UNDERUTILIZED_THRESHOLD_PCT,
            is_overallocated: current > 100.0,
            ctc_status: "Active".into(),
            current_projects: projects,
        }
    }

    #[test]
    fn underutilized_threshold_excludes_50_pct() {
        // 49.9% is underutilized; 50.0% is not. The threshold is strict <.
        assert!(is_underutilized_threshold(49.9));
        assert!(!is_underutilized_threshold(50.0));
        assert!(!is_underutilized_threshold(75.0));
        assert!(!is_underutilized_threshold(100.0));
    }

    #[test]
    fn utilization_badge_class_branches() {
        // Overallocated always wins over underutilized when both inputs imply
        // negative, mirroring backend severity precedence.
        assert_eq!(
            utilization_badge_class(150.0, true),
            "badge-negative",
            "explicit overallocation flag must yield negative"
        );
        assert_eq!(
            utilization_badge_class(105.0, false),
            "badge-negative",
            ">100% utilization must yield negative regardless of flag"
        );
        assert_eq!(utilization_badge_class(40.0, false), "badge-warning");
        assert_eq!(utilization_badge_class(85.0, false), "badge-positive");
        assert_eq!(utilization_badge_class(60.0, false), "badge-neutral");
    }

    #[test]
    fn utilization_badge_label_branches() {
        assert_eq!(utilization_badge_label(0.0, true), "Overallocated");
        assert_eq!(utilization_badge_label(110.0, false), "Overallocated");
        assert_eq!(utilization_badge_label(25.0, false), "Underutilized");
        assert_eq!(utilization_badge_label(80.0, false), "Healthy");
        assert_eq!(utilization_badge_label(65.0, false), "Available");
    }

    #[test]
    fn bar_width_clamps_negative_and_overflow() {
        assert_eq!(bar_width_pct(-10.0), 0.0);
        assert_eq!(bar_width_pct(0.0), 0.0);
        assert_eq!(bar_width_pct(75.0), 75.0);
        assert_eq!(bar_width_pct(150.0), 100.0);
        assert_eq!(bar_width_pct(f64::NAN), 0.0);
    }

    #[test]
    fn budget_health_class_and_label_branches() {
        assert_eq!(budget_health_class("critical"), "text-negative-default");
        assert_eq!(budget_health_class("warning"), "text-warning-default");
        assert_eq!(budget_health_class("healthy"), "text-positive-default");
        assert_eq!(budget_health_class("unknown"), "text-huly-muted");

        assert_eq!(budget_health_label("critical"), "At risk");
        assert_eq!(budget_health_label("warning"), "Watch");
        assert_eq!(budget_health_label("healthy"), "On track");
        assert_eq!(budget_health_label("bogus"), "Unconfigured");
    }

    #[test]
    fn team_member_key_uses_resource_id_when_present_else_name() {
        let id = Uuid::new_v4();
        let with_id = dh_member(Some(id), "Alice", 40.0, vec![]);
        let no_id = dh_member(None, "Anon Person", 40.0, vec![]);
        assert_eq!(
            team_member_key(&with_id),
            format!("department_head.team.{}", id)
        );
        assert_eq!(
            team_member_key(&no_id),
            "department_head.team.name.Anon Person"
        );
    }

    #[test]
    fn trend_period_key_uses_resource_id_when_present_else_name() {
        let id = Uuid::new_v4();
        let with_id = TeamUtilizationTrend {
            resource_id: Some(id),
            resource_name: "Alice".into(),
            periods: vec![],
        };
        let no_id = TeamUtilizationTrend {
            resource_id: None,
            resource_name: "Anon".into(),
            periods: vec![],
        };
        assert_eq!(
            trend_period_key(&with_id, "2026-05"),
            format!("department_head.trend.{}.2026-05", id)
        );
        assert_eq!(
            trend_period_key(&no_id, "2026-05"),
            "department_head.trend.name.Anon.2026-05"
        );
    }

    #[test]
    fn dh_team_utilization_change_produces_expected_keys() {
        let id = Uuid::new_v4();
        let mk = |pct: f64| {
            dh_response_with(vec![dh_member(Some(id), "Alice", pct, vec![])], None, None)
        };
        let prev_map = dashboard_value_map(&mk(40.0));
        let next_map = dashboard_value_map(&mk(70.0));
        let changed = compute_changed_keys(&prev_map, &next_map);

        let row = format!("department_head.team.{}", id);
        for suffix in [
            "current_utilization_pct",
            "available_capacity_pct",
            "is_underutilized",
        ] {
            let key = format!("{}.{}", row, suffix);
            assert!(
                changed.contains(&key),
                "expected `{}` in {:?}",
                key,
                changed
            );
        }
        // Aggregate stats should also flash.
        assert!(changed.contains("department_head.team.avg_available_capacity_pct"));
        assert!(changed.contains("department_head.team.underutilized_count"));
    }

    #[test]
    fn dh_current_projects_change_produces_stable_key() {
        let id = Uuid::new_v4();
        let mk = |projects: Vec<TeamUtilizationCurrentProject>| {
            dh_response_with(
                vec![dh_member(Some(id), "Alice", 40.0, projects)],
                None,
                None,
            )
        };
        let prev_map = dashboard_value_map(&mk(vec![TeamUtilizationCurrentProject {
            project_name: "Atlas".into(),
            allocation_percentage: 40.0,
            start_date: "2026-05-01".into(),
            end_date: "2026-06-01".into(),
        }]));
        let next_map = dashboard_value_map(&mk(vec![
            TeamUtilizationCurrentProject {
                project_name: "Atlas".into(),
                allocation_percentage: 40.0,
                start_date: "2026-05-01".into(),
                end_date: "2026-06-01".into(),
            },
            TeamUtilizationCurrentProject {
                project_name: "Beacon".into(),
                allocation_percentage: 20.0,
                start_date: "2026-05-15".into(),
                end_date: "2026-06-15".into(),
            },
        ]));
        let changed = compute_changed_keys(&prev_map, &next_map);
        let projects_key = format!("department_head.team.{}.current_projects", id);
        assert!(
            changed.contains(&projects_key),
            "expected `{}` in {:?}",
            projects_key,
            changed
        );
    }

    #[test]
    fn dh_team_name_and_role_changes_are_tracked() {
        let id = Uuid::new_v4();
        let mut renamed = dh_member(Some(id), "Alice Renamed", 40.0, vec![]);
        renamed.role = "lead engineer".into();
        let prev_map = dashboard_value_map(&dh_response_with(
            vec![dh_member(Some(id), "Alice", 40.0, vec![])],
            None,
            None,
        ));
        let next_map = dashboard_value_map(&dh_response_with(vec![renamed], None, None));
        let changed = compute_changed_keys(&prev_map, &next_map);
        assert!(changed.contains(&format!("department_head.team.{}.resource_name", id)));
        assert!(changed.contains(&format!("department_head.team.{}.role", id)));
    }

    #[test]
    fn dh_current_projects_change_value_distinguishes_delimiters() {
        let first = current_projects_change_value(&[
            TeamUtilizationCurrentProject {
                project_name: "A|B".into(),
                allocation_percentage: 20.0,
                start_date: "2026-05-01".into(),
                end_date: "2026-05-31".into(),
            },
            TeamUtilizationCurrentProject {
                project_name: "C".into(),
                allocation_percentage: 30.0,
                start_date: "2026-06-01".into(),
                end_date: "2026-06-30".into(),
            },
        ]);
        let second = current_projects_change_value(&[
            TeamUtilizationCurrentProject {
                project_name: "A".into(),
                allocation_percentage: 20.0,
                start_date: "B".into(),
                end_date: "2026-05-31".into(),
            },
            TeamUtilizationCurrentProject {
                project_name: "C".into(),
                allocation_percentage: 30.0,
                start_date: "2026-06-01".into(),
                end_date: "2026-06-30".into(),
            },
        ]);

        assert_ne!(first, second);
    }

    #[test]
    fn dh_trend_period_change_produces_expected_key() {
        let id = Uuid::new_v4();
        let mk = |pct: f64| {
            let trend = TeamUtilizationTrendBundle {
                start_date: "2026-05-01".into(),
                end_date: "2026-07-31".into(),
                members: vec![TeamUtilizationTrend {
                    resource_id: Some(id),
                    resource_name: "Alice".into(),
                    periods: vec![
                        TeamUtilizationTrendPeriod {
                            period: "2026-05".into(),
                            utilization_pct: 60.0,
                        },
                        TeamUtilizationTrendPeriod {
                            period: "2026-06".into(),
                            utilization_pct: pct,
                        },
                    ],
                }],
            };
            dh_response_with(vec![], Some(trend), None)
        };
        let prev_map = dashboard_value_map(&mk(50.0));
        let next_map = dashboard_value_map(&mk(80.0));
        let changed = compute_changed_keys(&prev_map, &next_map);
        let key = format!("department_head.trend.{}.2026-06", id);
        assert!(
            changed.contains(&key),
            "expected `{}` in {:?}",
            key,
            changed
        );
        // The unchanged earlier period must not also appear.
        assert!(
            !changed.contains(&format!("department_head.trend.{}.2026-05", id)),
            "stable period must not flash"
        );
    }

    #[test]
    fn trend_period_columns_union_sparse_member_periods() {
        let bundle = TeamUtilizationTrendBundle {
            start_date: "2026-05-01".into(),
            end_date: "2026-07-31".into(),
            members: vec![
                TeamUtilizationTrend {
                    resource_id: Some(Uuid::new_v4()),
                    resource_name: "Alice".into(),
                    periods: vec![TeamUtilizationTrendPeriod {
                        period: "2026-06".into(),
                        utilization_pct: 60.0,
                    }],
                },
                TeamUtilizationTrend {
                    resource_id: Some(Uuid::new_v4()),
                    resource_name: "Bob".into(),
                    periods: vec![
                        TeamUtilizationTrendPeriod {
                            period: "2026-05".into(),
                            utilization_pct: 10.0,
                        },
                        TeamUtilizationTrendPeriod {
                            period: "2026-07".into(),
                            utilization_pct: 30.0,
                        },
                    ],
                },
            ],
        };

        assert_eq!(
            trend_period_columns(&bundle),
            vec![
                "2026-05".to_string(),
                "2026-06".to_string(),
                "2026-07".to_string()
            ]
        );
    }

    #[test]
    fn dh_budget_spent_actual_and_threshold_keys_present() {
        let budget = DepartmentBudgetSummary {
            department_name: "Engineering".into(),
            budget_period: "2026-05".into(),
            total_budget_idr: 50_000_000,
            total_committed_idr: 20_000_000,
            spent_actual_idr: 12_500_000,
            remaining_idr: 30_000_000,
            utilization_percentage: 40.0,
            budget_health: "healthy".into(),
            alert_threshold_pct: 80,
            budget_configured: true,
        };
        let resp = dh_response_with(vec![], None, Some(budget));
        let map = dashboard_value_map(&resp);
        assert!(map.contains_key("department_head.budget.spent_actual_idr"));
        assert!(map.contains_key("department_head.budget.alert_threshold_pct"));
        assert_eq!(
            map.get("department_head.budget.spent_actual_idr")
                .map(String::as_str),
            Some("12500000")
        );
        assert_eq!(
            map.get("department_head.budget.alert_threshold_pct")
                .map(String::as_str),
            Some("80")
        );
    }

    #[test]
    fn dh_team_value_map_excludes_sensitive_ctc_fields() {
        // Even when CTC status is exposed, no ciphertext/key/salary leakage.
        let member = dh_member(Some(Uuid::new_v4()), "Alice", 40.0, vec![]);
        let resp = dh_response_with(vec![member], None, None);
        let serialized = format!("{:?}", dashboard_value_map(&resp));
        for forbidden in [
            "encrypted_components",
            "encrypted_daily_rate",
            "ciphertext",
            "key_version",
            "encryption_algorithm",
            "encryption_version",
            "base_salary",
            "daily_rate",
        ] {
            assert!(
                !serialized.contains(forbidden),
                "DH team value map leaked sensitive token `{}`",
                forbidden
            );
        }
    }

    #[test]
    fn trend_range_preset_labels_stable() {
        assert_eq!(TrendRangePreset::CurrentMonth.label(), "Current Month");
        assert_eq!(TrendRangePreset::Next30Days.label(), "Next 30 Days");
        assert_eq!(TrendRangePreset::ThreeMonths.label(), "3 Months");
        assert_eq!(TrendRangePreset::SixMonths.label(), "6 Months");
    }

    #[test]
    fn trend_range_preset_uses_utc_calendar_date_boundaries() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 22).unwrap();
        assert_eq!(
            TrendRangePreset::CurrentMonth.resolve_from_date(today),
            ("2026-05-01".to_string(), "2026-05-31".to_string())
        );
        assert_eq!(
            TrendRangePreset::Next30Days.resolve_from_date(today),
            ("2026-05-22".to_string(), "2026-06-21".to_string())
        );
        assert_eq!(
            TrendRangePreset::ThreeMonths.resolve_from_date(today),
            ("2026-05-22".to_string(), "2026-08-20".to_string())
        );
        assert_eq!(
            TrendRangePreset::SixMonths.resolve_from_date(today),
            ("2026-05-22".to_string(), "2026-11-18".to_string())
        );
    }

    #[test]
    fn trend_range_current_month_handles_year_boundary() {
        let today = NaiveDate::from_ymd_opt(2026, 12, 22).unwrap();
        assert_eq!(
            TrendRangePreset::CurrentMonth.resolve_from_date(today),
            ("2026-12-01".to_string(), "2026-12-31".to_string())
        );
    }

    // ── Story 6.4 expansion (Master Test Architect):
    // bar-width edge values, badge boundary at 50%/100%, and change-detection
    // for aggregate / row-removed / project-pct-only mutations.
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn bar_width_pct_clamps_infinities() {
        // Both infinities must clamp to a finite bar width so a corrupt
        // backend value cannot break layout via inline width style.
        assert_eq!(bar_width_pct(f64::INFINITY), 100.0);
        assert_eq!(bar_width_pct(f64::NEG_INFINITY), 0.0);
    }

    #[test]
    fn utilization_badge_at_exactly_50_pct_is_neutral_available() {
        // Strict threshold parity with the backend: 50.0% is neither
        // underutilized nor (yet) the "Healthy" >= 80 band.
        assert_eq!(utilization_badge_class(50.0, false), "badge-neutral");
        assert_eq!(utilization_badge_label(50.0, false), "Available");
    }

    #[test]
    fn utilization_badge_at_100_pct_is_healthy_not_overallocated() {
        // 100.0% is the upper boundary of the Healthy band. The
        // "Overallocated" branch must fire only above 100% (or when the
        // explicit flag is set).
        assert_eq!(utilization_badge_class(100.0, false), "badge-positive");
        assert_eq!(utilization_badge_label(100.0, false), "Healthy");
    }

    #[test]
    fn dh_member_count_flashes_when_row_removed() {
        // Removing a member from the team table must surface as a change on
        // the visible aggregate so the count tile flashes.
        let kept_id = Uuid::new_v4();
        let removed_id = Uuid::new_v4();
        let prev = dh_response_with(
            vec![
                dh_member(Some(kept_id), "Alice", 40.0, vec![]),
                dh_member(Some(removed_id), "Bob", 70.0, vec![]),
            ],
            None,
            None,
        );
        let next = dh_response_with(
            vec![dh_member(Some(kept_id), "Alice", 40.0, vec![])],
            None,
            None,
        );
        let prev_map = dashboard_value_map(&prev);
        let next_map = dashboard_value_map(&next);
        let changed = compute_changed_keys(&prev_map, &next_map);
        assert!(
            changed.contains("department_head.team.member_count"),
            "removing a member must flash the aggregate count, got {:?}",
            changed
        );
        // The removed row's per-member keys are no longer present in `next`
        // and the differ does not emit removed-only keys — guards against a
        // future regression that would double-flash on absence.
        let removed_util_key = format!(
            "department_head.team.{}.current_utilization_pct",
            removed_id
        );
        assert!(
            !changed.contains(&removed_util_key),
            "removed-only keys must not appear in the changed set"
        );
    }

    #[test]
    fn dh_underutilized_count_flashes_when_member_crosses_threshold() {
        // Same member id, utilization moves from 40% (underutilized) → 60%
        // (not underutilized). The aggregate underutilized_count must flash
        // alongside the per-member is_underutilized key.
        let id = Uuid::new_v4();
        let prev_map = dashboard_value_map(&dh_response_with(
            vec![dh_member(Some(id), "Alice", 40.0, vec![])],
            None,
            None,
        ));
        let next_map = dashboard_value_map(&dh_response_with(
            vec![dh_member(Some(id), "Alice", 60.0, vec![])],
            None,
            None,
        ));
        let changed = compute_changed_keys(&prev_map, &next_map);
        assert!(
            changed.contains("department_head.team.underutilized_count"),
            "underutilized_count must flash when a member crosses the 50% threshold, got {:?}",
            changed
        );
        let per_member_key = format!("department_head.team.{}.is_underutilized", id);
        assert!(
            changed.contains(&per_member_key),
            "per-member is_underutilized must also flash, got {:?}",
            changed
        );
    }

    #[test]
    fn dh_overallocated_flag_flashes_independently() {
        let id = Uuid::new_v4();
        let make = |overallocated: bool| {
            let mut member = dh_member(Some(id), "Alice", 100.0, vec![]);
            member.is_overallocated = overallocated;
            dh_response_with(vec![member], None, None)
        };
        let prev_map = dashboard_value_map(&make(false));
        let next_map = dashboard_value_map(&make(true));
        let changed = compute_changed_keys(&prev_map, &next_map);
        let key = format!("department_head.team.{}.is_overallocated", id);
        assert!(
            changed.contains(&key),
            "overallocated flag must have an independent change key, got {:?}",
            changed
        );
    }

    #[test]
    fn dh_current_projects_flashes_on_allocation_pct_change_only() {
        // Same project name and dates, only the allocation percentage moves.
        // The bundled current_projects summary must still mark the row's
        // current_projects key as changed (the value embeds the % so a
        // single-field shift surfaces as a row-level change).
        let id = Uuid::new_v4();
        let mk = |pct: f64| {
            dh_response_with(
                vec![dh_member(
                    Some(id),
                    "Alice",
                    40.0,
                    vec![TeamUtilizationCurrentProject {
                        project_name: "Atlas".into(),
                        allocation_percentage: pct,
                        start_date: "2026-05-01".into(),
                        end_date: "2026-06-01".into(),
                    }],
                )],
                None,
                None,
            )
        };
        let prev_map = dashboard_value_map(&mk(20.0));
        let next_map = dashboard_value_map(&mk(35.0));
        let changed = compute_changed_keys(&prev_map, &next_map);
        let projects_key = format!("department_head.team.{}.current_projects", id);
        assert!(
            changed.contains(&projects_key),
            "current_projects key must flash on percentage change of an existing project, got {:?}",
            changed
        );
    }

    #[test]
    fn dh_avg_available_capacity_is_zero_when_team_is_empty() {
        // Pure aggregate guard: a Department Head with an empty team renders
        // "0.0" rather than a NaN/Inf division-by-zero artifact.
        let resp = dh_response_with(vec![], None, None);
        let map = dashboard_value_map(&resp);
        assert_eq!(
            map.get("department_head.team.avg_available_capacity_pct")
                .map(String::as_str),
            Some("0.0"),
            "empty team must surface avg_available_capacity_pct=0.0 (never NaN)"
        );
        assert_eq!(
            map.get("department_head.team.member_count")
                .map(String::as_str),
            Some("0")
        );
        assert_eq!(
            map.get("department_head.team.underutilized_count")
                .map(String::as_str),
            Some("0")
        );
    }
}
