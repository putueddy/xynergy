use crate::auth::{
    auth_token, authenticated_get, clear_auth_storage, use_auth, validate_token, AuthContext,
};
use leptos::either::{Either, EitherOf3};
use leptos::prelude::*;
use leptos_router::hooks::*;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Debug)]
struct DepartmentRow {
    row_key: String,
    department: String,
    total_employees: i64,
    with_ctc: i64,
    missing_ctc: i64,
    completion_pct: f64,
}

#[derive(Clone, Debug)]
struct MissingEmployee {
    id: String,
    name: String,
    department: String,
}

#[derive(Clone, Debug)]
struct ComplianceRow {
    resource_id: String,
    name: String,
    stored_bpjs_kes: i64,
    expected_bpjs_kes: i64,
    stored_bpjs_kt: i64,
    expected_bpjs_kt: i64,
    risk_tier: i64,
    status: String,
    variance_amount: i64,
}

#[derive(Clone, Debug)]
struct TrendPoint {
    month: String,
    total_employees: i64,
    total_with_ctc: i64,
    total_missing: i64,
    completion_pct: f64,
}

#[derive(Clone, Debug, Default)]
struct CompletenessSummary {
    departments: Vec<DepartmentRow>,
    total_employees: i64,
    total_with_ctc: i64,
    total_missing: i64,
    overall_completion_pct: f64,
    trend: Vec<TrendPoint>,
}

fn current_access_token(auth: &AuthContext) -> Option<String> {
    if let Ok(stored) = auth_token() {
        if auth.token.get() != Some(stored.clone()) {
            auth.token.set(Some(stored.clone()));
        }
        return Some(stored);
    }
    auth.token.get()
}

fn value_to_i64(value: &Value) -> Option<i64> {
    if let Some(v) = value.as_i64() {
        return Some(v);
    }
    value.as_str()?.parse::<i64>().ok()
}

fn value_to_f64(value: &Value) -> Option<f64> {
    if let Some(v) = value.as_f64() {
        return v.is_finite().then_some(v);
    }
    if let Some(v) = value.as_i64() {
        return Some(v as f64);
    }
    value
        .as_str()?
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

fn valid_resource_id(id: &str) -> bool {
    Uuid::parse_str(id).is_ok()
}

fn percent_from_totals(total: i64, with_ctc: i64) -> f64 {
    if total <= 0 {
        0.0
    } else {
        (with_ctc.max(0).min(total) as f64 / total as f64) * 100.0
    }
}

/// Parse the backend `CompletenessReport` shape into the local summary.
/// Reads the current top-level field names (`total_with_ctc`, `total_missing`,
/// `overall_completion_pct`) rather than the pre-Story-6.5 names. Defaults
/// every numeric to zero so partial / older payloads do not crash the UI.
fn parse_completeness_summary(body: &Value) -> CompletenessSummary {
    let total_employees = body
        .get("total_employees")
        .and_then(value_to_i64)
        .unwrap_or(0);
    let total_with_ctc = body
        .get("total_with_ctc")
        .and_then(value_to_i64)
        .unwrap_or(0);
    // Prefer the canonical `total_missing` from the backend, but tolerate a
    // missing field by deriving it locally from totals.
    let total_missing = body
        .get("total_missing")
        .and_then(value_to_i64)
        .unwrap_or_else(|| (total_employees - total_with_ctc).max(0));
    let overall_completion_pct = body
        .get("overall_completion_pct")
        .and_then(value_to_f64)
        .unwrap_or_else(|| {
            if total_employees > 0 {
                (total_with_ctc as f64 / total_employees as f64) * 100.0
            } else {
                0.0
            }
        });

    let departments = body
        .get("departments")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(index, d)| {
            let department_id = d
                .get("department_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let department = d
                .get("department")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let total_employees = d.get("total_employees").and_then(value_to_i64).unwrap_or(0);
            let with_ctc = d.get("with_ctc").and_then(value_to_i64).unwrap_or(0);
            let missing_ctc = d
                .get("missing_ctc")
                .and_then(value_to_i64)
                .unwrap_or_else(|| (total_employees - with_ctc).max(0));
            DepartmentRow {
                row_key: if department_id.is_empty() {
                    format!("name:{}:{}", department, index)
                } else {
                    department_id.clone()
                },
                department,
                total_employees,
                with_ctc,
                missing_ctc,
                completion_pct: d
                    .get("completion_pct")
                    .and_then(value_to_f64)
                    .unwrap_or_else(|| percent_from_totals(total_employees, with_ctc)),
            }
        })
        .collect();

    let trend = body
        .get("trend")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|t| {
            let total_employees = t.get("total_employees").and_then(value_to_i64).unwrap_or(0);
            let total_with_ctc = t.get("total_with_ctc").and_then(value_to_i64).unwrap_or(0);
            let total_missing = t
                .get("total_missing")
                .and_then(value_to_i64)
                .unwrap_or_else(|| (total_employees - total_with_ctc).max(0));
            TrendPoint {
                month: t
                    .get("month")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                total_employees,
                total_with_ctc,
                total_missing,
                completion_pct: t
                    .get("completion_pct")
                    .and_then(value_to_f64)
                    .unwrap_or_else(|| percent_from_totals(total_employees, total_with_ctc)),
            }
        })
        .collect();

    CompletenessSummary {
        departments,
        total_employees,
        total_with_ctc,
        total_missing,
        overall_completion_pct,
        trend,
    }
}

