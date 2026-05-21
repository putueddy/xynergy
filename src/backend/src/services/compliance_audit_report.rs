//! Service for Story 5.4 compliance audit reports.
//!
//! Produces report rows for four report types backed by existing audit and
//! revision tables (no parallel audit store):
//! - `ctc_change_log`: sourced from `ctc_revisions` (decrypted server-side).
//! - `assignment_history`: sourced from `audit_logs` where `entity_type = 'allocation'`.
//! - `budget_modifications`: sourced from `audit_logs` where `entity_type = 'project_budget'`.
//! - `access_logs`: sourced from `audit_logs` for login/view/access events.

use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Row};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::ctc_crypto::{CtcCryptoService, DefaultCtcCryptoService, EncryptedPayload};
use crate::services::key_provider::EnvKeyProvider;

pub const REPORT_LIMIT_MAX: i64 = 200;
pub const REPORT_LIMIT_DEFAULT: i64 = 50;
pub const REPORT_OFFSET_MAX: i64 = 100_000;
// Caps the interactive (synchronous) window so admins/finance cannot trigger
// minutes-long CTC decrypts; wider windows must go through the export workflow.
pub const REPORT_MAX_RANGE_DAYS: i64 = 90;
const CTC_SYNTHETIC_BASELINE_REASON: &str = "Baseline snapshot before first revision update";
const REPORT_TIMEZONE_LABEL: &str = "Asia/Jakarta";
const JAKARTA_UTC_OFFSET_SECONDS: i32 = 7 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditReportType {
    CtcChangeLog,
    AssignmentHistory,
    BudgetModifications,
    AccessLogs,
}

