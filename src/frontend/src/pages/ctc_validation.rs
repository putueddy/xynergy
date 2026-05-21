use crate::auth::{authenticated_get, use_auth};
use leptos::either::Either;
use leptos::prelude::*;
use leptos_router::hooks::*;
use serde::Deserialize;
use std::collections::HashSet;
use uuid::Uuid;

const PAGE_LIMIT: i64 = 100;
const MAX_SAMPLED_EMPLOYEE_IDS: usize = 200;

#[derive(Debug, Clone, Deserialize)]
struct ValidationMismatch {
    employee_id: Uuid,
    employee_name: String,
    field_name: String,
    xynergy_value: i64,
    payroll_value: i64,
    variance_amount: i64,
    status: String,
    #[serde(default)]
    bpjs_metadata: Option<BpjsMismatchMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
struct BpjsMismatchMetadata {
    risk_tier: i32,
    recalculated_value: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct ExcludedRecord {
    employee_id: Uuid,
    employee_name: String,
    reason: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ValidationReportData {
    start_date: String,
    end_date: String,
    total_compared: i64,
    total_discrepancies: i64,
    match_rate_pct: f64,
    excluded_count: i64,
    bpjs_error_count: i64,
    #[serde(default)]
    sampled: bool,
    #[serde(default)]
    payroll_coverage_pct: f64,
    mismatches: Vec<ValidationMismatch>,
    excluded: Vec<ExcludedRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReportFilters {
    start_date: String,
    end_date: String,
    sample_employee_ids: String,
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

fn report_filters(
    start_date: String,
    end_date: String,
    sample_employee_ids: String,
) -> ReportFilters {
    ReportFilters {
        start_date: start_date.trim().to_string(),
        end_date: end_date.trim().to_string(),
        sample_employee_ids,
    }
}

fn validate_report_filters(filters: &ReportFilters) -> Result<(), String> {
    if filters.start_date.is_empty() || filters.end_date.is_empty() {
        return Err("Start date and end date are required".to_string());
    }
    if filters.start_date.as_str() > filters.end_date.as_str() {
        return Err("End date must be greater than or equal to start date".to_string());
    }

    Ok(())
}

fn plain_text_preview(body: &str) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() || trimmed.starts_with('{') || trimmed.starts_with('[') {
        return None;
    }

    let preview = trimmed.chars().take(240).collect::<String>();
    Some(if trimmed.chars().count() > 240 {
        format!("{}...", preview)
    } else {
        preview
    })
}

fn validation_error_message(status: reqwest::StatusCode, body: &str) -> String {
    let parsed_message = serde_json::from_str::<ApiErrorEnvelope>(body)
        .ok()
        .and_then(|envelope| {
            envelope
                .error
                .and_then(|error| error.message)
                .or(envelope.message)
        })
        .map(|message| message.trim().to_string())
        .filter(|message| !message.is_empty());

    let message = parsed_message
        .or_else(|| plain_text_preview(body))
        .unwrap_or_else(|| {
            "The validation report could not be loaded. Check the selected filters and try again."
                .to_string()
        });

    format!(
        "Failed to fetch validation report ({}): {}",
        status.as_u16(),
        message
    )
}

fn format_idr(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut reversed_grouped = String::new();

    for (idx, ch) in digits.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            reversed_grouped.push('.');
        }
        reversed_grouped.push(ch);
    }

    let mut grouped: String = reversed_grouped.chars().rev().collect();
    if value < 0 {
        grouped = format!("-{}", grouped);
    }

    format!("Rp {}", grouped)
}

fn titleize_token(token: &str) -> String {
    let mut chars = token.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str().to_lowercase()),
        None => String::new(),
    }
}