async fn fetch_completeness(department_id: Option<String>) -> Result<CompletenessSummary, String> {
    let url = match department_id {
        Some(ref id) if !id.is_empty() => {
            format!("/api/v1/ctc/completeness?department_id={}", id)
        }
        _ => "/api/v1/ctc/completeness".to_string(),
    };

    let response = authenticated_get(&url)
        .await
        .map_err(|e| format!("Failed to fetch completeness: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch completeness: {}",
            response.status()
        ));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse completeness response: {}", e))?;

    Ok(parse_completeness_summary(&body))
}

async fn fetch_missing_employees(
    department_id: Option<String>,
) -> Result<Vec<MissingEmployee>, String> {
    let url = match department_id {
        Some(ref id) if !id.is_empty() => {
            format!("/api/v1/ctc/completeness/missing?department_id={}", id)
        }
        _ => "/api/v1/ctc/completeness/missing".to_string(),
    };

    let response = authenticated_get(&url)
        .await
        .map_err(|e| format!("Failed to fetch missing employees: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch missing employees: {}",
            response.status()
        ));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse missing employees: {}", e))?;

    let arr = body.as_array().cloned().unwrap_or_else(|| {
        body.get("missing_employees")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    });

    let mut employees = Vec::new();
    for e in arr {
        employees.push(MissingEmployee {
            id: e
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            name: e
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            department: e
                .get("department")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }

    Ok(employees)
}

async fn fetch_departments_list() -> Result<Vec<(String, String)>, String> {
    let response = authenticated_get("/api/v1/departments")
        .await
        .map_err(|e| format!("Failed to fetch departments: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch departments: {}",
            response.status()
        ));
    }

    let values: Vec<Value> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse departments: {}", e))?;

    Ok(values
        .into_iter()
        .filter_map(|v| {
            Some((
                v.get("id")?.as_str()?.to_string(),
                v.get("name")?.as_str()?.to_string(),
            ))
        })
        .collect())
}

async fn fetch_compliance_report(
    start_date: &str,
    end_date: &str,
) -> Result<(Vec<ComplianceRow>, i64, i64, i64, f64), String> {
    let response = authenticated_get(&format!(
        "/api/v1/ctc/compliance-report?start_date={}&end_date={}",
        start_date, end_date
    ))
    .await
    .map_err(|e| format!("Failed to fetch compliance report: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch compliance report: {}",
            response.status()
        ));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse compliance response: {}", e))?;

    let total_validated = body
        .get("total_validated")
        .and_then(value_to_i64)
        .unwrap_or(0);
    let passed = body.get("passed").and_then(value_to_i64).unwrap_or(0);
    let discrepancies = body
        .get("discrepancies")
        .and_then(value_to_i64)
        .unwrap_or(0);
    let compliance_rate = body
        .get("compliance_rate")
        .and_then(value_to_f64)
        .unwrap_or(0.0);

    let res = body
        .get("results")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut rows = Vec::new();
    for r in res {
        rows.push(ComplianceRow {
            resource_id: r
                .get("resource_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            name: r
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            stored_bpjs_kes: r.get("stored_bpjs_kes").and_then(value_to_i64).unwrap_or(0),
            expected_bpjs_kes: r
                .get("expected_bpjs_kes")
                .and_then(value_to_i64)
                .unwrap_or(0),
            stored_bpjs_kt: r.get("stored_bpjs_kt").and_then(value_to_i64).unwrap_or(0),
            expected_bpjs_kt: r
                .get("expected_bpjs_kt")
                .and_then(value_to_i64)
                .unwrap_or(0),
            risk_tier: r.get("risk_tier").and_then(value_to_i64).unwrap_or(0),
            status: r
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            variance_amount: r.get("variance_amount").and_then(value_to_i64).unwrap_or(0),
        });
    }

    Ok((
        rows,
        total_validated,
        passed,
        discrepancies,
        compliance_rate,
    ))
}

fn get_color_class(pct: f64) -> &'static str {
    if pct >= 90.0 {
        "text-positive-default"
    } else if pct >= 70.0 {
        "text-warning-default"
    } else {
        "text-negative-default"
    }
}

fn trend_bar_color(pct: f64) -> &'static str {
    if pct >= 90.0 {
        "background-color: var(--color-positive-default);"
    } else if pct >= 70.0 {
        "background-color: var(--color-warning-default);"
    } else {
        "background-color: var(--color-negative-default);"
    }
}

