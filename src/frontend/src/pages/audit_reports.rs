use crate::auth::{authenticated_get, authenticated_post_json, use_auth};
use leptos::either::Either;
use leptos::prelude::*;
use leptos_router::hooks::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const PAGE_LIMIT: i64 = 100;
const MAX_INTERACTIVE_RANGE_DAYS: i64 = 90;
const VALUE_PREVIEW_LIMIT: usize = 240;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportType {
    CtcChangeLog,
    AssignmentHistory,
    BudgetModifications,
    AccessLogs,
}

impl ReportType {
    fn as_str(&self) -> &'static str {
        match self {
            ReportType::CtcChangeLog => "ctc_change_log",
            ReportType::AssignmentHistory => "assignment_history",
            ReportType::BudgetModifications => "budget_modifications",
            ReportType::AccessLogs => "access_logs",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            ReportType::CtcChangeLog => "CTC Change Log",
            ReportType::AssignmentHistory => "Assignment History",
            ReportType::BudgetModifications => "Budget Modifications",
            ReportType::AccessLogs => "Access Logs",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "ctc_change_log" => Some(ReportType::CtcChangeLog),
            "assignment_history" => Some(ReportType::AssignmentHistory),
            "budget_modifications" => Some(ReportType::BudgetModifications),
            "access_logs" => Some(ReportType::AccessLogs),
            _ => None,
        }
    }

    fn label_for(value: &str) -> &'static str {
        Self::from_str(value)
            .map(|report_type| report_type.label())
            .unwrap_or("Unknown Report")
    }

    fn supports_user_action_filters(&self) -> bool {
        matches!(self, ReportType::AccessLogs)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct CtcChangeLogRow {
    revision_id: Uuid,
    revision_number: i32,
    change_date: String,
    employee_id: Uuid,
    employee_name: String,
    changed_by_id: Uuid,
    changed_by_name: Option<String>,
    field: String,
    old_value: serde_json::Value,
    new_value: serde_json::Value,
    reason: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AssignmentHistoryRow {
    audit_id: Uuid,
    timestamp: String,
    user_id: Option<Uuid>,
    user_name: Option<String>,
    action: String,
    allocation_id: Option<Uuid>,
    resource_id: Option<Uuid>,
    resource_name: Option<String>,
    project_id: Option<Uuid>,
    project_name: Option<String>,
    before_summary: String,
    after_summary: String,
}

#[derive(Debug, Clone, Deserialize)]
struct BudgetModificationRow {
    audit_id: Uuid,
    timestamp: String,
    user_id: Option<Uuid>,
    user_name: Option<String>,
    action: String,
    project_id: Option<Uuid>,
    project_name: Option<String>,
    before_summary: String,
    after_summary: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AccessLogRow {
    audit_id: Uuid,
    timestamp: String,
    user_id: Option<Uuid>,
    user_name: Option<String>,
    action: String,
    resource_type: String,
    resource_id: Option<Uuid>,
    success: bool,
    reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
enum ReportRows {
    CtcChangeLog(Vec<CtcChangeLogRow>),
    AssignmentHistory(Vec<AssignmentHistoryRow>),
    BudgetModifications(Vec<BudgetModificationRow>),
    AccessLogs(Vec<AccessLogRow>),
}

impl ReportRows {
    fn len(&self) -> usize {
        match self {
            ReportRows::CtcChangeLog(r) => r.len(),
            ReportRows::AssignmentHistory(r) => r.len(),
            ReportRows::BudgetModifications(r) => r.len(),
            ReportRows::AccessLogs(r) => r.len(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ComplianceAuditReportData {
    report_type: String,
    start_date: String,
    end_date: String,
    snapshot_at: String,
    limit: i64,
    offset: i64,
    has_more: bool,
    rows: ReportRows,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReportFilters {
    report_type: String,
    start_date: String,
    end_date: String,
    user_id: String,
    action_type: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportRequestPayload {
    report_type: String,
    start_date: String,
    end_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    action_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ExportApprovalState {
    export_id: Uuid,
    status: String,
    #[serde(default)]
    report_type: Option<String>,
    #[serde(default)]
    watermark: Option<ExportWatermark>,
}

#[derive(Debug, Clone, Deserialize)]
struct ExportWatermark {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ApiErrorEnvelope {
    error: Option<ApiErrorBody>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    message: Option<String>,
}

fn audit_error_message(status: reqwest::StatusCode, body: &str) -> String {
    let parsed = serde_json::from_str::<ApiErrorEnvelope>(body)
        .ok()
        .and_then(|env| env.error.and_then(|e| e.message).or(env.message))
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty());

    let message = parsed.unwrap_or_else(|| {
        "The audit report could not be loaded. Check the selected filters and try again."
            .to_string()
    });

    format!(
        "Audit report request failed ({}): {}",
        status.as_u16(),
        message
    )
}

fn parsed_filter_dates(
    filters: &ReportFilters,
) -> Result<(chrono::NaiveDate, chrono::NaiveDate), String> {
    if filters.start_date.is_empty() || filters.end_date.is_empty() {
        return Err("Start date and end date are required".to_string());
    }
    let start = chrono::NaiveDate::parse_from_str(&filters.start_date, "%Y-%m-%d")
        .map_err(|_| "Start date must use YYYY-MM-DD".to_string())?;
    let end = chrono::NaiveDate::parse_from_str(&filters.end_date, "%Y-%m-%d")
        .map_err(|_| "End date must use YYYY-MM-DD".to_string())?;
    if start > end {
        return Err("End date must be greater than or equal to start date".to_string());
    }
    Ok((start, end))
}

fn is_interactive_range(filters: &ReportFilters) -> bool {
    parsed_filter_dates(filters)
        .map(|(start, end)| (end - start).num_days() + 1 <= MAX_INTERACTIVE_RANGE_DAYS)
        .unwrap_or(false)
}

fn validate_filters(filters: &ReportFilters, enforce_interactive_cap: bool) -> Result<(), String> {
    if ReportType::from_str(&filters.report_type).is_none() {
        return Err("Report type is not supported".to_string());
    }
    let (start, end) = parsed_filter_dates(filters)?;
    let inclusive_days = (end - start).num_days() + 1;
    if enforce_interactive_cap && inclusive_days > MAX_INTERACTIVE_RANGE_DAYS {
        return Err(format!(
            "Audit report preview is limited to {} days. Narrow the date range or request an export for the wider window.",
            MAX_INTERACTIVE_RANGE_DAYS
        ));
    }
    if !filters.user_id.is_empty() && Uuid::parse_str(&filters.user_id).is_err() {
        return Err("User ID must be a valid UUID".to_string());
    }
    Ok(())
}

fn append_limited(out: &mut String, text: &str, limit: usize) {
    let remaining = limit.saturating_sub(out.chars().count());
    if remaining == 0 {
        return;
    }
    for ch in text.chars().take(remaining) {
        out.push(ch);
    }
}

fn push_json_preview(value: &serde_json::Value, out: &mut String, depth: usize, limit: usize) {
    if out.chars().count() >= limit {
        return;
    }

    match value {
        serde_json::Value::Null => append_limited(out, "—", limit),
        serde_json::Value::Bool(b) => append_limited(out, &b.to_string(), limit),
        serde_json::Value::Number(n) => append_limited(out, &n.to_string(), limit),
        serde_json::Value::String(s) => {
            append_limited(out, "\"", limit);
            append_limited(out, s, limit);
            append_limited(out, "\"", limit);
        }
        serde_json::Value::Array(items) => {
            append_limited(out, "[", limit);
            if depth >= 2 {
                append_limited(out, "…", limit);
            } else {
                for (idx, item) in items.iter().take(6).enumerate() {
                    if idx > 0 {
                        append_limited(out, ", ", limit);
                    }
                    push_json_preview(item, out, depth + 1, limit);
                }
                if items.len() > 6 {
                    append_limited(out, ", …", limit);
                }
            }
            append_limited(out, "]", limit);
        }
        serde_json::Value::Object(map) => {
            append_limited(out, "{", limit);
            if depth >= 2 {
                append_limited(out, "…", limit);
            } else {
                for (idx, (key, item)) in map.iter().take(8).enumerate() {
                    if idx > 0 {
                        append_limited(out, ", ", limit);
                    }
                    append_limited(out, "\"", limit);
                    append_limited(out, key, limit);
                    append_limited(out, "\": ", limit);
                    push_json_preview(item, out, depth + 1, limit);
                }
                if map.len() > 8 {
                    append_limited(out, ", …", limit);
                }
            }
            append_limited(out, "}", limit);
        }
    }
}

fn json_preview(value: &serde_json::Value) -> String {
    let mut out = String::new();
    push_json_preview(value, &mut out, 0, VALUE_PREVIEW_LIMIT);
    if out.chars().count() >= VALUE_PREVIEW_LIMIT {
        append_limited(&mut out, "…", VALUE_PREVIEW_LIMIT + 1);
    }
    out
}

fn value_preview(text: &str) -> String {
    let preview = text.chars().take(VALUE_PREVIEW_LIMIT).collect::<String>();
    if text.chars().count() > VALUE_PREVIEW_LIMIT {
        format!("{}…", preview)
    } else {
        preview
    }
}

fn pretty_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "—".to_string(),
        serde_json::Value::String(s) => value_preview(s),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => json_preview(value),
    }
}

fn format_timestamp(raw: &str) -> String {
    // Trim trailing fractional seconds for readability while keeping ISO-8601.
    if let Some(idx) = raw.find('.') {
        let mut shortened = raw[..idx].to_string();
        if let Some(suffix_idx) = raw[idx..].find(|c: char| c == 'Z' || c == '+' || c == '-') {
            shortened.push_str(&raw[idx + suffix_idx..]);
        }
        shortened
    } else {
        raw.to_string()
    }
}

async fn fetch_audit_report(
    filters: &ReportFilters,
    offset: i64,
    snapshot_at: Option<String>,
) -> Result<ComplianceAuditReportData, String> {
    let mut url = format!(
        "/api/v1/audit-logs/reports?report_type={}&start_date={}&end_date={}&limit={}&offset={}",
        filters.report_type, filters.start_date, filters.end_date, PAGE_LIMIT, offset
    );
    if !filters.user_id.is_empty() {
        url.push_str("&user_id=");
        url.push_str(&filters.user_id);
    }
    if !filters.action_type.is_empty() {
        url.push_str("&action_type=");
        let encoded = js_sys::encode_uri_component(&filters.action_type)
            .as_string()
            .unwrap_or_else(|| filters.action_type.clone());
        url.push_str(&encoded);
    }
    if let Some(snapshot_at) = snapshot_at {
        url.push_str("&snapshot_at=");
        let encoded = js_sys::encode_uri_component(&snapshot_at)
            .as_string()
            .unwrap_or(snapshot_at);
        url.push_str(&encoded);
    }

    let response = authenticated_get(&url)
        .await
        .map_err(|e| format!("Failed to fetch audit report: {}", e))?;

    if response.status().is_success() {
        response
            .json::<ComplianceAuditReportData>()
            .await
            .map_err(|e| format!("Failed to parse audit report: {}", e))
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(audit_error_message(status, &body))
    }
}

async fn request_export(
    filters: &ReportFilters,
    snapshot_at: Option<String>,
) -> Result<ExportApprovalState, String> {
    let user_id = if filters.user_id.is_empty() {
        None
    } else {
        Uuid::parse_str(&filters.user_id).ok()
    };
    let action_type = if filters.action_type.is_empty() {
        None
    } else {
        Some(filters.action_type.clone())
    };

    let payload = ExportRequestPayload {
        report_type: filters.report_type.clone(),
        start_date: filters.start_date.clone(),
        end_date: filters.end_date.clone(),
        snapshot_at,
        user_id,
        action_type,
    };

    let response = authenticated_post_json("/api/v1/audit-logs/export", &payload)
        .await
        .map_err(|e| format!("Failed to initiate export request: {}", e))?;

    if response.status().is_success() {
        response
            .json::<ExportApprovalState>()
            .await
            .map_err(|e| format!("Failed to parse export response: {}", e))
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(audit_error_message(status, &body))
    }
}

#[component]
pub fn AuditReportsPage() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();

    let user_role = Signal::derive(move || auth.user.get().map(|u| u.role).unwrap_or_default());
    let is_admin = Signal::derive(move || user_role.get() == "admin");
    let is_finance = Signal::derive(move || user_role.get() == "finance");
    let can_access = Signal::derive(move || is_finance.get() || is_admin.get());
    let auth_hydrating =
        Signal::derive(move || auth.token.get().is_some() && auth.user.get().is_none());

    let now = chrono::Local::now().date_naive();
    let default_start = (now - chrono::Duration::days(MAX_INTERACTIVE_RANGE_DAYS - 1)).to_string();
    let default_end = now.to_string();

    let (report_type, set_report_type) = signal(ReportType::CtcChangeLog.as_str().to_string());
    let (start_date, set_start_date) = signal(default_start);
    let (end_date, set_end_date) = signal(default_end);
    let (user_id_filter, set_user_id_filter) = signal(String::new());
    let (action_filter, set_action_filter) = signal(String::new());

    let (report, set_report) = signal(Option::<ComplianceAuditReportData>::None);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (applied_filters, set_applied_filters) = signal(Option::<ReportFilters>::None);
    let (page_offset, set_page_offset) = signal(0_i64);

    let (export_status, set_export_status) = signal(Option::<ExportApprovalState>::None);
    let (export_error, set_export_error) = signal(Option::<String>::None);
    let (exporting, set_exporting) = signal(false);

    let clear_export_state = move || {
        set_export_status.set(None);
        set_export_error.set(None);
    };

    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            if !auth.is_authenticated.get() {
                navigate("/login", Default::default());
            }
        });
    }

    let build_filters = move || {
        let mut filters = ReportFilters {
            report_type: report_type.get().trim().to_string(),
            start_date: start_date.get().trim().to_string(),
            end_date: end_date.get().trim().to_string(),
            user_id: user_id_filter.get().trim().to_string(),
            action_type: action_filter.get().trim().to_string(),
        };
        if !ReportType::from_str(&filters.report_type)
            .map(|report_type| report_type.supports_user_action_filters())
            .unwrap_or(false)
        {
            filters.user_id.clear();
            filters.action_type.clear();
        }
        filters
    };

    let filters_dirty = Signal::derive(move || {
        applied_filters
            .get()
            .map(|applied| applied != build_filters())
            .unwrap_or(false)
    });
    let can_request_export = Signal::derive(move || {
        let current_filters = build_filters();
        if !is_interactive_range(&current_filters) {
            return true;
        }
        applied_filters
            .get()
            .map(|applied| applied == current_filters)
            .unwrap_or(false)
    });

    let run_report = move |offset: i64| {
        set_error.set(None);
        let filters = if offset == 0 {
            build_filters()
        } else {
            match applied_filters.get() {
                Some(filters) => filters,
                None => {
                    set_error.set(Some(
                        "Run the report before loading additional pages.".to_string(),
                    ));
                    return;
                }
            }
        };
        if let Err(message) = validate_filters(&filters, true) {
            set_error.set(Some(message));
            if offset == 0 {
                set_report.set(None);
                set_applied_filters.set(None);
                set_page_offset.set(0);
            }
            return;
        }
        clear_export_state();
        if offset == 0 {
            set_report.set(None);
            set_applied_filters.set(None);
            set_page_offset.set(0);
        }
        set_loading.set(true);
        let snapshot_at = if offset == 0 {
            None
        } else {
            report.get().map(|data| data.snapshot_at)
        };
        leptos::task::spawn_local(async move {
            match fetch_audit_report(&filters, offset, snapshot_at).await {
                Ok(data) => {
                    let server_offset = data.offset;
                    set_applied_filters.set(Some(filters.clone()));
                    set_page_offset.set(server_offset);
                    set_report.set(Some(data));
                }
                Err(e) => {
                    if offset == 0 {
                        set_report.set(None);
                        set_applied_filters.set(None);
                        set_page_offset.set(0);
                    }
                    set_error.set(Some(e));
                }
            }
            set_loading.set(false);
        });
    };

    let handle_run = move |_| run_report(0);

    let handle_export = move |_| {
        if exporting.get() || loading.get() || export_status.get().is_some() {
            return;
        }
        set_export_error.set(None);
        set_export_status.set(None);
        let current_filters = build_filters();
        if let Err(message) = validate_filters(&current_filters, false) {
            set_export_error.set(Some(message));
            return;
        }
        let (filters, snapshot_at) = if is_interactive_range(&current_filters) {
            let Some(filters) = applied_filters.get() else {
                set_export_error.set(Some(
                    "Run the report first to confirm filters before requesting an export."
                        .to_string(),
                ));
                return;
            };
            if filters != current_filters {
                set_export_error.set(Some(
                    "Run the report again before requesting an export with changed filters."
                        .to_string(),
                ));
                return;
            }
            let Some(snapshot_at) = report.get().map(|data| data.snapshot_at) else {
                set_export_error.set(Some(
                    "Run the report again before requesting an export.".to_string(),
                ));
                return;
            };
            (filters, Some(snapshot_at))
        } else {
            (current_filters, None)
        };
        set_exporting.set(true);
        leptos::task::spawn_local(async move {
            match request_export(&filters, snapshot_at).await {
                Ok(state) => {
                    set_export_status.set(Some(state));
                }
                Err(e) => {
                    set_export_error.set(Some(e));
                }
            }
            set_exporting.set(false);
        });
    };

    view! {
        <div class="h-full">
            <div class="page-container fade-in">
                <div class="space-y-4">
                    <div class="page-header">
                        <h1 class="text-xl font-semibold text-huly-caption">"Audit Reports"</h1>
                    </div>

                    {move || {
                        if auth_hydrating.get() {
                            view! {
                                <div class="panel p-4" role="status" aria-live="polite">
                                    <p class="text-sm text-huly-muted">"Loading account access..."</p>
                                </div>
                            }.into_any()
                        } else if !can_access.get() {
                            view! {
                                <div class="alert-error" role="alert">
                                    "Access denied. Audit Reports are available to Finance and Admin roles only."
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="space-y-4">
                                    {move || error.get().map(|err| view! { <div class="alert-error" role="alert">{err}</div> })}

                                    <div class="panel p-4" aria-busy=move || loading.get()>
                                        <h2 class="section-header mb-4">"Report Filters"</h2>
                                        <div class="flex flex-wrap items-end gap-4">
                                            <div>
                                                <label class="label text-xs" for="audit-report-type">"Report Type"</label>
                                                <select
                                                    id="audit-report-type"
                                                    class="input mt-1 min-h-[44px] w-56"
                                                    disabled=move || loading.get()
                                                    on:change=move |ev| {
                                                        let next_report_type = event_target_value(&ev);
                                                        if !ReportType::from_str(&next_report_type)
                                                            .map(|report_type| report_type.supports_user_action_filters())
                                                            .unwrap_or(false)
                                                        {
                                                            set_user_id_filter.set(String::new());
                                                            set_action_filter.set(String::new());
                                                        }
                                                        set_report_type.set(next_report_type);
                                                        set_error.set(None);
                                                        clear_export_state();
                                                    }
                                                >
                                                    <option value="ctc_change_log" selected=move || report_type.get() == "ctc_change_log">{ReportType::CtcChangeLog.label()}</option>
                                                    <option value="assignment_history" selected=move || report_type.get() == "assignment_history">{ReportType::AssignmentHistory.label()}</option>
                                                    <option value="budget_modifications" selected=move || report_type.get() == "budget_modifications">{ReportType::BudgetModifications.label()}</option>
                                                    <option value="access_logs" selected=move || report_type.get() == "access_logs">{ReportType::AccessLogs.label()}</option>
                                                </select>
                                            </div>
                                            <div>
                                                <label class="label text-xs" for="audit-start">"Start Date"</label>
                                                <input
                                                    id="audit-start"
                                                    type="date"
                                                    class="input mt-1 min-h-[44px] w-40"
                                                    prop:value=start_date
                                                    disabled=move || loading.get()
                                                    on:input=move |ev| {
                                                        set_start_date.set(event_target_value(&ev));
                                                        set_error.set(None);
                                                        clear_export_state();
                                                    }
                                                />
                                            </div>
                                            <div>
                                                <label class="label text-xs" for="audit-end">"End Date"</label>
                                                <input
                                                    id="audit-end"
                                                    type="date"
                                                    class="input mt-1 min-h-[44px] w-40"
                                                    prop:value=end_date
                                                    disabled=move || loading.get()
                                                    on:input=move |ev| {
                                                        set_end_date.set(event_target_value(&ev));
                                                        set_error.set(None);
                                                        clear_export_state();
                                                    }
                                                />
                                            </div>
                                            {move || if ReportType::from_str(&report_type.get())
                                                .map(|report_type| report_type.supports_user_action_filters())
                                                .unwrap_or(false)
                                            {
                                                Some(view! {
                                                    <>
                                                        <div>
                                                            <label class="label text-xs" for="audit-user">"User ID"</label>
                                                            <input
                                                                id="audit-user"
                                                                type="text"
                                                                class="input mt-1 min-h-[44px] w-64 font-mono text-xs"
                                                                placeholder="optional UUID"
                                                                prop:value=user_id_filter
                                                                disabled=move || loading.get()
                                                                on:input=move |ev| {
                                                                    set_user_id_filter.set(event_target_value(&ev));
                                                                    set_error.set(None);
                                                                    clear_export_state();
                                                                }
                                                            />
                                                        </div>
                                                        <div>
                                                            <label class="label text-xs" for="audit-action">"Action Type"</label>
                                                            <input
                                                                id="audit-action"
                                                                type="text"
                                                                class="input mt-1 min-h-[44px] w-56 text-xs"
                                                                placeholder="e.g. LOGIN_SUCCESS"
                                                                prop:value=action_filter
                                                                disabled=move || loading.get()
                                                                on:input=move |ev| {
                                                                    set_action_filter.set(event_target_value(&ev));
                                                                    set_error.set(None);
                                                                    clear_export_state();
                                                                }
                                                            />
                                                        </div>
                                                    </>
                                                })
                                            } else {
                                                None
                                            }}
                                            <button
                                                class="btn-primary btn-press text-sm min-h-[44px] min-w-[44px]"
                                                on:click=handle_run
                                                disabled=move || loading.get()
                                                aria-label="Run audit report"
                                                title="Generates the selected audit report using backend-authoritative data."
                                            >
                                                {move || if loading.get() { "Running..." } else { "Run Report" }}
                                            </button>
                                            <button
                                                class="btn-secondary btn-press text-sm min-h-[44px] min-w-[44px]"
                                                on:click=handle_export
                                                disabled=move || {
                                                    exporting.get()
                                                        || loading.get()
                                                        || !can_request_export.get()
                                                        || export_status.get().is_some()
                                                }
                                                aria-label="Request audit export with four-eyes approval"
                                                title="Creates a pending approval request. The downloadable export is not produced until a second approver approves."
                                            >
                                                {move || if exporting.get() { "Submitting..." } else { "Request Export" }}
                                            </button>
                                        </div>
                                        <p class="mt-3 text-xs text-huly-muted">
                                            "Audit Reports surface event history for compliance reviews. Exports are gated by four-eyes approval before any downloadable artifact is generated."
                                        </p>
                                    </div>

                                    {move || export_error.get().map(|e| view! { <div class="alert-error" role="alert">{e}</div> })}
                                    {move || export_status.get().map(|state| {
                                        let watermark_text = state.watermark.as_ref().map(|w| w.text.clone());
                                        view! {
                                            <div class="panel p-4" role="status" aria-live="polite">
                                                <h2 class="section-header mb-2">"Export Request"</h2>
                                                <p class="text-sm text-huly-content">
                                                    {format!("Export ID: {}", state.export_id)}
                                                </p>
                                                <p class="text-sm text-huly-content">
                                                    {format!("Status: {}", state.status)}
                                                </p>
                                                {state.report_type.map(|rt| view! {
                                                    <p class="text-sm text-huly-content">{format!("Report type: {}", rt)}</p>
                                                })}
                                                {watermark_text.map(|text| view! {
                                                    <p class="text-xs text-huly-muted mt-2 font-mono break-all">{text}</p>
                                                })}
                                                <p class="text-xs text-huly-muted mt-2">
                                                    {if state.status == "pending_approval" {
                                                        "Pending approval by a second authorized user. The downloadable export is not generated until approval is granted."
                                                    } else {
                                                        "The approval workflow returned this status for the export request."
                                                    }}
                                                </p>
                                            </div>
                                        }
                                    })}

                                    {move || {
                                        let current = report.get();
                                        let current_offset = page_offset.get();
                                        let dirty = filters_dirty.get();
                                        match current {
                                            Some(data) => {
                                                let total_rows = data.rows.len();
                                                let has_prev = current_offset > 0;
                                                let has_next = data.has_more;
                                                let report_label = ReportType::label_for(&data.report_type);
                                                let server_limit = data.limit;
                                                let range_label = format!("Showing {} ({} to {})", report_label, data.start_date, data.end_date);
                                                let rows_view = render_rows(&data.rows);

                                                Some(view! {
                                                    <div class="panel overflow-hidden" aria-busy=move || loading.get()>
                                                        <div class="toolbar flex items-center justify-between gap-3">
                                                            <div>
                                                                <h2 class="text-sm font-semibold text-huly-secondary uppercase tracking-wider">{report_label}</h2>
                                                                <p class="text-xs text-huly-muted mt-1">{range_label}</p>
                                                            </div>
                                                            <div class="flex items-center gap-2">
                                                                {move || loading.get().then(|| view! {
                                                                    <span class="text-xs text-huly-muted" role="status" aria-live="polite">"Loading..."</span>
                                                                })}
                                                                {dirty.then(|| view! {
                                                                    <span class="text-xs text-huly-muted" role="status" aria-live="polite">"Run Report to apply filter changes"</span>
                                                                })}
                                                                <span class="text-xs text-huly-muted">
                                                                    {if total_rows == 0 {
                                                                        "0 rows".to_string()
                                                                    } else {
                                                                        format!("Rows {}-{}", current_offset + 1, current_offset + total_rows as i64)
                                                                    }}
                                                                </span>
                                                                <button
                                                                    type="button"
                                                                    class="btn-secondary btn-press text-xs min-h-[44px] min-w-[44px]"
                                                                    disabled=move || loading.get() || dirty || !has_prev
                                                                    on:click=move |_| {
                                                                        let next_offset = (page_offset.get() - server_limit).max(0);
                                                                        run_report(next_offset);
                                                                    }
                                                                >
                                                                    "Previous"
                                                                </button>
                                                                <button
                                                                    type="button"
                                                                    class="btn-secondary btn-press text-xs min-h-[44px] min-w-[44px]"
                                                                    disabled=move || loading.get() || dirty || !has_next
                                                                    on:click=move |_| {
                                                                        run_report(page_offset.get() + server_limit);
                                                                    }
                                                                >
                                                                    "Next"
                                                                </button>
                                                            </div>
                                                        </div>
                                                        {if total_rows == 0 {
                                                            Either::Left(view! {
                                                                <div class="empty-state py-10">
                                                                    <p class="text-huly-muted text-sm">
                                                                        "No rows for the selected report and filters."
                                                                    </p>
                                                                </div>
                                                            })
                                                        } else {
                                                            Either::Right(rows_view)
                                                        }}
                                                    </div>
                                                }.into_any())
                                            }
                                            None => {
                                                if loading.get() {
                                                    Some(view! {
                                                        <div class="panel p-4" role="status" aria-live="polite">
                                                            <p class="text-sm text-huly-muted">"Loading audit report..."</p>
                                                        </div>
                                                    }.into_any())
                                                } else {
                                                    Some(view! {
                                                        <div class="panel p-4">
                                                            <p class="text-sm text-huly-muted">
                                                                "Select a report type and date range, then click \"Run Report\" to view audit data."
                                                            </p>
                                                        </div>
                                                    }.into_any())
                                                }
                                            }
                                        }
                                    }}
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

fn render_rows(rows: &ReportRows) -> AnyView {
    match rows {
        ReportRows::CtcChangeLog(rows) => render_ctc_change_log(rows.clone()).into_any(),
        ReportRows::AssignmentHistory(rows) => render_assignment_history(rows.clone()).into_any(),
        ReportRows::BudgetModifications(rows) => {
            render_budget_modifications(rows.clone()).into_any()
        }
        ReportRows::AccessLogs(rows) => render_access_logs(rows.clone()).into_any(),
    }
}

fn render_ctc_change_log(rows: Vec<CtcChangeLogRow>) -> impl IntoView {
    view! {
        <div class="overflow-x-auto">
            <table class="min-w-full divide-y divide-huly-divider">
                <caption class="sr-only">"CTC change log report"</caption>
                <thead class="table-head">
                    <tr>
                        <th class="th-cell-compact" scope="col">"Employee"</th>
                        <th class="th-cell-compact" scope="col">"Changed By"</th>
                        <th class="th-cell-compact" scope="col">"Change Date"</th>
                        <th class="th-cell-compact" scope="col">"Field"</th>
                        <th class="th-cell-compact" scope="col">"Old Value"</th>
                        <th class="th-cell-compact" scope="col">"New Value"</th>
                        <th class="th-cell-compact" scope="col">"Reason"</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-huly-divider bg-huly-surface">
                    {rows.into_iter().map(|row| view! {
                        <tr class="table-row-hover">
                            <td class="td-cell-compact font-medium text-huly-caption">
                                <div>{row.employee_name}</div>
                                <div class="text-[0.6875rem] font-normal text-huly-muted">{row.employee_id.to_string()}</div>
                            </td>
                            <td class="td-cell-compact text-huly-content">
                                <div>{row.changed_by_name.unwrap_or_else(|| "Unknown".to_string())}</div>
                                <div class="text-[0.6875rem] font-normal text-huly-muted">{row.changed_by_id.to_string()}</div>
                            </td>
                            <td class="td-cell-compact text-huly-content text-xs">
                                <div>{format_timestamp(&row.change_date)}</div>
                                <div class="text-[0.6875rem] font-normal text-huly-muted">{format!("rev {}", row.revision_number)}</div>
                            </td>
                            <td class="td-cell-compact text-huly-content text-xs">
                                <div>{row.field}</div>
                                <div class="text-[0.6875rem] font-normal text-huly-muted">{row.revision_id.to_string()}</div>
                            </td>
                            <td class="td-cell-compact text-huly-content text-xs font-mono">{pretty_value(&row.old_value)}</td>
                            <td class="td-cell-compact text-huly-content text-xs font-mono">{pretty_value(&row.new_value)}</td>
                            <td class="td-cell-compact text-huly-content text-xs">{row.reason}</td>
                        </tr>
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }
}

fn render_assignment_history(rows: Vec<AssignmentHistoryRow>) -> impl IntoView {
    view! {
        <div class="overflow-x-auto">
            <table class="min-w-full divide-y divide-huly-divider">
                <caption class="sr-only">"Assignment history report"</caption>
                <thead class="table-head">
                    <tr>
                        <th class="th-cell-compact" scope="col">"Timestamp"</th>
                        <th class="th-cell-compact" scope="col">"User"</th>
                        <th class="th-cell-compact" scope="col">"Action"</th>
                        <th class="th-cell-compact" scope="col">"Resource"</th>
                        <th class="th-cell-compact" scope="col">"Project"</th>
                        <th class="th-cell-compact" scope="col">"Before"</th>
                        <th class="th-cell-compact" scope="col">"After"</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-huly-divider bg-huly-surface">
                    {rows.into_iter().map(|row| view! {
                        <tr class="table-row-hover">
                            <td class="td-cell-compact text-huly-content text-xs">
                                <div>{format_timestamp(&row.timestamp)}</div>
                                <div class="text-[0.6875rem] font-normal text-huly-muted">{row.audit_id.to_string()}</div>
                            </td>
                            <td class="td-cell-compact text-huly-content">
                                <div>{row.user_name.unwrap_or_else(|| "Unknown".to_string())}</div>
                                <div class="text-[0.6875rem] font-normal text-huly-muted">{row.user_id.map(|u| u.to_string()).unwrap_or_default()}</div>
                            </td>
                            <td class="td-cell-compact text-huly-content text-xs">
                                <div>{row.action}</div>
                                <div class="text-[0.6875rem] font-normal text-huly-muted">{row.allocation_id.map(|id| id.to_string()).unwrap_or_default()}</div>
                            </td>
                            <td class="td-cell-compact text-huly-content text-xs">
                                {row.resource_name.unwrap_or_else(|| "—".to_string())}
                                <div class="text-[0.6875rem] font-normal text-huly-muted">{row.resource_id.map(|r| r.to_string()).unwrap_or_default()}</div>
                            </td>
                            <td class="td-cell-compact text-huly-content text-xs">
                                {row.project_name.unwrap_or_else(|| "—".to_string())}
                                <div class="text-[0.6875rem] font-normal text-huly-muted">{row.project_id.map(|p| p.to_string()).unwrap_or_default()}</div>
                            </td>
                            <td class="td-cell-compact text-huly-content text-xs">{row.before_summary}</td>
                            <td class="td-cell-compact text-huly-content text-xs">{row.after_summary}</td>
                        </tr>
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }
}

fn render_budget_modifications(rows: Vec<BudgetModificationRow>) -> impl IntoView {
    view! {
        <div class="overflow-x-auto">
            <table class="min-w-full divide-y divide-huly-divider">
                <caption class="sr-only">"Budget modifications report"</caption>
                <thead class="table-head">
                    <tr>
                        <th class="th-cell-compact" scope="col">"Timestamp"</th>
                        <th class="th-cell-compact" scope="col">"User"</th>
                        <th class="th-cell-compact" scope="col">"Action"</th>
                        <th class="th-cell-compact" scope="col">"Project"</th>
                        <th class="th-cell-compact" scope="col">"Before"</th>
                        <th class="th-cell-compact" scope="col">"After"</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-huly-divider bg-huly-surface">
                    {rows.into_iter().map(|row| view! {
                        <tr class="table-row-hover">
                            <td class="td-cell-compact text-huly-content text-xs">
                                <div>{format_timestamp(&row.timestamp)}</div>
                                <div class="text-[0.6875rem] font-normal text-huly-muted">{row.audit_id.to_string()}</div>
                            </td>
                            <td class="td-cell-compact text-huly-content">
                                <div>{row.user_name.unwrap_or_else(|| "Unknown".to_string())}</div>
                                <div class="text-[0.6875rem] font-normal text-huly-muted">{row.user_id.map(|u| u.to_string()).unwrap_or_default()}</div>
                            </td>
                            <td class="td-cell-compact text-huly-content text-xs">{row.action}</td>
                            <td class="td-cell-compact text-huly-content text-xs">
                                {row.project_name.unwrap_or_else(|| "—".to_string())}
                                <div class="text-[0.6875rem] font-normal text-huly-muted">{row.project_id.map(|p| p.to_string()).unwrap_or_default()}</div>
                            </td>
                            <td class="td-cell-compact text-huly-content text-xs">{row.before_summary}</td>
                            <td class="td-cell-compact text-huly-content text-xs">{row.after_summary}</td>
                        </tr>
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }
}

fn render_access_logs(rows: Vec<AccessLogRow>) -> impl IntoView {
    view! {
        <div class="overflow-x-auto">
            <table class="min-w-full divide-y divide-huly-divider">
                <caption class="sr-only">"Access logs report"</caption>
                <thead class="table-head">
                    <tr>
                        <th class="th-cell-compact" scope="col">"Timestamp"</th>
                        <th class="th-cell-compact" scope="col">"User"</th>
                        <th class="th-cell-compact" scope="col">"Action"</th>
                        <th class="th-cell-compact" scope="col">"Resource Accessed"</th>
                        <th class="th-cell-compact" scope="col">"Success/Failure"</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-huly-divider bg-huly-surface">
                    {rows.into_iter().map(|row| {
                        let status_class = if row.success { "text-positive-default" } else { "text-negative-default" };
                        let status_text = if row.success { "Success" } else { "Failure" };
                        view! {
                            <tr class="table-row-hover">
                                <td class="td-cell-compact text-huly-content text-xs">
                                    <div>{format_timestamp(&row.timestamp)}</div>
                                    <div class="text-[0.6875rem] font-normal text-huly-muted">{row.audit_id.to_string()}</div>
                                </td>
                                <td class="td-cell-compact text-huly-content">
                                    <div>{row.user_name.unwrap_or_else(|| "Unknown".to_string())}</div>
                                    <div class="text-[0.6875rem] font-normal text-huly-muted">{row.user_id.map(|u| u.to_string()).unwrap_or_default()}</div>
                                </td>
                                <td class="td-cell-compact text-huly-content text-xs">{row.action}</td>
                                <td class="td-cell-compact text-huly-content text-xs">
                                    <div>{row.resource_type}</div>
                                    <div class="text-[0.6875rem] font-normal text-huly-muted">{row.resource_id.map(|r| r.to_string()).unwrap_or_default()}</div>
                                </td>
                                <td class=format!("td-cell-compact text-xs {}", status_class)>
                                    <div>{status_text}</div>
                                    {row.reason.map(|r| view! { <div class="text-[0.6875rem] font-normal text-huly-muted">{r}</div> })}
                                </td>
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }
}