impl AuditReportType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditReportType::CtcChangeLog => "ctc_change_log",
            AuditReportType::AssignmentHistory => "assignment_history",
            AuditReportType::BudgetModifications => "budget_modifications",
            AuditReportType::AccessLogs => "access_logs",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "ctc_change_log" => Ok(Self::CtcChangeLog),
            "assignment_history" => Ok(Self::AssignmentHistory),
            "budget_modifications" => Ok(Self::BudgetModifications),
            "access_logs" => Ok(Self::AccessLogs),
            other => Err(AppError::Validation(format!(
                "Unsupported report_type '{}'; expected one of ctc_change_log, assignment_history, budget_modifications, access_logs",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReportFilters {
    pub report_type: AuditReportType,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub limit: i64,
    pub offset: i64,
    pub snapshot_at: DateTime<Utc>,
    pub user_id: Option<Uuid>,
    pub action_type: Option<String>,
}

pub fn clamp_limit(limit: Option<i64>) -> i64 {
    limit
        .unwrap_or(REPORT_LIMIT_DEFAULT)
        .max(1)
        .min(REPORT_LIMIT_MAX)
}

pub fn clamp_offset(offset: Option<i64>) -> i64 {
    offset.unwrap_or(0).max(0).min(REPORT_OFFSET_MAX)
}

pub fn validate_date_order(start: NaiveDate, end: NaiveDate) -> Result<()> {
    if end < start {
        return Err(AppError::Validation(
            "end_date must be greater than or equal to start_date".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_date_range(start: NaiveDate, end: NaiveDate) -> Result<()> {
    validate_date_order(start, end)?;

    let inclusive_days = (end - start).num_days() + 1;
    if inclusive_days > REPORT_MAX_RANGE_DAYS {
        return Err(AppError::Validation(format!(
            "date range {} days exceeds maximum {} days; narrow the window or use the export workflow",
            inclusive_days, REPORT_MAX_RANGE_DAYS,
        )));
    }
    Ok(())
}

/// Common envelope wrapping per-report rows for paginated responses.
#[derive(Debug, Serialize)]
pub struct ComplianceAuditReport {
    pub report_type: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub snapshot_at: DateTime<Utc>,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
    pub rows: ReportRows,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ReportRows {
    CtcChangeLog(Vec<CtcChangeLogRow>),
    AssignmentHistory(Vec<AssignmentHistoryRow>),
    BudgetModifications(Vec<BudgetModificationRow>),
    AccessLogs(Vec<AccessLogRow>),
}

impl ReportRows {
    pub fn len(&self) -> usize {
        match self {
            ReportRows::CtcChangeLog(rows) => rows.len(),
            ReportRows::AssignmentHistory(rows) => rows.len(),
            ReportRows::BudgetModifications(rows) => rows.len(),
            ReportRows::AccessLogs(rows) => rows.len(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CtcChangeLogRow {
    pub revision_id: Uuid,
    pub revision_number: i32,
    pub change_date: DateTime<Utc>,
    pub employee_id: Uuid,
    pub employee_name: String,
    pub changed_by_id: Uuid,
    pub changed_by_name: Option<String>,
    pub field: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct AssignmentHistoryRow {
    pub audit_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<Uuid>,
    pub user_name: Option<String>,
    pub action: String,
    pub allocation_id: Option<Uuid>,
    pub resource_id: Option<Uuid>,
    pub resource_name: Option<String>,
    pub project_id: Option<Uuid>,
    pub project_name: Option<String>,
    pub before_summary: String,
    pub after_summary: String,
}

#[derive(Debug, Serialize)]
pub struct BudgetModificationRow {
    pub audit_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<Uuid>,
    pub user_name: Option<String>,
    pub action: String,
    pub project_id: Option<Uuid>,
    pub project_name: Option<String>,
    pub before_summary: String,
    pub after_summary: String,
}

#[derive(Debug, Serialize)]
pub struct AccessLogRow {
    pub audit_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<Uuid>,
    pub user_name: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub success: bool,
    pub reason: Option<String>,
}

fn full_name(first: Option<String>, last: Option<String>) -> Option<String> {
    let combined = format!("{} {}", first.unwrap_or_default(), last.unwrap_or_default());
    let trimmed = combined.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn report_timezone() -> Result<FixedOffset> {
    FixedOffset::east_opt(JAKARTA_UTC_OFFSET_SECONDS).ok_or_else(|| {
        AppError::Internal(format!(
            "{} report timezone offset is invalid",
            REPORT_TIMEZONE_LABEL
        ))
    })
}

fn start_of_day(date: NaiveDate) -> Result<DateTime<Utc>> {
    let local_midnight = date.and_hms_opt(0, 0, 0).ok_or_else(|| {
        AppError::Validation(format!(
            "start_date cannot be represented in {}",
            REPORT_TIMEZONE_LABEL
        ))
    })?;
    report_timezone()?
        .from_local_datetime(&local_midnight)
        .single()
        .map(|dt| dt.with_timezone(&Utc))
        .ok_or_else(|| {
            AppError::Validation(format!(
                "start_date cannot be represented in {}",
                REPORT_TIMEZONE_LABEL
            ))
        })
}

fn exclusive_end(date: NaiveDate) -> Result<DateTime<Utc>> {
    let next_day = date
        .succ_opt()
        .ok_or_else(|| AppError::Validation("end_date is too large".to_string()))?;
    start_of_day(next_day)
}

pub async fn generate_report(
    pool: &PgPool,
    filters: ReportFilters,
) -> Result<ComplianceAuditReport> {
    validate_date_range(filters.start_date, filters.end_date)?;

    let rows = match filters.report_type {
        AuditReportType::CtcChangeLog => {
            ReportRows::CtcChangeLog(build_ctc_change_log(pool, &filters).await?)
        }
        AuditReportType::AssignmentHistory => {
            ReportRows::AssignmentHistory(build_assignment_history(pool, &filters).await?)
        }
        AuditReportType::BudgetModifications => {
            ReportRows::BudgetModifications(build_budget_modifications(pool, &filters).await?)
        }
        AuditReportType::AccessLogs => {
            ReportRows::AccessLogs(build_access_logs(pool, &filters).await?)
        }
    };

    // We fetch limit + 1 in each builder; compute has_more here.
    let has_more = rows.len() as i64 > filters.limit;
    let rows = trim_to_limit(rows, filters.limit);

    Ok(ComplianceAuditReport {
        report_type: filters.report_type.as_str().to_string(),
        start_date: filters.start_date,
        end_date: filters.end_date,
        snapshot_at: filters.snapshot_at,
        limit: filters.limit,
        offset: filters.offset,
        has_more,
        rows,
    })
}

fn trim_to_limit(rows: ReportRows, limit: i64) -> ReportRows {
    let limit = limit as usize;
    match rows {
        ReportRows::CtcChangeLog(mut r) => {
            if r.len() > limit {
                r.truncate(limit);
            }
            ReportRows::CtcChangeLog(r)
        }
        ReportRows::AssignmentHistory(mut r) => {
            if r.len() > limit {
                r.truncate(limit);
            }
            ReportRows::AssignmentHistory(r)
        }
        ReportRows::BudgetModifications(mut r) => {
            if r.len() > limit {
                r.truncate(limit);
            }
            ReportRows::BudgetModifications(r)
        }
        ReportRows::AccessLogs(mut r) => {
            if r.len() > limit {
                r.truncate(limit);
            }
            ReportRows::AccessLogs(r)
        }
    }
}

// ---------------- CTC Change Log ----------------

/// Per-revision raw row used to assemble change-log diffs.
struct CtcRevisionRow {
    revision_id: Uuid,
    resource_id: Uuid,
    employee_name: String,
    revision_number: i32,
    created_at: DateTime<Utc>,
    changed_by_id: Uuid,
    changed_by_name: Option<String>,
    reason: String,
    encrypted_components: String,
    encrypted_daily_rate: Option<String>,
    key_version: String,
    encryption_version: String,
    encryption_algorithm: String,
    encrypted_at: DateTime<Utc>,
}

async fn build_ctc_change_log(
    pool: &PgPool,
    filters: &ReportFilters,
) -> Result<Vec<CtcChangeLogRow>> {
    // Pull all revisions in window plus the immediately previous revision per
    // resource so the diff for the first in-window revision has a real baseline.
    let window_start = start_of_day(filters.start_date)?;
    let window_end = exclusive_end(filters.end_date)?;
    // Pagination is applied after building field-level diff rows. Limiting the
    // revision query first can undercount when one revision emits many fields,
    // or when synthetic baseline rows emit none.
    let revisions_in_window = sqlx::query(
        r#"
        SELECT
            r.id AS revision_id,
            r.resource_id AS resource_id,
            res.name AS employee_name,
            r.revision_number AS revision_number,
            r.created_at AS created_at,
            r.changed_by AS changed_by,
            u.first_name AS first_name,
            u.last_name AS last_name,
            r.reason AS reason,
            r.encrypted_components AS encrypted_components,
            r.encrypted_daily_rate AS encrypted_daily_rate,
            r.key_version AS key_version,
            r.encryption_version AS encryption_version,
            r.encryption_algorithm AS encryption_algorithm,
            r.encrypted_at AS encrypted_at
        FROM ctc_revisions r
        LEFT JOIN resources res ON res.id = r.resource_id
        LEFT JOIN users u ON u.id = r.changed_by
        WHERE r.created_at >= $1 AND r.created_at < $2
          AND r.created_at <= $3
        ORDER BY r.created_at DESC, r.id DESC
        "#,
    )
    .bind(window_start)
    .bind(window_end)
    .bind(filters.snapshot_at)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let crypto = DefaultCtcCryptoService::new(EnvKeyProvider::new());

    // Build collection plus index of (resource_id, revision_number)
    let mut window_rows: Vec<CtcRevisionRow> = Vec::with_capacity(revisions_in_window.len());
    let mut resource_min_rev: HashMap<Uuid, i32> = HashMap::new();

    for row in revisions_in_window {
        let resource_id: Uuid = row.try_get("resource_id").map_err(db_err)?;
        let revision_number: i32 = row.try_get("revision_number").map_err(db_err)?;
        resource_min_rev
            .entry(resource_id)
            .and_modify(|v| {
                if revision_number < *v {
                    *v = revision_number;
                }
            })
            .or_insert(revision_number);

        let first_name: Option<String> = row.try_get("first_name").map_err(db_err)?;
        let last_name: Option<String> = row.try_get("last_name").map_err(db_err)?;
        window_rows.push(CtcRevisionRow {
            revision_id: row.try_get("revision_id").map_err(db_err)?,
            resource_id,
            employee_name: row
                .try_get::<Option<String>, _>("employee_name")
                .map_err(db_err)?
                .unwrap_or_else(|| "Unknown".to_string()),
            revision_number,
            created_at: row.try_get("created_at").map_err(db_err)?,
            changed_by_id: row.try_get("changed_by").map_err(db_err)?,
            changed_by_name: full_name(first_name, last_name),
            reason: row.try_get("reason").map_err(db_err)?,
            encrypted_components: row.try_get("encrypted_components").map_err(db_err)?,
            encrypted_daily_rate: row.try_get("encrypted_daily_rate").map_err(db_err)?,
            key_version: row.try_get("key_version").map_err(db_err)?,
            encryption_version: row.try_get("encryption_version").map_err(db_err)?,
            encryption_algorithm: row.try_get("encryption_algorithm").map_err(db_err)?,
            encrypted_at: row.try_get("encrypted_at").map_err(db_err)?,
        });
    }

    // For each resource fetch the revision immediately before the earliest
    // in-window revision, so we can diff against actual previous values.
    let mut baseline_by_resource: HashMap<Uuid, CtcRevisionRow> = HashMap::new();
    for (resource_id, min_rev) in &resource_min_rev {
        if *min_rev <= 1 {
            continue;
        }
        if let Some(row) = sqlx::query(
            r#"
            SELECT
                r.id AS revision_id,
                r.resource_id AS resource_id,
                res.name AS employee_name,
                r.revision_number AS revision_number,
                r.created_at AS created_at,
                r.changed_by AS changed_by,
                u.first_name AS first_name,
                u.last_name AS last_name,
                r.reason AS reason,
                r.encrypted_components AS encrypted_components,
                r.encrypted_daily_rate AS encrypted_daily_rate,
                r.key_version AS key_version,
                r.encryption_version AS encryption_version,
                r.encryption_algorithm AS encryption_algorithm,
                r.encrypted_at AS encrypted_at
            FROM ctc_revisions r
            LEFT JOIN resources res ON res.id = r.resource_id
            LEFT JOIN users u ON u.id = r.changed_by
            WHERE r.resource_id = $1
              AND r.revision_number < $2
              AND r.created_at <= $3
            ORDER BY r.revision_number DESC, r.created_at DESC, r.id DESC
            LIMIT 1
            "#,
        )
        .bind(resource_id)
        .bind(min_rev)
        .bind(filters.snapshot_at)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        {
            let first_name: Option<String> = row.try_get("first_name").map_err(db_err)?;
            let last_name: Option<String> = row.try_get("last_name").map_err(db_err)?;
            baseline_by_resource.insert(
                *resource_id,
                CtcRevisionRow {
                    revision_id: row.try_get("revision_id").map_err(db_err)?,
                    resource_id: *resource_id,
                    employee_name: row
                        .try_get::<Option<String>, _>("employee_name")
                        .map_err(db_err)?
                        .unwrap_or_else(|| "Unknown".to_string()),
                    revision_number: row.try_get("revision_number").map_err(db_err)?,
                    created_at: row.try_get("created_at").map_err(db_err)?,
                    changed_by_id: row.try_get("changed_by").map_err(db_err)?,
                    changed_by_name: full_name(first_name, last_name),
                    reason: row.try_get("reason").map_err(db_err)?,
                    encrypted_components: row.try_get("encrypted_components").map_err(db_err)?,
                    encrypted_daily_rate: row.try_get("encrypted_daily_rate").map_err(db_err)?,
                    key_version: row.try_get("key_version").map_err(db_err)?,
                    encryption_version: row.try_get("encryption_version").map_err(db_err)?,
                    encryption_algorithm: row.try_get("encryption_algorithm").map_err(db_err)?,
                    encrypted_at: row.try_get("encrypted_at").map_err(db_err)?,
                },
            );
        }
    }

    // Sort window rows ascending by (resource_id, revision_number) so we can
    // walk previous → current diffs while keeping deterministic ordering.
    window_rows.sort_by(|a, b| {
        a.resource_id
            .cmp(&b.resource_id)
            .then_with(|| a.revision_number.cmp(&b.revision_number))
    });

    let mut state_by_resource: HashMap<Uuid, serde_json::Value> = HashMap::new();
    let mut diffs: Vec<CtcChangeLogRow> = Vec::new();
    let synthetic_baseline_ids = synthetic_baseline_revision_ids(&window_rows);
    for row in &window_rows {
        // Initialize previous state from baseline if not already set.
        if !state_by_resource.contains_key(&row.resource_id) {
            if let Some(baseline) = baseline_by_resource.get(&row.resource_id) {
                let baseline_state = decrypt_revision_state(&crypto, baseline).await?;
                state_by_resource.insert(row.resource_id, baseline_state);
            }
        }

        let current_state = decrypt_revision_state(&crypto, row).await?;
        if synthetic_baseline_ids.contains(&row.revision_id) {
            state_by_resource.insert(row.resource_id, current_state);
            continue;
        }

        let previous_state = state_by_resource.get(&row.resource_id);
        if previous_state.map(is_redacted_state).unwrap_or(false)
            || is_redacted_state(&current_state)
        {
            diffs.push(redacted_ctc_row(row));
            state_by_resource.insert(row.resource_id, current_state);
            continue;
        }

        let diff_fields = diff_fields(previous_state, &current_state);
        for (field, old_value, new_value) in diff_fields {
            diffs.push(CtcChangeLogRow {
                revision_id: row.revision_id,
                revision_number: row.revision_number,
                change_date: row.created_at,
                employee_id: row.resource_id,
                employee_name: row.employee_name.clone(),
                changed_by_id: row.changed_by_id,
                changed_by_name: row.changed_by_name.clone(),
                field,
                old_value,
                new_value,
                reason: row.reason.clone(),
            });
        }

        state_by_resource.insert(row.resource_id, current_state);
    }

    // Final ordering: newest revision first, then deterministic id, then field name.
    diffs.sort_by(|a, b| {
        b.change_date
            .cmp(&a.change_date)
            .then_with(|| b.revision_id.cmp(&a.revision_id))
            .then_with(|| a.field.cmp(&b.field))
    });

    apply_pagination(diffs, filters.offset, filters.limit + 1)
}

async fn decrypt_revision_state(
    crypto: &DefaultCtcCryptoService<EnvKeyProvider>,
    row: &CtcRevisionRow,
) -> Result<serde_json::Value> {
    let payload = EncryptedPayload {
        ciphertext: row.encrypted_components.clone(),
        key_version: row.key_version.clone(),
        encryption_version: row.encryption_version.clone(),
        algorithm: row.encryption_algorithm.clone(),
        encrypted_at: row.encrypted_at,
    };

    match crypto.decrypt_components(&payload).await {
        Ok(mut state) => {
            if let Some(daily) = &row.encrypted_daily_rate {
                let daily_payload = EncryptedPayload {
                    ciphertext: daily.clone(),
                    key_version: row.key_version.clone(),
                    encryption_version: row.encryption_version.clone(),
                    algorithm: row.encryption_algorithm.clone(),
                    encrypted_at: row.encrypted_at,
                };
                let daily_state = match crypto.decrypt_components(&daily_payload).await {
                    Ok(daily_state) => daily_state,
                    Err(_) => return Ok(redacted_state()),
                };
                if let (Some(obj), Some(rate)) =
                    (state.as_object_mut(), daily_state.get("daily_rate"))
                {
                    obj.insert("daily_rate".to_string(), rate.clone());
                }
            }
            Ok(state)
        }
        Err(_) => {
            // Surface a deterministic redacted row instead of failing the report.
            Ok(redacted_state())
        }
    }
}

fn redacted_state() -> serde_json::Value {
    serde_json::json!({
        "_redacted": true,
        "_reason": "decryption_failed"
    })
}

fn redacted_ctc_row(row: &CtcRevisionRow) -> CtcChangeLogRow {
    CtcChangeLogRow {
        revision_id: row.revision_id,
        revision_number: row.revision_number,
        change_date: row.created_at,
        employee_id: row.resource_id,
        employee_name: row.employee_name.clone(),
        changed_by_id: row.changed_by_id,
        changed_by_name: row.changed_by_name.clone(),
        field: "ctc_components".to_string(),
        old_value: serde_json::json!({
            "redacted": true,
            "reason": "decryption_failed"
        }),
        new_value: serde_json::json!({
            "redacted": true,
            "reason": "decryption_failed"
        }),
        reason: row.reason.clone(),
    }
}

fn is_redacted_state(value: &serde_json::Value) -> bool {
    value
        .get("_redacted")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn diff_fields(
    previous: Option<&serde_json::Value>,
    current: &serde_json::Value,
) -> Vec<(String, serde_json::Value, serde_json::Value)> {
    let mut out = Vec::new();
    let current_obj = current.as_object().cloned().unwrap_or_default();

    let prev_obj = previous
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    // Filter internal / sensitive keys.
    let is_internal = |k: &str| k.starts_with('_');

    for (k, v) in &current_obj {
        if is_internal(k) {
            continue;
        }
        let prev_v = prev_obj.get(k).cloned().unwrap_or(serde_json::Value::Null);
        if *v != prev_v {
            out.push((k.clone(), prev_v, v.clone()));
        }
    }

    // Removed fields.
    for (k, v) in &prev_obj {
        if is_internal(k) {
            continue;
        }
        if !current_obj.contains_key(k) {
            out.push((k.clone(), v.clone(), serde_json::Value::Null));
        }
    }

    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn apply_pagination<T>(rows: Vec<T>, offset: i64, limit_plus_one: i64) -> Result<Vec<T>> {
    let offset = offset.max(0) as usize;
    let take = limit_plus_one.max(1) as usize;
    Ok(rows.into_iter().skip(offset).take(take).collect())
}

fn synthetic_baseline_revision_ids(rows: &[CtcRevisionRow]) -> HashSet<Uuid> {
    rows.iter()
        .filter(|candidate| {
            candidate.revision_number == 1
                && candidate.reason == CTC_SYNTHETIC_BASELINE_REASON
                && rows.iter().any(|other| {
                    other.resource_id == candidate.resource_id
                        && other.revision_number == 2
                        && other.created_at >= candidate.created_at
                        && other.created_at == candidate.created_at
                })
        })
        .map(|row| row.revision_id)
        .collect()
}

fn uuid_from_value(value: Option<&serde_json::Value>) -> Option<Uuid> {
    value
        .and_then(|v| v.as_str())
        .and_then(|raw| Uuid::parse_str(raw).ok())
}

fn audit_uuid_from_changes(changes: &serde_json::Value, key: &str) -> Option<Uuid> {
    let top_level = changes.get(key);
    let after = changes.get("after").and_then(|v| v.get(key));
    let before = changes.get("before").and_then(|v| v.get(key));
    uuid_from_value(top_level)
        .or_else(|| uuid_from_value(after))
        .or_else(|| uuid_from_value(before))
}

async fn lookup_resource_name(pool: &PgPool, resource_id: Option<Uuid>) -> Result<Option<String>> {
    let Some(resource_id) = resource_id else {
        return Ok(None);
    };

    sqlx::query_scalar::<_, String>("SELECT name FROM resources WHERE id = $1")
        .bind(resource_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
}

async fn lookup_project_name(pool: &PgPool, project_id: Option<Uuid>) -> Result<Option<String>> {
    let Some(project_id) = project_id else {
        return Ok(None);
    };

    sqlx::query_scalar::<_, String>("SELECT name FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
}

async fn lookup_resource_name_cached(
    pool: &PgPool,
    cache: &mut HashMap<Uuid, Option<String>>,
    resource_id: Option<Uuid>,
) -> Result<Option<String>> {
    let Some(resource_id) = resource_id else {
        return Ok(None);
    };
    if let Some(name) = cache.get(&resource_id) {
        return Ok(name.clone());
    }
    let name = lookup_resource_name(pool, Some(resource_id)).await?;
    cache.insert(resource_id, name.clone());
    Ok(name)
}

async fn lookup_project_name_cached(
    pool: &PgPool,
    cache: &mut HashMap<Uuid, Option<String>>,
    project_id: Option<Uuid>,
) -> Result<Option<String>> {
    let Some(project_id) = project_id else {
        return Ok(None);
    };
    if let Some(name) = cache.get(&project_id) {
        return Ok(name.clone());
    }
    let name = lookup_project_name(pool, Some(project_id)).await?;
    cache.insert(project_id, name.clone());
    Ok(name)
}

// ---------------- Assignment History ----------------

async fn build_assignment_history(
    pool: &PgPool,
    filters: &ReportFilters,
) -> Result<Vec<AssignmentHistoryRow>> {
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
        SELECT
            al.id AS audit_id,
            al.user_id AS user_id,
            u.first_name AS first_name,
            u.last_name AS last_name,
            al.action AS action,
            al.entity_id AS allocation_id,
            al.changes AS changes,
            al.created_at AS created_at,
            a.resource_id AS resource_id,
            res.name AS resource_name,
            a.project_id AS project_id,
            p.name AS project_name
        FROM audit_logs al
        LEFT JOIN users u ON u.id = al.user_id
        LEFT JOIN allocations a ON a.id = al.entity_id
        LEFT JOIN resources res ON res.id = a.resource_id
        LEFT JOIN projects p ON p.id = a.project_id
        WHERE al.entity_type = 'allocation'
          AND al.action <> 'ACCESS_DENIED'
          AND al.action IN (
              'create',
              'update',
              'delete',
              'CREATE_ALLOCATION',
              'UPDATE_ALLOCATION',
              'DELETE_ALLOCATION',
              'overallocation_confirmed'
          )
        "#,
    );

    qb.push(" AND al.created_at >= ")
        .push_bind(start_of_day(filters.start_date)?);
    qb.push(" AND al.created_at < ")
        .push_bind(exclusive_end(filters.end_date)?);
    qb.push(" AND al.created_at <= ")
        .push_bind(filters.snapshot_at);

    if let Some(user_id) = filters.user_id {
        qb.push(" AND al.user_id = ").push_bind(user_id);
    }
    if let Some(action) = &filters.action_type {
        qb.push(" AND al.action = ").push_bind(action.clone());
    }

    qb.push(" ORDER BY al.created_at DESC, al.id DESC ");
    qb.push(" LIMIT ").push_bind(filters.limit + 1);
    qb.push(" OFFSET ").push_bind(filters.offset);

    let rows = qb
        .build()
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut result = Vec::with_capacity(rows.len());
    let mut resource_name_cache: HashMap<Uuid, Option<String>> = HashMap::new();
    let mut project_name_cache: HashMap<Uuid, Option<String>> = HashMap::new();
    for row in rows {
        let first: Option<String> = row.try_get("first_name").map_err(db_err)?;
        let last: Option<String> = row.try_get("last_name").map_err(db_err)?;
        let changes: serde_json::Value = row
            .try_get::<Option<serde_json::Value>, _>("changes")
            .map_err(db_err)?
            .unwrap_or(serde_json::Value::Null);

        let live_resource_id: Option<Uuid> = row.try_get("resource_id").map_err(db_err)?;
        let live_project_id: Option<Uuid> = row.try_get("project_id").map_err(db_err)?;
        let resource_id = audit_uuid_from_changes(&changes, "resource_id").or(live_resource_id);
        let project_id = audit_uuid_from_changes(&changes, "project_id").or(live_project_id);
        let live_resource_name: Option<String> = row.try_get("resource_name").map_err(db_err)?;
        let live_project_name: Option<String> = row.try_get("project_name").map_err(db_err)?;
        let before = changes
            .get("before")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let after = changes
            .get("after")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let before_summary = summarize_change_payload(&before);
        let after_summary = summarize_change_payload(&after);
        let resource_name = if resource_id == live_resource_id {
            live_resource_name
        } else {
            lookup_resource_name_cached(pool, &mut resource_name_cache, resource_id).await?
        };
        let project_name = if project_id == live_project_id {
            live_project_name
        } else {
            lookup_project_name_cached(pool, &mut project_name_cache, project_id).await?
        };

        result.push(AssignmentHistoryRow {
            audit_id: row.try_get("audit_id").map_err(db_err)?,
            timestamp: row.try_get("created_at").map_err(db_err)?,
            user_id: row.try_get("user_id").map_err(db_err)?,
            user_name: full_name(first, last),
            action: row.try_get("action").map_err(db_err)?,
            allocation_id: row.try_get("allocation_id").map_err(db_err)?,
            resource_id,
            resource_name,
            project_id,
            project_name,
            before_summary,
            after_summary,
        });
    }

    Ok(result)
}

// ---------------- Budget Modifications ----------------

async fn build_budget_modifications(
    pool: &PgPool,
    filters: &ReportFilters,
) -> Result<Vec<BudgetModificationRow>> {
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
        SELECT
            al.id AS audit_id,
            al.user_id AS user_id,
            u.first_name AS first_name,
            u.last_name AS last_name,
            al.action AS action,
            al.entity_id AS project_id,
            al.changes AS changes,
            al.created_at AS created_at,
            p.name AS project_name
        FROM audit_logs al
        LEFT JOIN users u ON u.id = al.user_id
        LEFT JOIN projects p ON p.id = al.entity_id
        WHERE al.entity_type = 'project_budget'
          AND al.action <> 'ACCESS_DENIED'
        "#,
    );

    qb.push(" AND al.created_at >= ")
        .push_bind(start_of_day(filters.start_date)?);
    qb.push(" AND al.created_at < ")
        .push_bind(exclusive_end(filters.end_date)?);
    qb.push(" AND al.created_at <= ")
        .push_bind(filters.snapshot_at);

    if let Some(user_id) = filters.user_id {
        qb.push(" AND al.user_id = ").push_bind(user_id);
    }
    if let Some(action) = &filters.action_type {
        qb.push(" AND al.action = ").push_bind(action.clone());
    }

    qb.push(" ORDER BY al.created_at DESC, al.id DESC ");
    qb.push(" LIMIT ").push_bind(filters.limit + 1);
    qb.push(" OFFSET ").push_bind(filters.offset);

    let rows = qb
        .build()
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let first: Option<String> = row.try_get("first_name").map_err(db_err)?;
        let last: Option<String> = row.try_get("last_name").map_err(db_err)?;
        let changes: serde_json::Value = row
            .try_get::<Option<serde_json::Value>, _>("changes")
            .map_err(db_err)?
            .unwrap_or(serde_json::Value::Null);
        let before = changes
            .get("before")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let after = changes
            .get("after")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let before_summary = summarize_change_payload(&before);
        let after_summary = summarize_change_payload(&after);

        result.push(BudgetModificationRow {
            audit_id: row.try_get("audit_id").map_err(db_err)?,
            timestamp: row.try_get("created_at").map_err(db_err)?,
            user_id: row.try_get("user_id").map_err(db_err)?,
            user_name: full_name(first, last),
            action: row.try_get("action").map_err(db_err)?,
            project_id: row.try_get("project_id").map_err(db_err)?,
            project_name: row.try_get("project_name").map_err(db_err)?,
            before_summary,
            after_summary,
        });
    }

    Ok(result)
}

// ---------------- Access Logs ----------------

pub fn access_log_action_set() -> &'static [&'static str] {
    &[
        "LOGIN_SUCCESS",
        "LOGIN_FAILED",
        "LOGIN_BLOCKED",
        "TOKEN_REFRESH",
        "ACCESS_DENIED",
        "ACCOUNT_LOCKED",
        "VIEW",
        "VIEW_AUDIT_REPORT",
        "VERIFY_CHAIN",
        "CTC_VIEW_CROSS_DEPT",
        "THR_REPORT_VIEW",
        "COMPLIANCE_AUDIT_REPORT_GENERATED",
        "compliance_report_generated",
        "ctc_validation_report_generated",
        "EXPORT_REQUESTED",
    ]
}

fn classify_success(action: &str) -> bool {
    matches!(
        action,
        "LOGIN_SUCCESS"
            | "TOKEN_REFRESH"
            | "VIEW"
            | "VIEW_AUDIT_REPORT"
            | "VERIFY_CHAIN"
            | "CTC_VIEW_CROSS_DEPT"
            | "THR_REPORT_VIEW"
            | "COMPLIANCE_AUDIT_REPORT_GENERATED"
            | "compliance_report_generated"
            | "ctc_validation_report_generated"
            | "EXPORT_REQUESTED"
    )
}

async fn build_access_logs(pool: &PgPool, filters: &ReportFilters) -> Result<Vec<AccessLogRow>> {
    let action_set = access_log_action_set();

    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
        SELECT
            al.id AS audit_id,
            al.user_id AS user_id,
            u.first_name AS first_name,
            u.last_name AS last_name,
            al.action AS action,
            al.entity_type AS entity_type,
            al.entity_id AS entity_id,
            al.changes AS changes,
            al.created_at AS created_at
        FROM audit_logs al
        LEFT JOIN users u ON u.id = al.user_id
        WHERE
        "#,
    );

    qb.push(" al.action = ANY(")
        .push_bind(action_set)
        .push(") ");
    qb.push(" AND al.created_at >= ")
        .push_bind(start_of_day(filters.start_date)?);
    qb.push(" AND al.created_at < ")
        .push_bind(exclusive_end(filters.end_date)?);
    qb.push(" AND al.created_at <= ")
        .push_bind(filters.snapshot_at);

    if let Some(user_id) = filters.user_id {
        qb.push(" AND al.user_id = ").push_bind(user_id);
    }
    if let Some(action) = &filters.action_type {
        qb.push(" AND al.action = ").push_bind(action.clone());
    }

    qb.push(" ORDER BY al.created_at DESC, al.id DESC ");
    qb.push(" LIMIT ").push_bind(filters.limit + 1);
    qb.push(" OFFSET ").push_bind(filters.offset);

    let rows = qb
        .build()
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let first: Option<String> = row.try_get("first_name").map_err(db_err)?;
        let last: Option<String> = row.try_get("last_name").map_err(db_err)?;
        let action: String = row.try_get("action").map_err(db_err)?;
        let changes: serde_json::Value = row
            .try_get::<Option<serde_json::Value>, _>("changes")
            .map_err(db_err)?
            .unwrap_or(serde_json::Value::Null);
        let reason = changes
            .get("reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        result.push(AccessLogRow {
            audit_id: row.try_get("audit_id").map_err(db_err)?,
            timestamp: row.try_get("created_at").map_err(db_err)?,
            user_id: row.try_get("user_id").map_err(db_err)?,
            user_name: full_name(first, last),
            success: classify_success(&action),
            action,
            resource_type: row.try_get("entity_type").map_err(db_err)?,
            resource_id: row.try_get("entity_id").map_err(db_err)?,
            reason,
        });
    }

    Ok(result)
}

fn db_err(e: sqlx::Error) -> AppError {
    AppError::Database(e.to_string())
}

/// Watermark metadata returned to clients when initiating an export request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportWatermark {
    pub export_id: Uuid,
    pub report_type: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub snapshot_at: DateTime<Utc>,
    pub requested_by: Uuid,
    pub requested_by_email: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub text: String,
}

pub fn build_watermark(
    export_id: Uuid,
    requested_by: Uuid,
    requested_by_email: Option<String>,
    requested_at: DateTime<Utc>,
    filters: &ReportFilters,
) -> ExportWatermark {
    let mut text = format!(
        "Xynergy audit export | export_id={} | requested_by={} | requested_at={} | report_type={} | window={}..{} | snapshot_at={}",
        export_id,
        requested_by,
        requested_at.to_rfc3339(),
        filters.report_type.as_str(),
        filters.start_date,
        filters.end_date,
        filters.snapshot_at.to_rfc3339(),
    );
    if let Some(email) = &requested_by_email {
        text.push_str(" | requested_by_email=");
        text.push_str(&watermark_safe_value(email));
    }
    if let Some(user_id) = filters.user_id {
        text.push_str(" | filter_user_id=");
        text.push_str(&user_id.to_string());
    }
    if let Some(action_type) = &filters.action_type {
        text.push_str(" | filter_action_type=");
        text.push_str(&watermark_safe_value(action_type));
    }

    ExportWatermark {
        export_id,
        report_type: filters.report_type.as_str().to_string(),
        start_date: filters.start_date,
        end_date: filters.end_date,
        snapshot_at: filters.snapshot_at,
        requested_by,
        requested_by_email,
        requested_at,
        text,
    }
}

fn watermark_safe_value(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if matches!(ch, '|' | '\n' | '\r') {
                ' '
            } else {
                ch
            }
        })
        .collect()
}

fn summarize_change_payload(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "No captured value".to_string(),
        serde_json::Value::Object(map) if map.is_empty() => "No captured value".to_string(),
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map
                .keys()
                .filter(|key| !key.starts_with('_') && !key.starts_with("encrypted"))
                .collect();
            keys.sort();

            if keys.is_empty() {
                return "No reportable fields".to_string();
            }

            let mut parts = Vec::new();
            for key in keys.into_iter().take(6) {
                if let Some(field_value) = map.get(key) {
                    parts.push(format!("{}: {}", key, summarize_scalar(field_value)));
                }
            }
            if map.len() > parts.len() {
                parts.push("...".to_string());
            }
            parts.join(", ")
        }
        other => summarize_scalar(other),
    }
}

fn summarize_scalar(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "none".to_string(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => value.chars().take(80).collect(),
        serde_json::Value::Array(items) => format!("{} item(s)", items.len()),
        serde_json::Value::Object(map) => format!("{} field(s)", map.len()),
    }
}
