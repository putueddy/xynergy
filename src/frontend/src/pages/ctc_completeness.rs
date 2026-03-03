use crate::auth::{
    auth_token, authenticated_get, clear_auth_storage, use_auth, validate_token, AuthContext,
};
use crate::components::{Footer, Header};
use leptos::*;
use leptos_router::*;
use serde_json::Value;

#[derive(Clone, Debug)]
struct DepartmentRow {
    department_id: String,
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
        return Some(v);
    }
    if let Some(v) = value.as_i64() {
        return Some(v as f64);
    }
    value.as_str()?.parse::<f64>().ok()
}

async fn fetch_completeness(
    department_id: Option<String>,
) -> Result<(Vec<DepartmentRow>, i64, i64, i64, f64), String> {
    let url = match department_id {
        Some(ref id) if !id.is_empty() => {
            format!(
                "/api/v1/ctc/completeness?department_id={}",
                id
            )
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

    let total_employees = body
        .get("total_employees")
        .and_then(value_to_i64)
        .unwrap_or(0);
    let with_ctc = body.get("with_ctc").and_then(value_to_i64).unwrap_or(0);
    let missing_ctc = body.get("missing_ctc").and_then(value_to_i64).unwrap_or(0);
    let completion_pct = body
        .get("completion_pct")
        .and_then(value_to_f64)
        .unwrap_or(0.0);

    let deps = body
        .get("departments")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut department_rows = Vec::new();
    for d in deps {
        department_rows.push(DepartmentRow {
            department_id: d
                .get("department_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            department: d
                .get("department")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            total_employees: d.get("total_employees").and_then(value_to_i64).unwrap_or(0),
            with_ctc: d.get("with_ctc").and_then(value_to_i64).unwrap_or(0),
            missing_ctc: d.get("missing_ctc").and_then(value_to_i64).unwrap_or(0),
            completion_pct: d
                .get("completion_pct")
                .and_then(value_to_f64)
                .unwrap_or(0.0),
        });
    }

    Ok((
        department_rows,
        total_employees,
        with_ctc,
        missing_ctc,
        completion_pct,
    ))
}

async fn fetch_missing_employees() -> Result<Vec<MissingEmployee>, String> {
    let response = authenticated_get("/api/v1/ctc/completeness/missing")
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
        "text-green-600 dark:text-green-400"
    } else if pct >= 70.0 {
        "text-yellow-600 dark:text-yellow-400"
    } else {
        "text-red-600 dark:text-red-400"
    }
}

#[component]
pub fn CtcCompleteness() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();
    let (auth_checked, set_auth_checked) = create_signal(false);
    let (auth_check_in_progress, set_auth_check_in_progress) = create_signal(false);

    let (_loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(None::<String>);
    let (success, set_success) = create_signal(None::<String>);

    let (departments, set_departments) = create_signal(Vec::<DepartmentRow>::new());
    let (dept_filter, set_dept_filter) = create_signal(String::new());
    let (dept_options, set_dept_options) = create_signal(Vec::<(String, String)>::new());
    let (total_employees_summary, set_total_employees_summary) = create_signal(0i64);
    let (with_ctc_summary, set_with_ctc_summary) = create_signal(0i64);
    let (missing_ctc_summary, set_missing_ctc_summary) = create_signal(0i64);
    let (completion_pct_summary, set_completion_pct_summary) = create_signal(0.0f64);

    let (missing_employees, set_missing_employees) = create_signal(Vec::<MissingEmployee>::new());
    let (show_missing, set_show_missing) = create_signal(false);

    let (start_date, set_start_date) = create_signal(String::new());
    let (end_date, set_end_date) = create_signal(String::new());
    let (compliance_results, set_compliance_results) = create_signal(Vec::<ComplianceRow>::new());
    let (compliance_loading, set_compliance_loading) = create_signal(false);
    let (total_validated, set_total_validated) = create_signal(0i64);
    let (passed, set_passed) = create_signal(0i64);
    let (discrepancies, set_discrepancies) = create_signal(0i64);
    let (compliance_rate, set_compliance_rate) = create_signal(0.0f64);

    {
        let navigate = navigate.clone();
        create_effect(move |_| {
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
            spawn_local(async move {
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

    let is_authorized = Signal::derive(move || {
        auth.user
            .get()
            .map(|u| u.role == "hr" || u.role == "department_head" || u.role == "finance")
            .unwrap_or(false)
    });

    let is_hr_or_finance = Signal::derive(move || {
        auth.user
            .get()
            .map(|u| u.role == "hr" || u.role == "finance")
            .unwrap_or(false)
    });

    create_effect(move |_| {
        if auth.token.get().is_some() {
            if !is_authorized.get() {
                return;
            }
            set_loading.set(true);
            spawn_local(async move {
                if let Ok(depts) = fetch_departments_list().await {
                    set_dept_options.set(depts);
                }

                match fetch_completeness(None).await {
                    Ok((deps, total, with_ctc, missing, pct)) => {
                        set_departments.set(deps);
                        set_total_employees_summary.set(total);
                        set_with_ctc_summary.set(with_ctc);
                        set_missing_ctc_summary.set(missing);
                        set_completion_pct_summary.set(pct);
                        set_error.set(None);
                    }
                    Err(e) => set_error.set(Some(e)),
                }
                set_loading.set(false);
            });
        }
    });

    let fetch_missing = move |_| {
        if show_missing.get() {
            set_show_missing.set(false);
            return;
        }

        set_loading.set(true);
        spawn_local(async move {
            match fetch_missing_employees().await {
                Ok(emps) => {
                    set_missing_employees.set(emps);
                    set_show_missing.set(true);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
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
        spawn_local(async move {
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
        <div class="min-h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
            <Header/>

            <main class="flex-grow max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12 w-full">
                {move || {
                    if !auth_checked.get() {
                        return view! {
                            <div class="rounded-md bg-blue-50 p-4 dark:bg-blue-900/20 text-blue-800 dark:text-blue-200">
                                "Checking access..."
                            </div>
                        }.into_view();
                    }

                    if !is_authorized.get() {
                        return view! {
                            <div class="rounded-md bg-red-50 p-4 dark:bg-red-900/20 text-red-800 dark:text-red-200">
                                "Access denied."
                            </div>
                        }.into_view();
                    }

                    view! {
                        <div class="space-y-8">
                            <div class="flex items-center justify-between">
                                <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                                    "CTC Completeness & Compliance"
                                </h1>
                            </div>

                            {move || error.get().map(|err| view! {
                                <div class="rounded-md bg-red-50 p-4 dark:bg-red-900/20 text-red-800 dark:text-red-200">{err}</div>
                            })}

                            {move || success.get().map(|msg| view! {
                                <div class="rounded-md bg-green-50 p-4 dark:bg-green-900/20 text-green-800 dark:text-green-200">{msg}</div>
                            })}

                            // TASK 5: CTC Completeness Dashboard UI
                            <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 space-y-4">
                                <h2 class="text-xl font-semibold text-gray-900 dark:text-white">"Completeness Dashboard"</h2>

                                <div class="flex items-center gap-3">
                                    <label class="text-sm font-medium text-gray-700 dark:text-gray-300">"Filter by Department:"</label>
                                    <select
                                        class="border rounded px-3 py-2 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                                        prop:value=dept_filter
                                        on:change=move |ev| {
                                            let selected = event_target_value(&ev);
                                            set_dept_filter.set(selected.clone());
                                            set_loading.set(true);
                                            spawn_local(async move {
                                                let filter = if selected.is_empty() { None } else { Some(selected) };
                                                match fetch_completeness(filter).await {
                                                    Ok((deps, total, with_ctc, missing, pct)) => {
                                                        set_departments.set(deps);
                                                        set_total_employees_summary.set(total);
                                                        set_with_ctc_summary.set(with_ctc);
                                                        set_missing_ctc_summary.set(missing);
                                                        set_completion_pct_summary.set(pct);
                                                        set_error.set(None);
                                                    }
                                                    Err(e) => set_error.set(Some(e)),
                                                }
                                                set_loading.set(false);
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

                                <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                                    <div class="p-4 bg-gray-50 dark:bg-gray-700 rounded-lg">
                                        <div class="text-sm text-gray-500 dark:text-gray-400">"Total Employees"</div>
                                        <div class="text-2xl font-bold text-gray-900 dark:text-white">{move || total_employees_summary.get()}</div>
                                    </div>
                                    <div class="p-4 bg-gray-50 dark:bg-gray-700 rounded-lg">
                                        <div class="text-sm text-gray-500 dark:text-gray-400">"With CTC"</div>
                                        <div class="text-2xl font-bold text-gray-900 dark:text-white">{move || with_ctc_summary.get()}</div>
                                    </div>
                                    <div class="p-4 bg-gray-50 dark:bg-gray-700 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors"
                                         on:click=fetch_missing>
                                        <div class="text-sm text-gray-500 dark:text-gray-400">"Missing CTC"</div>
                                        <div class="text-2xl font-bold text-red-600 dark:text-red-400">{move || missing_ctc_summary.get()}</div>
                                        <div class="text-xs text-blue-500 mt-1">"Click to view list"</div>
                                    </div>
                                    <div class="p-4 bg-gray-50 dark:bg-gray-700 rounded-lg">
                                        <div class="text-sm text-gray-500 dark:text-gray-400">"Completeness %"</div>
                                        <div class=move || {
                                            let pct = completion_pct_summary.get();
                                            format!("text-2xl font-bold {}", get_color_class(pct))
                                        }>
                                            {move || format!("{:.1}%", completion_pct_summary.get())}
                                        </div>
                                    </div>
                                </div>

                                {move || show_missing.get().then(|| view! {
                                    <div class="mt-4 p-4 border border-red-200 dark:border-red-800 rounded-lg bg-red-50 dark:bg-red-900/10">
                                        <h3 class="text-lg font-medium text-red-800 dark:text-red-200 mb-2">"Employees Missing CTC"</h3>
                                        <div class="overflow-x-auto">
                                            <table class="min-w-full divide-y divide-red-200 dark:divide-red-800/50">
                                                <thead>
                                                    <tr>
                                                        <th class="px-4 py-2 text-left text-xs font-medium text-red-800 dark:text-red-200">"ID"</th>
                                                        <th class="px-4 py-2 text-left text-xs font-medium text-red-800 dark:text-red-200">"Name"</th>
                                                        <th class="px-4 py-2 text-left text-xs font-medium text-red-800 dark:text-red-200">"Department"</th>
                                                        <th class="px-4 py-2 text-left text-xs font-medium text-red-800 dark:text-red-200">"Action"</th>
                                                    </tr>
                                                </thead>
                                                <tbody class="divide-y divide-red-200 dark:divide-red-800/50">
                                                    <For
                                                        each=move || missing_employees.get()
                                                        key=|e| e.id.clone()
                                                        children=move |e| {
                                                            let emp_id = e.id.clone();
                                                            view! {
                                                                <tr>
                                                                    <td class="px-4 py-2 text-sm text-red-900 dark:text-red-100">{emp_id.clone()}</td>
                                                                    <td class="px-4 py-2 text-sm text-red-900 dark:text-red-100">{e.name.clone()}</td>
                                                                    <td class="px-4 py-2 text-sm text-red-900 dark:text-red-100">{e.department.clone()}</td>
                                                                    <td class="px-4 py-2 text-sm">
                                                                        <a href=format!("/ctc?resource_id={}", emp_id) class="text-blue-600 dark:text-blue-400 hover:underline">
                                                                            "Add CTC"
                                                                        </a>
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

                                <div class="mt-6 overflow-x-auto">
                                    <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                        <thead class="bg-gray-50 dark:bg-gray-700">
                                            <tr>
                                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Department"</th>
                                                <th class="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase">"Employees"</th>
                                                <th class="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase">"CTC Complete"</th>
                                                <th class="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase">"Missing"</th>
                                                <th class="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase">"% Complete"</th>
                                            </tr>
                                        </thead>
                                        <tbody class="divide-y divide-gray-200 dark:divide-gray-700 bg-white dark:bg-gray-800">
                                            <For
                                                each=move || departments.get()
                                                key=|d| d.department_id.clone()
                                                children=move |d| {
                                                    let pct_color = get_color_class(d.completion_pct);
                                                    view! {
                                                        <tr>
                                                            <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">{d.department.clone()}</td>
                                                            <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100 text-right font-mono">{d.total_employees}</td>
                                                            <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100 text-right font-mono">{d.with_ctc}</td>
                                                            <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100 text-right font-mono">{d.missing_ctc}</td>
                                                            <td class=format!("px-4 py-2 text-sm text-right font-mono font-medium {}", pct_color)>
                                                                {format!("{:.1}%", d.completion_pct)}
                                                            </td>
                                                        </tr>
                                                    }
                                                }
                                            />
                                        </tbody>
                                    </table>
                                </div>
                            </div>

                            // TASK 6: BPJS Compliance Report UI
                            {move || is_hr_or_finance.get().then(|| view! {
                                <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 space-y-4">
                                    <h2 class="text-xl font-semibold text-gray-900 dark:text-white">"BPJS Compliance Report"</h2>

                                    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 items-end">
                                        <div>
                                            <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">"Start Date"</label>
                                            <input type="date"
                                                class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                                prop:value=start_date
                                                on:input=move |ev| set_start_date.set(event_target_value(&ev))
                                            />
                                        </div>
                                        <div>
                                            <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">"End Date"</label>
                                            <input type="date"
                                                class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
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
                                            <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                                                <div class="p-3 bg-blue-50 dark:bg-blue-900/20 rounded border border-blue-100 dark:border-blue-800 text-center">
                                                    <div class="text-xs text-blue-600 dark:text-blue-400 uppercase tracking-wider">"Total Validated"</div>
                                                    <div class="text-xl font-bold text-blue-900 dark:text-blue-100">{move || total_validated.get()}</div>
                                                </div>
                                                <div class="p-3 bg-green-50 dark:bg-green-900/20 rounded border border-green-100 dark:border-green-800 text-center">
                                                    <div class="text-xs text-green-600 dark:text-green-400 uppercase tracking-wider">"Passed"</div>
                                                    <div class="text-xl font-bold text-green-900 dark:text-green-100">{move || passed.get()}</div>
                                                </div>
                                                <div class="p-3 bg-red-50 dark:bg-red-900/20 rounded border border-red-100 dark:border-red-800 text-center">
                                                    <div class="text-xs text-red-600 dark:text-red-400 uppercase tracking-wider">"Discrepancies"</div>
                                                    <div class="text-xl font-bold text-red-900 dark:text-red-100">{move || discrepancies.get()}</div>
                                                </div>
                                                <div class="p-3 bg-gray-50 dark:bg-gray-700 rounded border border-gray-200 dark:border-gray-600 text-center">
                                                    <div class="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wider">"Compliance Rate"</div>
                                                    <div class=move || {
                                                        let rate = compliance_rate.get();
                                                        format!("text-xl font-bold {}", get_color_class(rate))
                                                    }>
                                                        {move || format!("{:.1}%", compliance_rate.get())}
                                                    </div>
                                                </div>
                                            </div>

                                            <div class="overflow-x-auto">
                                                <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                                    <thead class="bg-gray-50 dark:bg-gray-700">
                                                        <tr>
                                                            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Employee"</th>
                                                            <th class="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase">"Stored BPJS Kes"</th>
                                                            <th class="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase">"Expected BPJS Kes"</th>
                                                            <th class="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase">"Stored BPJS KT"</th>
                                                            <th class="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase">"Expected BPJS KT"</th>
                                                            <th class="px-4 py-2 text-center text-xs font-medium text-gray-500 uppercase">"Risk Tier"</th>
                                                            <th class="px-4 py-2 text-center text-xs font-medium text-gray-500 uppercase">"Status"</th>
                                                            <th class="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase">"Variance"</th>
                                                        </tr>
                                                    </thead>
                                                    <tbody class="divide-y divide-gray-200 dark:divide-gray-700 bg-white dark:bg-gray-800">
                                                        <For
                                                            each=move || compliance_results.get()
                                                            key=|r| r.resource_id.clone()
                                                            children=move |r| {
                                                                let status_badge = if r.status == "PASS" {
                                                                    "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300 border-green-200"
                                                                } else {
                                                                    "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300 border-red-200"
                                                                };
                                                                view! {
                                                                    <tr>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">
                                                                            <div class="font-medium">{r.name.clone()}</div>
                                                                            <div class="text-xs text-gray-500">{r.resource_id.clone()}</div>
                                                                        </td>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100 text-right font-mono">{r.stored_bpjs_kes}</td>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100 text-right font-mono">{r.expected_bpjs_kes}</td>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100 text-right font-mono">{r.stored_bpjs_kt}</td>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100 text-right font-mono">{r.expected_bpjs_kt}</td>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100 text-center">{r.risk_tier}</td>
                                                                        <td class="px-4 py-2 text-sm text-center">
                                                                            <span class=format!("px-2 py-1 inline-flex text-xs leading-5 font-semibold rounded-full border {}", status_badge)>
                                                                                {r.status.clone()}
                                                                            </span>
                                                                        </td>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100 text-right font-mono">{r.variance_amount}</td>
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
                    }.into_view()
                }}
            </main>
            <Footer/>
        </div>
    }
}