/// Clamp completion percentage into `[0, 100]` for bar widths.
pub fn bar_width_pct(value: f64) -> f64 {
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

#[component]
pub fn CtcCompleteness() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();
    let (auth_checked, set_auth_checked) = signal(false);
    let (auth_check_in_progress, set_auth_check_in_progress) = signal(false);

    let (_loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (success, set_success) = signal(None::<String>);

    let (summary, set_summary) = signal(CompletenessSummary::default());
    let (dept_filter, set_dept_filter) = signal(String::new());
    let (dept_options, set_dept_options) = signal(Vec::<(String, String)>::new());

    let (missing_employees, set_missing_employees) = signal(Vec::<MissingEmployee>::new());
    let (show_missing, set_show_missing) = signal(false);
    let (show_departments, set_show_departments) = signal(true);
    let (show_trend, set_show_trend) = signal(true);

    let (start_date, set_start_date) = signal(String::new());
    let (end_date, set_end_date) = signal(String::new());
    let (compliance_results, set_compliance_results) = signal(Vec::<ComplianceRow>::new());
    let (compliance_loading, set_compliance_loading) = signal(false);
    let (total_validated, set_total_validated) = signal(0i64);
    let (passed, set_passed) = signal(0i64);
    let (discrepancies, set_discrepancies) = signal(0i64);
    let (compliance_rate, set_compliance_rate) = signal(0.0f64);

    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            if !auth.is_authenticated.get() {
                navigate("/login", Default::default());
                return;
            }

            if let Some(user) = auth.user.get() {
                set_auth_checked.set(true);
                if user.role != "hr" && user.role != "department_head" && user.role != "finance" {
                    navigate("/dashboard", Default::default());
                }
                return;
            }

            if auth_check_in_progress.get() {
                return;
            }

            let token = match current_access_token(&auth) {
                Some(t) => t,
                None => {
                    navigate("/login", Default::default());
                    return;
                }
            };

            set_auth_check_in_progress.set(true);
            let navigate = navigate.clone();
            leptos::task::spawn_local(async move {
                match validate_token(token).await {
                    Ok(user) => {
                        auth.user.set(Some(user));
                    }
                    Err(_) => {
                        auth.user.set(None);
                        auth.token.set(None);
                        auth.refresh_token.set(None);
                        clear_auth_storage();
                        navigate("/login", Default::default());
                    }
                }
                set_auth_checked.set(true);
                set_auth_check_in_progress.set(false);
            });
        });
    }

    // HR + Department Head get full access; Finance is page-authorized for
    // the compliance section only and must not call the completeness route
    // (backend returns 403). is_authorized governs page access; the
    // completeness panel itself is gated on is_completeness_visible.
    let is_authorized = Signal::derive(move || {
        auth.user
            .get()
            .map(|u| u.role == "hr" || u.role == "department_head" || u.role == "finance")
            .unwrap_or(false)
    });
    let is_completeness_visible = Signal::derive(move || {
        auth.user
            .get()
            .map(|u| u.role == "hr" || u.role == "department_head")
            .unwrap_or(false)
    });
    let is_missing_list_enabled =
        Signal::derive(move || auth.user.get().map(|u| u.role == "hr").unwrap_or(false));
    let is_department_filter_visible =
        Signal::derive(move || auth.user.get().map(|u| u.role == "hr").unwrap_or(false));
    let is_hr_or_finance = Signal::derive(move || {
        auth.user
            .get()
            .map(|u| u.role == "hr" || u.role == "finance")
            .unwrap_or(false)
    });

    Effect::new(move |_| {
        if auth.token.get().is_some() {
            if !is_completeness_visible.get() {
                set_summary.set(CompletenessSummary::default());
                set_missing_employees.set(Vec::new());
                set_show_missing.set(false);
                return;
            }
            if is_department_filter_visible.get_untracked() {
                leptos::task::spawn_local(async move {
                    if let Ok(depts) = fetch_departments_list().await {
                        set_dept_options.set(depts);
                    }
                });
            }
            set_loading.set(true);
            let expected_filter = dept_filter.get_untracked();
            let filter = if expected_filter.is_empty() {
                None
            } else {
                Some(expected_filter.clone())
            };
            let expected_token = auth.token.get_untracked();
            leptos::task::spawn_local(async move {
                let still_current = || {
                    dept_filter.get_untracked() == expected_filter
                        && auth.token.get_untracked() == expected_token
                        && is_completeness_visible.get_untracked()
                };
                match fetch_completeness(filter).await {
                    Ok(s) => {
                        if still_current() {
                            set_summary.set(s);
                            set_error.set(None);
                        }
                    }
                    Err(e) => {
                        if still_current() {
                            set_summary.set(CompletenessSummary::default());
                            set_error.set(Some(e));
                        }
                    }
                }
                if still_current() {
                    set_loading.set(false);
                }
            });
        }
    });

    let fetch_missing = move |_| {
        if !is_missing_list_enabled.get_untracked() {
            return;
        }
        if show_missing.get() {
            set_show_missing.set(false);
            return;
        }

        set_loading.set(true);
        let expected_filter = dept_filter.get_untracked();
        let filter = if expected_filter.is_empty() {
            None
        } else {
            Some(expected_filter.clone())
        };
        let expected_token = auth.token.get_untracked();
        leptos::task::spawn_local(async move {
            let still_current = || {
                dept_filter.get_untracked() == expected_filter
                    && auth.token.get_untracked() == expected_token
                    && is_missing_list_enabled.get_untracked()
            };
            match fetch_missing_employees(filter).await {
                Ok(emps) => {
                    if still_current() {
                        set_missing_employees.set(emps);
                        set_show_missing.set(true);
                        set_error.set(None);
                    }
                }
                Err(e) => {
                    if still_current() {
                        set_missing_employees.set(Vec::new());
                        set_show_missing.set(false);
                        set_error.set(Some(e));
                    }
                }
            }
            if still_current() {
                set_loading.set(false);
            }
        });
    };

    let run_compliance_check = move |_| {
        set_error.set(None);
        set_success.set(None);

        let s_date = start_date.get();
        let e_date = end_date.get();

        if s_date.is_empty() || e_date.is_empty() {
            set_error.set(Some("Start Date and End Date are required".to_string()));
            return;
        }

        set_compliance_loading.set(true);
        leptos::task::spawn_local(async move {
            match fetch_compliance_report(&s_date, &e_date).await {
                Ok((results, total, pass, disc, rate)) => {
                    set_compliance_results.set(results);
                    set_total_validated.set(total);
                    set_passed.set(pass);
                    set_discrepancies.set(disc);
                    set_compliance_rate.set(rate);
                    set_success.set(Some("Compliance check completed".to_string()));
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_compliance_loading.set(false);
        });
    };

    view! {
        <div class="h-full">

            <div class="page-container fade-in">
                {move || {
                    if !auth_checked.get() {
                        return EitherOf3::A(view! {
                            <div class="alert-info">
                                "Checking access..."
                            </div>
                        });
                    }

                    if !is_authorized.get() {
                        return EitherOf3::B(view! {
                            <div class="alert-error">
                                "Access denied."
                            </div>
                        });
                    }

                    EitherOf3::C(view! {
                        <div class="space-y-4">
                            <div class="page-header">
                                <h1 class="text-xl font-semibold text-huly-caption">
                                    "CTC Completeness & Compliance"
                                </h1>
                            </div>

                            {move || error.get().map(|err| view! {
                                <div class="alert-error">{err}</div>
                            })}

                            {move || success.get().map(|msg| view! {
                                <div class="alert-success">{msg}</div>
                            })}

                            {move || is_completeness_visible.get().then(|| view! {
                                <div class="panel space-y-4">
                                    <div class="toolbar">
                                        <h2 class="section-header">"Completeness Dashboard"</h2>
                                        {move || is_department_filter_visible.get().then(|| view! {
                                            <div class="flex items-center gap-3">
                                                <label class="text-sm font-medium text-huly-content">"Filter by Department:"</label>
                                                <select
                                                    class="input"
                                                    prop:value=dept_filter
                                                    on:change=move |ev| {
                                                        let selected = event_target_value(&ev);
                                                        set_dept_filter.set(selected.clone());
                                                        set_show_missing.set(false);
                                                        set_missing_employees.set(Vec::new());
                                                        set_summary.set(CompletenessSummary::default());
                                                        set_loading.set(true);
                                                        let expected_token = auth.token.get_untracked();
                                                        leptos::task::spawn_local(async move {
                                                            let filter = if selected.is_empty() { None } else { Some(selected.clone()) };
                                                            let still_current = || {
                                                                dept_filter.get_untracked() == selected
                                                                    && auth.token.get_untracked() == expected_token
                                                                    && is_completeness_visible.get_untracked()
                                                            };
                                                            match fetch_completeness(filter).await {
                                                                Ok(s) => {
                                                                    if still_current() {
                                                                        set_summary.set(s);
                                                                        set_error.set(None);
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    if still_current() {
                                                                        set_summary.set(CompletenessSummary::default());
                                                                        set_error.set(Some(e));
                                                                    }
                                                                }
                                                            }
                                                            if still_current() {
                                                                set_loading.set(false);
                                                            }
                                                        });
                                                    }
                                                >
                                                    <option value="">"All Departments"</option>
                                                    <For
                                                        each=move || dept_options.get()
                                                        key=|(id, _)| id.clone()
                                                        children=move |(id, name)| {
                                                            view! { <option value={id}>{name}</option> }
                                                        }
                                                    />
                                                </select>
                                            </div>
                                        })}
                                    </div>

                                    <div class="grid grid-cols-1 md:grid-cols-4 gap-3">
                                        <div class="stat-card card-hover" style="min-height: 92px;">
                                            <div class="stat-label">"Total Employees"</div>
                                            <div class="stat-value">{move || summary.with(|s| s.total_employees)}</div>
                                        </div>
                                        <div class="stat-card card-hover" style="min-height: 92px;">
                                            <div class="stat-label">"With CTC"</div>
                                            <div class="stat-value">{move || summary.with(|s| s.total_with_ctc)}</div>
                                        </div>
                                        <div
                                            class="stat-card cursor-pointer hover:bg-huly-surface-hover transition-colors"
                                            style="min-height: 92px;"
                                            on:click=fetch_missing
                                        >
                                            <div class="stat-label">"Missing CTC"</div>
                                            <div class="stat-value text-negative-default">{move || summary.with(|s| s.total_missing)}</div>
                                            <div class="text-xs text-blue-500 mt-1">
                                                {move || if is_missing_list_enabled.get() {
                                                    if show_missing.get() { "Hide list" } else { "Click to view list" }
                                                } else { "HR-only list" }}
                                            </div>
                                        </div>
                                        <div class="stat-card card-hover" style="min-height: 92px;">
                                            <div class="stat-label">"Completeness %"</div>
                                            <div class=move || {
                                                let pct = summary.with(|s| s.overall_completion_pct);
                                                format!("stat-value {}", get_color_class(pct))
                                            }>
                                                {move || format!("{:.1}%", summary.with(|s| s.overall_completion_pct))}
                                            </div>
                                        </div>
                                    </div>

                                    {move || show_missing.get().then(|| view! {
                                        <div class="panel mt-4 border border-negative-default/20 bg-negative-default/5">
                                            <h3 class="section-header text-negative-default mb-2">"Employees Missing CTC"</h3>
                                            <div class="overflow-x-auto">
                                                <table class="min-w-full divide-y divide-negative-default/20">
                                                    <thead>
                                                        <tr>
                                                            <th class="th-cell-compact text-negative-default">"Name"</th>
                                                            <th class="th-cell-compact text-negative-default">"Department"</th>
                                                            <th class="th-cell-compact text-negative-default">"Action"</th>
                                                        </tr>
                                                    </thead>
                                                    <tbody class="divide-y divide-negative-default/20">
                                                        <For
                                                            each=move || missing_employees.get()
                                                            key=|e| e.id.clone()
                                                            children=move |e| {
                                                                let emp_id = e.id.clone();
                                                                let add_href = valid_resource_id(&emp_id)
                                                                    .then(|| format!("/ctc?resource_id={}", emp_id));
                                                                view! {
                                                                    <tr>
                                                                        <td class="td-cell-compact text-negative-default">{e.name.clone()}</td>
                                                                        <td class="td-cell-compact text-negative-default">{e.department.clone()}</td>
                                                                        <td class="td-cell-compact">
                                                                            {if let Some(href) = add_href {
                                                                                Either::Left(view! {
                                                                                    <a href=href class="text-primary-400 hover:underline">
                                                                                        "Add CTC"
                                                                                    </a>
                                                                                })
                                                                            } else {
                                                                                Either::Right(view! {
                                                                                    <span class="text-huly-muted">"Unavailable"</span>
                                                                                })
                                                                            }}
                                                                        </td>
                                                                    </tr>
                                                                }
                                                            }
                                                        />
                                                    </tbody>
                                                </table>
                                            </div>
                                        </div>
                                    })}

                                    <div class="border border-huly-divider rounded-md">
                                        <button
                                            type="button"
                                            class="w-full flex items-center justify-between px-3 py-2 text-left"
                                            style="min-height: 36px;"
                                            on:click=move |_| set_show_departments.update(|v| *v = !*v)
                                            aria-expanded=move || show_departments.get().to_string()
                                        >
                                            <span class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Department Breakdown"</span>
                                            <span class="text-xs text-huly-muted">
                                                {move || if show_departments.get() { "Collapse" } else { "Expand" }}
                                            </span>
                                        </button>
                                        {move || show_departments.get().then(|| view! {
                                            <div class="overflow-x-auto border-t border-huly-divider">
                                                <table class="min-w-full divide-y divide-huly-divider">
                                                    <thead class="bg-huly-surface-2">
                                                        <tr>
                                                            <th class="th-cell-compact">"Department"</th>
                                                            <th class="th-cell-compact text-right">"Employees"</th>
                                                            <th class="th-cell-compact text-right">"CTC Complete"</th>
                                                            <th class="th-cell-compact text-right">"Missing"</th>
                                                            <th class="th-cell-compact text-right">"% Complete"</th>
                                                        </tr>
                                                    </thead>
                                                    <tbody class="divide-y divide-huly-divider bg-huly-surface">
                                                        <For
                                                            each=move || summary.with(|s| s.departments.clone())
                                                            key=|d| d.row_key.clone()
                                                            children=move |d| {
                                                                let pct_color = get_color_class(d.completion_pct);
                                                                view! {
                                                                    <tr>
                                                                        <td class="td-cell-compact">{d.department.clone()}</td>
                                                                        <td class="td-cell-compact text-right font-mono">{d.total_employees}</td>
                                                                        <td class="td-cell-compact text-right font-mono">{d.with_ctc}</td>
                                                                        <td class="td-cell-compact text-right font-mono">{d.missing_ctc}</td>
                                                                        <td class=format!("td-cell-compact text-right font-mono font-medium {}", pct_color)>
                                                                            {format!("{:.1}%", d.completion_pct)}
                                                                        </td>
                                                                    </tr>
                                                                }
                                                            }
                                                        />
                                                    </tbody>
                                                </table>
                                            </div>
                                        })}
                                    </div>

                                    <div class="border border-huly-divider rounded-md">
                                        <button
                                            type="button"
                                            class="w-full flex items-center justify-between px-3 py-2 text-left"
                                            style="min-height: 36px;"
                                            on:click=move |_| set_show_trend.update(|v| *v = !*v)
                                            aria-expanded=move || show_trend.get().to_string()
                                        >
                                            <span class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Monthly Completeness Trend"</span>
                                            <span class="text-xs text-huly-muted">
                                                {move || if show_trend.get() { "Collapse" } else { "Expand" }}
                                            </span>
                                        </button>
                                        {move || show_trend.get().then(|| view! {
                                            <div class="overflow-x-auto border-t border-huly-divider">
                                                {move || {
                                                    let trend = summary.with(|s| s.trend.clone());
                                                    if trend.is_empty() {
                                                        leptos::either::Either::Left(view! {
                                                            <div class="empty-state py-4">
                                                                <p class="text-huly-muted text-xs">"Trend data is not yet available."</p>
                                                            </div>
                                                        })
                                                    } else {
                                                        leptos::either::Either::Right(view! {
                                                            <table class="min-w-full divide-y divide-huly-divider">
                                                                <thead class="bg-huly-surface-2">
                                                                    <tr>
                                                                        <th class="th-cell-compact">"Month"</th>
                                                                        <th class="th-cell-compact text-right">"With CTC"</th>
                                                                        <th class="th-cell-compact text-right">"Total"</th>
                                                                        <th class="th-cell-compact text-right">"Missing"</th>
                                                                        <th class="th-cell-compact text-right" style="min-width: 220px;">"Completion"</th>
                                                                    </tr>
                                                                </thead>
                                                                <tbody class="divide-y divide-huly-divider bg-huly-surface">
                                                                    {trend.into_iter().map(|p| {
                                                                        let bar_w = bar_width_pct(p.completion_pct);
                                                                        let bar_color = trend_bar_color(p.completion_pct);
                                                                        view! {
                                                                            <tr>
                                                                                <td class="td-cell-compact font-mono">{p.month.clone()}</td>
                                                                                <td class="td-cell-compact text-right font-mono">{p.total_with_ctc}</td>
                                                                                <td class="td-cell-compact text-right font-mono">{p.total_employees}</td>
                                                                                <td class="td-cell-compact text-right font-mono text-negative-default">{p.total_missing}</td>
                                                                                <td class="td-cell-compact text-right">
                                                                                    <div class="flex items-center justify-end gap-2">
                                                                                        <div class="progress-track h-2" style="width: 120px;">
                                                                                            <div
                                                                                                class="h-2 rounded-full transition-all"
                                                                                                style=format!("width: {:.2}%; {}", bar_w, bar_color)
                                                                                            ></div>
                                                                                        </div>
                                                                                        <span class="font-mono text-huly-content">{format!("{:.1}%", p.completion_pct)}</span>
                                                                                    </div>
                                                                                </td>
                                                                            </tr>
                                                                        }
                                                                    }).collect_view()}
                                                                </tbody>
                                                            </table>
                                                        })
                                                    }
                                                }}
                                            </div>
                                        })}
                                    </div>
                                </div>
                            })}

                            {move || is_hr_or_finance.get().then(|| view! {
                                <div class="panel space-y-4">
                                    <div class="toolbar">
                                        <h2 class="section-header">"BPJS Compliance Report"</h2>
                                    </div>

                                    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 items-end">
                                        <div>
                                            <label class="label">"Start Date"</label>
                                            <input type="date"
                                                class="input text-huly-caption"
                                                prop:value=start_date
                                                on:input=move |ev| set_start_date.set(event_target_value(&ev))
                                            />
                                        </div>
                                        <div>
                                            <label class="label">"End Date"</label>
                                            <input type="date"
                                                class="input text-huly-caption"
                                                prop:value=end_date
                                                on:input=move |ev| set_end_date.set(event_target_value(&ev))
                                            />
                                        </div>
                                        <div>
                                            <button class="btn-primary w-full" disabled=compliance_loading on:click=run_compliance_check>
                                                "Run Compliance Check"
                                            </button>
                                        </div>
                                    </div>

                                    {move || (!compliance_results.get().is_empty()).then(|| view! {
                                        <div class="mt-6 space-y-4">
                                            <div class="grid grid-cols-1 md:grid-cols-4 gap-3">
                                                <div class="p-3 bg-primary-600/10 rounded border border-primary-600/20 text-center">
                                                    <div class="text-xs text-primary-400 uppercase tracking-wider">"Total Validated"</div>
                                                    <div class="text-xl font-bold text-huly-caption">{move || total_validated.get()}</div>
                                                </div>
                                                <div class="p-3 bg-positive-default/10 rounded border border-positive-default/20 text-center">
                                                    <div class="text-xs text-positive-default uppercase tracking-wider">"Passed"</div>
                                                    <div class="text-xl font-bold text-huly-caption">{move || passed.get()}</div>
                                                </div>
                                                <div class="p-3 bg-negative-default/10 rounded border border-negative-default/20 text-center">
                                                    <div class="text-xs text-negative-default uppercase tracking-wider">"Discrepancies"</div>
                                                    <div class="text-xl font-bold text-huly-caption">{move || discrepancies.get()}</div>
                                                </div>
                                                <div class="p-3 bg-huly-surface-2 rounded border border-huly-divider text-center">
                                                    <div class="text-xs text-huly-muted uppercase tracking-wider">"Compliance Rate"</div>
                                                    <div class=move || {
                                                        let rate = compliance_rate.get();
                                                        format!("text-xl font-bold {}", get_color_class(rate))
                                                    }>
                                                        {move || format!("{:.1}%", compliance_rate.get())}
                                                    </div>
                                                </div>
                                            </div>

                                            <div class="overflow-x-auto">
                                                <table class="min-w-full divide-y divide-huly-divider">
                                                    <thead class="bg-huly-surface-2">
                                                        <tr>
                                                            <th class="th-cell-compact">"Employee"</th>
                                                            <th class="th-cell-compact text-right">"Stored BPJS Kes"</th>
                                                            <th class="th-cell-compact text-right">"Expected BPJS Kes"</th>
                                                            <th class="th-cell-compact text-right">"Stored BPJS KT"</th>
                                                            <th class="th-cell-compact text-right">"Expected BPJS KT"</th>
                                                            <th class="th-cell-compact text-center">"Risk Tier"</th>
                                                            <th class="th-cell-compact text-center">"Status"</th>
                                                            <th class="th-cell-compact text-right">"Variance"</th>
                                                        </tr>
                                                    </thead>
                                                    <tbody class="divide-y divide-huly-divider bg-huly-surface">
                                                        <For
                                                            each=move || compliance_results.get()
                                                            key=|r| r.resource_id.clone()
                                                            children=move |r| {
                                                                let status_badge = if r.status == "PASS" {
                                                                    "badge-positive"
                                                                } else {
                                                                    "badge-negative"
                                                                };
                                                                view! {
                                                                    <tr>
                                                                        <td class="td-cell-compact">
                                                                            <div class="font-medium">{r.name.clone()}</div>
                                                                            <div class="text-xs text-huly-muted">{r.resource_id.clone()}</div>
                                                                        </td>
                                                                        <td class="td-cell-compact text-right font-mono">{r.stored_bpjs_kes}</td>
                                                                        <td class="td-cell-compact text-right font-mono">{r.expected_bpjs_kes}</td>
                                                                        <td class="td-cell-compact text-right font-mono">{r.stored_bpjs_kt}</td>
                                                                        <td class="td-cell-compact text-right font-mono">{r.expected_bpjs_kt}</td>
                                                                        <td class="td-cell-compact text-center">{r.risk_tier}</td>
                                                                        <td class="td-cell-compact text-center">
                                                                            <span class=format!("px-2 py-1 inline-flex text-xs leading-5 font-semibold rounded-full border {}", status_badge)>
                                                                                {r.status.clone()}
                                                                            </span>
                                                                        </td>
                                                                        <td class="td-cell-compact text-right font-mono">{r.variance_amount}</td>
                                                                    </tr>
                                                                }
                                                            }
                                                        />
                                                    </tbody>
                                                </table>
                                            </div>
                                        </div>
                                    })}
                                </div>
                            })}
                        </div>
                    })
                }}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_completeness_uses_new_field_names() {
        let body = json!({
            "departments": [
                {
                    "department_id": "00000000-0000-0000-0000-000000000001",
                    "department": "Engineering",
                    "total_employees": 10,
                    "with_ctc": 7,
                    "missing_ctc": 3,
                    "completion_pct": 70.0
                }
            ],
            "total_employees": 10,
            "total_with_ctc": 7,
            "total_missing": 3,
            "overall_completion_pct": 70.0,
            "trend": [
                {
                    "month": "2026-04",
                    "total_employees": 9,
                    "total_with_ctc": 5,
                    "total_missing": 4,
                    "completion_pct": 55.6
                },
                {
                    "month": "2026-05",
                    "total_employees": 10,
                    "total_with_ctc": 7,
                    "total_missing": 3,
                    "completion_pct": 70.0
                }
            ]
        });

        let parsed = parse_completeness_summary(&body);
        assert_eq!(parsed.total_employees, 10);
        assert_eq!(parsed.total_with_ctc, 7);
        assert_eq!(parsed.total_missing, 3);
        assert!((parsed.overall_completion_pct - 70.0).abs() < 0.001);
        assert_eq!(parsed.departments.len(), 1);
        assert_eq!(parsed.departments[0].department, "Engineering");
        assert_eq!(parsed.trend.len(), 2);
        assert_eq!(parsed.trend[0].month, "2026-04");
        assert_eq!(parsed.trend[1].total_with_ctc, 7);
    }

    #[test]
    fn parse_completeness_handles_missing_optional_fields() {
        let body = json!({
            "departments": [],
            "total_employees": 0,
            "total_with_ctc": 0,
            "total_missing": 0,
            "overall_completion_pct": 0.0
        });

        let parsed = parse_completeness_summary(&body);
        assert_eq!(parsed.total_employees, 0);
        assert_eq!(parsed.overall_completion_pct, 0.0);
        assert!(parsed.trend.is_empty());
        assert!(parsed.departments.is_empty());
    }

    #[test]
    fn parse_completeness_derives_total_missing_when_absent() {
        let body = json!({
            "departments": [],
            "total_employees": 12,
            "total_with_ctc": 9,
            "overall_completion_pct": 75.0
        });

        let parsed = parse_completeness_summary(&body);
        assert_eq!(parsed.total_missing, 3);
    }

    #[test]
    fn parse_completeness_derives_department_missing_when_absent() {
        let body = json!({
            "departments": [
                {
                    "department_id": "00000000-0000-0000-0000-000000000001",
                    "department": "Engineering",
                    "total_employees": 8,
                    "with_ctc": 5,
                    "completion_pct": 62.5
                }
            ],
            "total_employees": 8,
            "total_with_ctc": 5,
            "total_missing": 3,
            "overall_completion_pct": 62.5
        });

        let parsed = parse_completeness_summary(&body);
        assert_eq!(parsed.departments.len(), 1);
        assert_eq!(parsed.departments[0].missing_ctc, 3);
    }

    #[test]
    fn parse_completeness_ignores_pre_story_6_5_field_names() {
        // Legacy payloads must not zero out the summary cards when the new
        // field names are also present. The legacy keys are ignored entirely.
        let body = json!({
            "departments": [],
            "total_employees": 4,
            "total_with_ctc": 3,
            "total_missing": 1,
            "overall_completion_pct": 75.0,
            "with_ctc": 99,
            "missing_ctc": 99,
            "completion_pct": 99.0
        });

        let parsed = parse_completeness_summary(&body);
        assert_eq!(parsed.total_with_ctc, 3);
        assert_eq!(parsed.total_missing, 1);
        assert!((parsed.overall_completion_pct - 75.0).abs() < 0.001);
    }

    #[test]
    fn bar_width_pct_clamps_negative_and_oversized() {
        assert_eq!(bar_width_pct(-5.0), 0.0);
        assert_eq!(bar_width_pct(0.0), 0.0);
        assert_eq!(bar_width_pct(50.0), 50.0);
        assert_eq!(bar_width_pct(100.0), 100.0);
        assert_eq!(bar_width_pct(120.0), 100.0);
        assert_eq!(bar_width_pct(f64::NAN), 0.0);
    }

    #[test]
    fn color_class_thresholds_match_dashboard() {
        assert_eq!(get_color_class(95.0), "text-positive-default");
        assert_eq!(get_color_class(75.0), "text-warning-default");
        assert_eq!(get_color_class(40.0), "text-negative-default");
    }

    #[test]
    fn valid_resource_id_accepts_only_uuid_targets() {
        assert!(valid_resource_id("00000000-0000-0000-0000-000000000001"));
        assert!(!valid_resource_id(""));
        assert!(!valid_resource_id("not-a-resource-id"));
    }

    /// Older serializers (or proxies) sometimes emit numeric fields as
    /// strings. The parser must accept stringified ints / floats end-to-end
    /// through `parse_completeness_summary` rather than zeroing out the
    /// summary cards.
    #[test]
    fn parse_completeness_accepts_stringified_numeric_values() {
        let body = json!({
            "departments": [
                {
                    "department_id": "00000000-0000-0000-0000-000000000001",
                    "department": "Stringified",
                    "total_employees": "5",
                    "with_ctc": "3",
                    "missing_ctc": "2",
                    "completion_pct": "60.0"
                }
            ],
            "total_employees": "5",
            "total_with_ctc": "3",
            "total_missing": "2",
            "overall_completion_pct": "60.0",
            "trend": [
                {
                    "month": "2026-05",
                    "total_employees": "5",
                    "total_with_ctc": "3",
                    "total_missing": "2",
                    "completion_pct": "60.0"
                }
            ]
        });

        let parsed = parse_completeness_summary(&body);
        assert_eq!(parsed.total_employees, 5);
        assert_eq!(parsed.total_with_ctc, 3);
        assert_eq!(parsed.total_missing, 2);
        assert!((parsed.overall_completion_pct - 60.0).abs() < 0.001);
        assert_eq!(parsed.departments.len(), 1);
        assert_eq!(parsed.departments[0].total_employees, 5);
        assert_eq!(parsed.departments[0].with_ctc, 3);
        assert!((parsed.departments[0].completion_pct - 60.0).abs() < 0.001);
        assert_eq!(parsed.trend.len(), 1);
        assert_eq!(parsed.trend[0].total_with_ctc, 3);
        assert!((parsed.trend[0].completion_pct - 60.0).abs() < 0.001);
    }

    #[test]
    fn parse_completeness_rejects_non_finite_numeric_strings() {
        let body = json!({
            "departments": [
                {
                    "department_id": "00000000-0000-0000-0000-000000000001",
                    "department": "Non-finite",
                    "total_employees": 5,
                    "with_ctc": 3,
                    "completion_pct": "inf"
                }
            ],
            "total_employees": 5,
            "total_with_ctc": 3,
            "total_missing": 2,
            "overall_completion_pct": "NaN",
            "trend": [
                {
                    "month": "2026-05",
                    "total_employees": 5,
                    "total_with_ctc": 3,
                    "completion_pct": "inf"
                }
            ]
        });

        let parsed = parse_completeness_summary(&body);
        assert!((parsed.overall_completion_pct - 60.0).abs() < 0.001);
        assert!((parsed.departments[0].completion_pct - 60.0).abs() < 0.001);
        assert!((parsed.trend[0].completion_pct - 60.0).abs() < 0.001);
    }

    /// Trend buckets with `total_employees = 0` must yield a clean
    /// `completion_pct = 0.0` and a non-negative `total_missing`: never NaN
    /// or a negative derived value. Bars width clamps follow.
    #[test]
    fn parse_completeness_trend_with_zero_total_has_zero_percent_no_nan() {
        let body = json!({
            "departments": [],
            "total_employees": 0,
            "total_with_ctc": 0,
            "overall_completion_pct": 0.0,
            "trend": [
                {
                    "month": "2026-05",
                    "total_employees": 0,
                    "total_with_ctc": 0
                }
            ]
        });

        let parsed = parse_completeness_summary(&body);
        assert_eq!(parsed.trend.len(), 1);
        let bucket = &parsed.trend[0];
        assert_eq!(bucket.total_employees, 0);
        assert_eq!(bucket.total_with_ctc, 0);
        assert_eq!(
            bucket.total_missing, 0,
            "missing must derive to 0 (never negative) when both totals are zero"
        );
        assert!(
            bucket.completion_pct == 0.0 && !bucket.completion_pct.is_nan(),
            "completion_pct must be 0.0 (no NaN/Inf), got {}",
            bucket.completion_pct
        );

        // Bar width clamp survives the zero path (already covered for the
        // standalone helper; this re-checks the path through the parser).
        assert_eq!(bar_width_pct(bucket.completion_pct), 0.0);
    }
}