fn field_label(field: &str) -> String {
    match field {
        "base_salary" => "Base Salary".to_string(),
        "hra_allowance" => "HRA Allowance".to_string(),
        "medical_allowance" => "Medical Allowance".to_string(),
        "transport_allowance" => "Transport Allowance".to_string(),
        "meal_allowance" => "Meal Allowance".to_string(),
        "bpjs_kesehatan_employer" => "BPJS Kesehatan (Employer)".to_string(),
        "bpjs_ketenagakerjaan_employer" => "BPJS Ketenagakerjaan (Employer)".to_string(),
        _ => field
            .split('_')
            .filter(|part| !part.is_empty())
            .map(titleize_token)
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn status_label(status: &str) -> String {
    match status {
        "BPJS_REGULATION_ERROR" => "BPJS Regulation Error".to_string(),
        "MISSING_IN_XYNERGY" => "Missing in Xynergy".to_string(),
        "DISCREPANCY" => "Discrepancy".to_string(),
        _ => status
            .split('_')
            .filter(|part| !part.is_empty())
            .map(titleize_token)
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn normalize_sample_employee_ids(raw: &str) -> Result<Option<String>, String> {
    let mut ids = Vec::<Uuid>::new();
    for raw_id in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let id =
            Uuid::parse_str(raw_id).map_err(|_| format!("Invalid employee ID UUID: {}", raw_id))?;
        if !ids.contains(&id) {
            ids.push(id);
        }
        if ids.len() > MAX_SAMPLED_EMPLOYEE_IDS {
            return Err(format!(
                "Employee sample may contain at most {} distinct IDs",
                MAX_SAMPLED_EMPLOYEE_IDS
            ));
        }
    }

    if ids.is_empty() {
        return Ok(None);
    }

    Ok(Some(
        ids.into_iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(","),
    ))
}

async fn fetch_validation_report(
    start_date: &str,
    end_date: &str,
    limit: i64,
    offset: i64,
    employee_ids_csv: Option<&str>,
) -> Result<ValidationReportData, String> {
    let mut url = format!(
        "/api/v1/ctc/validation-report?start_date={}&end_date={}&limit={}&offset={}",
        start_date, end_date, limit, offset
    );
    if let Some(ids) = employee_ids_csv {
        if let Some(cleaned) = normalize_sample_employee_ids(ids)? {
            let encoded = js_sys::encode_uri_component(&cleaned)
                .as_string()
                .unwrap_or(cleaned);
            url.push_str("&employee_ids=");
            url.push_str(&encoded);
        }
    }

    let response = authenticated_get(&url)
        .await
        .map_err(|e| format!("Failed to fetch validation report: {}", e))?;

    if response.status().is_success() {
        response
            .json::<ValidationReportData>()
            .await
            .map_err(|e| format!("Failed to parse validation report: {}", e))
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(validation_error_message(status, &body))
    }
}

#[component]
pub fn CtcValidationPage() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();

    let user_role = Signal::derive(move || auth.user.get().map(|u| u.role).unwrap_or_default());
    let is_admin = Signal::derive(move || user_role.get() == "admin");
    let is_finance = Signal::derive(move || user_role.get() == "finance");
    let can_access = Signal::derive(move || is_finance.get() || is_admin.get());
    let auth_hydrating =
        Signal::derive(move || auth.token.get().is_some() && auth.user.get().is_none());

    let now = chrono::Local::now().date_naive();
    let default_start = format!("{}-01-01", now.format("%Y"));
    let default_end = format!("{}-12-31", now.format("%Y"));

    let (start_date, set_start_date) = signal(default_start);
    let (end_date, set_end_date) = signal(default_end);
    let (report, set_report) = signal(Option::<ValidationReportData>::None);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (expanded_rows, set_expanded_rows) = signal(HashSet::<String>::new());
    let (page_offset, set_page_offset) = signal(0_i64);
    let (sample_panel_open, set_sample_panel_open) = signal(false);
    let (sample_employee_ids, set_sample_employee_ids) = signal(String::new());
    let (applied_filters, set_applied_filters) = signal(Option::<ReportFilters>::None);
    let filters_dirty = Signal::derive(move || {
        applied_filters
            .get()
            .map(|applied| {
                applied
                    != report_filters(start_date.get(), end_date.get(), sample_employee_ids.get())
            })
            .unwrap_or(false)
    });

    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            if !auth.is_authenticated.get() {
                navigate("/login", Default::default());
            }
        });
    }

    let run_report = move |offset: i64| {
        set_error.set(None);
        set_expanded_rows.set(HashSet::new());
        let filters = if offset == 0 {
            report_filters(start_date.get(), end_date.get(), sample_employee_ids.get())
        } else {
            match applied_filters.get() {
                Some(filters) => filters,
                None => {
                    set_error.set(Some(
                        "Run the report before loading additional mismatch pages.".to_string(),
                    ));
                    return;
                }
            }
        };

        if let Err(message) = validate_report_filters(&filters) {
            set_error.set(Some(message));
            if offset == 0 {
                set_report.set(None);
                set_applied_filters.set(None);
                set_page_offset.set(0);
            }
            return;
        }
        if offset == 0 {
            set_report.set(None);
            set_applied_filters.set(None);
            set_page_offset.set(0);
        }

        set_loading.set(true);
        leptos::task::spawn_local(async move {
            let sample_arg = if filters.sample_employee_ids.trim().is_empty() {
                None
            } else {
                Some(filters.sample_employee_ids.as_str())
            };
            match fetch_validation_report(
                &filters.start_date,
                &filters.end_date,
                PAGE_LIMIT,
                offset,
                sample_arg,
            )
            .await
            {
                Ok(data) => {
                    set_applied_filters.set(Some(filters));
                    set_page_offset.set(offset);
                    set_report.set(Some(data));
                    set_expanded_rows.set(HashSet::new());
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

    view! {
        <div class="h-full">
            <div class="page-container fade-in">
                <div class="space-y-4">
                    <div class="page-header">
                        <h1 class="text-xl font-semibold text-huly-caption">"CTC Validation"</h1>
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
                                    "Access denied. CTC Validation is available to Finance and Admin roles only."
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
                                                <label class="label text-xs" for="val-start">"Start Date"</label>
                                                <input
                                                    id="val-start"
                                                    type="date"
                                                    class="input mt-1 w-40"
                                                    prop:value=start_date
                                                    disabled=move || loading.get()
                                                    on:input=move |ev| set_start_date.set(event_target_value(&ev))
                                                />
                                            </div>
                                            <div>
                                                <label class="label text-xs" for="val-end">"End Date"</label>
                                                <input
                                                    id="val-end"
                                                    type="date"
                                                    class="input mt-1 w-40"
                                                    prop:value=end_date
                                                    disabled=move || loading.get()
                                                    on:input=move |ev| set_end_date.set(event_target_value(&ev))
                                                />
                                            </div>
                                            <button
                                                class="btn-primary btn-press text-sm"
                                                on:click=handle_run
                                                disabled=move || loading.get()
                                                aria-label="Run validation report"
                                                title="Compares the latest payroll baseline per employee in the selected period."
                                            >
                                                {move || if loading.get() { "Running..." } else { "Run Report" }}
                                            </button>
                                        </div>
                                        <div class="mt-3">
                                            <button
                                                type="button"
                                                class="text-xs font-medium text-huly-secondary underline-offset-2 hover:underline rounded focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-400"
                                                on:click=move |_| set_sample_panel_open.update(|v| *v = !*v)
                                                aria-expanded=move || sample_panel_open.get()
                                                aria-controls="sample-employee-panel"
                                                disabled=move || loading.get()
                                            >
                                                {move || if sample_panel_open.get() {
                                                    "Hide sample employees"
                                                } else {
                                                    "Sample specific employees (optional)"
                                                }}
                                            </button>
                                            {move || {
                                                let active = !sample_employee_ids.get().trim().is_empty() && !sample_panel_open.get();
                                                active.then(|| view! {
                                                    <span class="ml-2 inline-flex items-center rounded-full border border-huly-divider bg-huly-surface-2 px-2 py-0.5 text-xs font-medium text-huly-caption">
                                                        "Sample filter active"
                                                    </span>
                                                })
                                            }}
                                            {move || if sample_panel_open.get() {
                                                Some(view! {
                                                    <div id="sample-employee-panel" class="mt-2 space-y-2">
                                                        <label class="label text-xs" for="sample-ids">"Employee IDs (comma-separated UUIDs)"</label>
                                                        <textarea
                                                            id="sample-ids"
                                                            rows="2"
                                                            class="input w-full text-xs font-mono"
                                                            placeholder="e.g. 11111111-2222-3333-4444-555555555555,aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
                                                            prop:value=sample_employee_ids
                                                            disabled=move || loading.get()
                                                            on:input=move |ev| set_sample_employee_ids.set(event_target_value(&ev))
                                                        ></textarea>
                                                        <p class="text-xs text-huly-muted">
                                                            "Leave blank to validate every employee in the date range. Sampled runs are logged in audit history."
                                                        </p>
                                                    </div>
                                                })
                                            } else {
                                                None
                                            }}
                                        </div>
                                        <p class="mt-3 text-xs text-huly-muted">
                                            "Compares each employee's Xynergy CTC against the latest payroll baseline in the selected period. BPJS calculations are validated against current regulation rates."
                                        </p>
                                    </div>

                                    {move || {
                                        let current = report.get();
                                        match current {
                                            Some(data) => {
                                                let rate_color = if data.match_rate_pct >= 99.0 {
                                                    "text-positive-default"
                                                } else if data.match_rate_pct >= 90.0 {
                                                    "text-huly-caption"
                                                } else {
                                                    "text-negative-default"
                                                };
                                                let match_rate_str = format!("{:.2}%", data.match_rate_pct);
                                                let range_label = format!(
                                                    "Showing comparison from {} to {}.",
                                                    data.start_date, data.end_date
                                                );

                                                let mismatches = data.mismatches.clone();
                                                let excluded = data.excluded.clone();
                                                let current_offset = page_offset.get();
                                                let filter_controls_dirty = filters_dirty.get();
                                                let has_previous_page = current_offset > 0;
                                                let has_next_page = current_offset + (mismatches.len() as i64) < data.total_discrepancies;

                                                let coverage_pct = data.payroll_coverage_pct;
                                                let coverage_str = format!("{:.2}%", coverage_pct);
                                                let coverage_color = if coverage_pct >= 90.0 {
                                                    "text-positive-default"
                                                } else if coverage_pct > 0.0 {
                                                    "text-negative-default"
                                                } else {
                                                    "text-huly-caption"
                                                };
                                                let sampled_pill = if data.sampled {
                                                    let count = data.total_compared + data.excluded_count;
                                                    Some(view! {
                                                        <span class="ml-2 inline-flex items-center rounded-full bg-huly-surface-2 px-2 py-0.5 text-xs font-medium text-huly-caption border border-huly-divider">
                                                            {format!("Sampled run - {} employees", count)}
                                                        </span>
                                                    })
                                                } else {
                                                    None
                                                };

                                                Some(view! {
                                                    <div class="panel p-4">
                                                        <div class="flex items-center justify-between mb-4">
                                                            <h2 class="section-header">"Report Summary"</h2>
                                                            {sampled_pill}
                                                        </div>
                                                        <p class="mb-4 text-xs text-huly-muted">{range_label}</p>

                                                        <div class="grid grid-cols-2 md:grid-cols-6 gap-4 mb-2">
                                                            <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                                <p class="text-sm text-huly-muted mb-1">"Match Rate"</p>
                                                                <p class=format!("text-xl font-bold {}", rate_color)>{match_rate_str}</p>
                                                            </div>
                                                            <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                                <p class="text-sm text-huly-muted mb-1">"Compared Records"</p>
                                                                <p class="text-xl font-bold text-huly-caption">{data.total_compared.to_string()}</p>
                                                            </div>
                                                            <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                                <p class="text-sm text-huly-muted mb-1">"Discrepancies"</p>
                                                                <p class="text-xl font-bold text-negative-default">{data.total_discrepancies.to_string()}</p>
                                                            </div>
                                                            <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                                <p class="text-sm text-huly-muted mb-1">"Excluded"</p>
                                                                <p class="text-xl font-bold text-huly-caption">{data.excluded_count.to_string()}</p>
                                                            </div>
                                                            <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                                <p class="text-sm text-huly-muted mb-1">"BPJS Errors"</p>
                                                                <p class="text-xl font-bold text-negative-default">{data.bpjs_error_count.to_string()}</p>
                                                            </div>
                                                            <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                                <p class="text-sm text-huly-muted mb-1">"Payroll Coverage"</p>
                                                                <p class=format!("text-xl font-bold {}", coverage_color)>{coverage_str}</p>
                                                            </div>
                                                        </div>
                                                    </div>

                                                    <div class="panel overflow-hidden" aria-busy=move || loading.get()>
                                                        <div class="toolbar flex items-center justify-between gap-3">
                                                            <h2 class="text-sm font-semibold text-huly-secondary uppercase tracking-wider">"Mismatches"</h2>
                                                            <div class="flex items-center gap-2">
                                                                {move || loading.get().then(|| view! {
                                                                    <span class="text-xs text-huly-muted" role="status" aria-live="polite">"Loading..."</span>
                                                                })}
                                                                {filter_controls_dirty.then(|| view! {
                                                                    <span class="text-xs text-huly-muted" role="status" aria-live="polite">"Run Report to apply filter changes"</span>
                                                                })}
                                                                <span class="text-xs text-huly-muted">
                                                                    {if mismatches.is_empty() {
                                                                        "0 rows".to_string()
                                                                    } else {
                                                                        format!("Rows {}-{}", current_offset + 1, current_offset + mismatches.len() as i64)
                                                                    }}
                                                                </span>
                                                                <button
                                                                    type="button"
                                                                    class="btn-secondary btn-press text-xs"
                                                                    disabled=move || loading.get() || filter_controls_dirty || !has_previous_page
                                                                    on:click=move |_| {
                                                                        let next_offset = (page_offset.get() - PAGE_LIMIT).max(0);
                                                                        run_report(next_offset);
                                                                    }
                                                                >
                                                                    "Previous"
                                                                </button>
                                                                <button
                                                                    type="button"
                                                                    class="btn-secondary btn-press text-xs"
                                                                    disabled=move || loading.get() || filter_controls_dirty || !has_next_page
                                                                    on:click=move |_| {
                                                                        run_report(page_offset.get() + PAGE_LIMIT);
                                                                    }
                                                                >
                                                                    "Next"
                                                                </button>
                                                            </div>
                                                        </div>
                                                        {if mismatches.is_empty() {
                                                            Either::Left(view! {
                                                                <div class="empty-state py-10">
                                                                    <p class="text-huly-muted text-sm">
                                                                        {if data.total_discrepancies > 0 {
                                                                            "No mismatch rows on this page."
                                                                        } else if data.excluded_count > 0 || data.payroll_coverage_pct < 90.0 {
                                                                            "No mismatches returned, but excluded records or payroll coverage need review."
                                                                        } else {
                                                                            "No discrepancies detected for the selected range."
                                                                        }}
                                                                    </p>
                                                                </div>
                                                            })
                                                        } else {
                                                            Either::Right(view! {
                                                                <div class="overflow-x-auto">
                                                                    <table class="min-w-full divide-y divide-huly-divider">
                                                                        <caption class="sr-only">"CTC validation mismatch details"</caption>
                                                                        <thead class="table-head">
                                                                            <tr>
                                                                                <th class="th-cell-compact" scope="col">"Employee"</th>
                                                                                <th class="th-cell-compact" scope="col">"Field"</th>
                                                                                <th class="th-cell-compact text-right" scope="col">"Xynergy"</th>
                                                                                <th class="th-cell-compact text-right" scope="col">"Payroll"</th>
                                                                                <th class="th-cell-compact text-right" scope="col">"Variance"</th>
                                                                                <th class="th-cell-compact" scope="col">"Status"</th>
                                                                                <th class="th-cell-compact text-center" scope="col">"Details"</th>
                                                                            </tr>
                                                                        </thead>
                                                                        <tbody class="divide-y divide-huly-divider bg-huly-surface">
                                                                            {mismatches.into_iter().enumerate().map(|(idx, m)| {
                                                                                let row_key = format!("{}-{}-{}", current_offset + idx as i64, m.employee_id, m.field_name);
                                                                                let row_key_for_button = row_key.clone();
                                                                                let row_key_for_check = row_key.clone();
                                                                                let row_key_for_aria = row_key.clone();
                                                                                let row_key_for_detail = row_key.clone();
                                                                                let detail_id = format!("detail-{}", row_key);
                                                                                let detail_id_for_aria = detail_id.clone();
                                                                                let detail_id_for_row = detail_id.clone();
                                                                                let status_color = match m.status.as_str() {
                                                                                    "BPJS_REGULATION_ERROR" => "text-negative-default",
                                                                                    "MISSING_IN_XYNERGY" => "text-huly-caption",
                                                                                    _ => "text-huly-caption",
                                                                                };
                                                                                let bpjs_metadata = m.bpjs_metadata.clone();
                                                                                let employee_name_for_aria = m.employee_name.clone();
                                                                                let field_label_text = field_label(&m.field_name);
                                                                                let field_label_for_aria = field_label_text.clone();
                                                                                let field_label_for_detail = field_label_text.clone();
                                                                                let status_text = status_label(&m.status);

                                                                                view! {
                                                                                    <tr class="table-row-hover">
                                                                                        <td class="td-cell-compact font-medium text-huly-caption">{m.employee_name.clone()}</td>
                                                                                        <td class="td-cell-compact text-huly-content">{field_label_text.clone()}</td>
                                                                                        <td class="td-cell-compact text-right text-huly-content">{format_idr(m.xynergy_value)}</td>
                                                                                        <td class="td-cell-compact text-right text-huly-content">{format_idr(m.payroll_value)}</td>
                                                                                        <td class="td-cell-compact text-right font-medium text-negative-default">{format_idr(m.variance_amount)}</td>
                                                                                        <td class=format!("td-cell-compact text-xs {}", status_color)>{status_text}</td>
                                                                                        <td class="td-cell-compact text-center">
                                                                                            <button
                                                                                                type="button"
                                                                                                class="inline-flex min-h-[44px] min-w-[44px] items-center justify-center rounded-md px-2 text-xs font-medium text-huly-caption hover:bg-huly-surface-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-400"
                                                                                                on:click=move |_| {
                                                                                                    let key = row_key_for_button.clone();
                                                                                                    set_expanded_rows.update(|set| {
                                                                                                        if set.contains(&key) {
                                                                                                            set.remove(&key);
                                                                                                        } else {
                                                                                                            set.insert(key);
                                                                                                        }
                                                                                                    });
                                                                                                }
                                                                                                aria-expanded=move || expanded_rows.get().contains(&row_key_for_check)
                                                                                                aria-controls=detail_id_for_aria.clone()
                                                                                                aria-label=format!("Toggle detail for {} - {}", employee_name_for_aria, field_label_for_aria)
                                                                                            >
                                                                                                {move || if expanded_rows.get().contains(&row_key_for_aria) { "Hide" } else { "Show" }}
                                                                                            </button>
                                                                                        </td>
                                                                                    </tr>
                                                                                    {move || {
                                                                                        if expanded_rows.get().contains(&row_key_for_detail) {
                                                                                            let bpjs_info = bpjs_metadata.as_ref().map(|meta| {
                                                                                                format!(
                                                                                                    "Risk tier {}: regulation-recalculated value is {}",
                                                                                                    meta.risk_tier,
                                                                                                    format_idr(meta.recalculated_value)
                                                                                                )
                                                                                            });
                                                                                            Some(view! {
                                                                                                <tr class="bg-huly-surface-2" id=detail_id_for_row.clone()>
                                                                                                    <td class="td-cell-compact text-xs text-huly-muted" colspan="7">
                                                                                                        <div class="space-y-1 py-1">
                                                                                                            <p>{format!("Employee ID: {}", m.employee_id)}</p>
                                                                                                            <p>{format!("Field: {}", field_label_for_detail)}</p>
                                                                                                            <p>{format!("Xynergy value: {}", format_idr(m.xynergy_value))}</p>
                                                                                                            <p>{format!("Payroll value: {}", format_idr(m.payroll_value))}</p>
                                                                                                            <p>{format!("Variance: {}", format_idr(m.variance_amount))}</p>
                                                                                                            {bpjs_info.map(|info| view! { <p>{info}</p> })}
                                                                                                        </div>
                                                                                                    </td>
                                                                                                </tr>
                                                                                            })
                                                                                        } else {
                                                                                            None
                                                                                        }
                                                                                    }}
                                                                                }
                                                                            }).collect_view()}
                                                                        </tbody>
                                                                    </table>
                                                                </div>
                                                            })
                                                        }}
                                                    </div>

                                                    {if excluded.is_empty() {
                                                        None
                                                    } else {
                                                        Some(view! {
                                                            <div class="panel overflow-hidden">
                                                                <div class="toolbar flex items-center justify-between gap-3">
                                                                    <h2 class="text-sm font-semibold text-huly-secondary uppercase tracking-wider">"Excluded Records"</h2>
                                                                    <span class="text-xs text-huly-muted">
                                                                        {if data.excluded_count > excluded.len() as i64 {
                                                                            format!("Showing {} of {} rows", excluded.len(), data.excluded_count)
                                                                        } else {
                                                                            format!("{} rows", excluded.len())
                                                                        }}
                                                                    </span>
                                                                </div>
                                                                <div class="overflow-x-auto">
                                                                    <table class="min-w-full divide-y divide-huly-divider">
                                                                        <caption class="sr-only">"CTC validation excluded records"</caption>
                                                                        <thead class="table-head">
                                                                            <tr>
                                                                                <th class="th-cell-compact" scope="col">"Employee"</th>
                                                                                <th class="th-cell-compact" scope="col">"Reason"</th>
                                                                            </tr>
                                                                        </thead>
                                                                        <tbody class="divide-y divide-huly-divider bg-huly-surface">
                                                                            {excluded.into_iter().map(|row| view! {
                                                                                <tr class="table-row-hover">
                                                                                    <td class="td-cell-compact font-medium text-huly-caption">
                                                                                        <div>{row.employee_name}</div>
                                                                                        <div class="text-[0.6875rem] font-normal text-huly-muted">{row.employee_id.to_string()}</div>
                                                                                    </td>
                                                                                    <td class="td-cell-compact text-huly-content text-xs">{row.reason}</td>
                                                                                </tr>
                                                                            }).collect_view()}
                                                                        </tbody>
                                                                    </table>
                                                                </div>
                                                            </div>
                                                        })
                                                    }}
                                                }.into_any())
                                            }
                                            None => {
                                                if loading.get() {
                                                    Some(view! {
                                                        <div class="panel p-4" role="status" aria-live="polite">
                                                            <p class="text-sm text-huly-muted">"Loading validation report..."</p>
                                                        </div>
                                                    }.into_any())
                                                } else {
                                                    Some(view! {
                                                        <div class="panel p-4">
                                                            <p class="text-sm text-huly-muted">
                                                                "Select a date range and click \"Run Report\" to compare Xynergy CTC data against payroll records."
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
