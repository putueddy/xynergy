use crate::auth::{
    AuthContext, authenticated_get, authenticated_post_json, auth_token, clear_auth_storage,
    use_auth, validate_token,
};
use crate::components::{Footer, Header};
use chrono::NaiveDate;
use leptos::*;
use leptos_router::*;
use serde_json::{json, Value};

#[derive(Clone, Debug)]
struct ResourceOption {
    id: String,
    name: String,
    department_id: Option<String>,
}

#[derive(Clone, Debug)]
struct ThrConfig {
    thr_eligible: bool,
    thr_calculation_basis: String,
    employment_start_date: String,
}

#[derive(Clone, Debug)]
struct ThrAccrualHistoryRow {
    period: String,
    service_months: i64,
    basis: String,
    accrual_amount: i64,
    annual_entitlement: i64,
}

#[derive(Clone, Debug)]
struct ThrReportRow {
    resource_id: String,
    month: String,
    service_months: i64,
    basis: String,
    basis_explanation: String,
    thr_basis_amount: i64,
    annual_entitlement: i64,
    accrued_to_date: i64,
    remaining_top_up: i64,
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

#[component]
pub fn ThrManagement() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();
    let (auth_checked, set_auth_checked) = create_signal(false);
    let (auth_check_in_progress, set_auth_check_in_progress) = create_signal(false);

    let (resources, set_resources) = create_signal(Vec::<ResourceOption>::new());
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(None::<String>);
    let (success, set_success) = create_signal(None::<String>);

    let (selected_resource, set_selected_resource) = create_signal(String::new());

    let (thr_eligible, set_thr_eligible) = create_signal(false);
    let (thr_calculation_basis, set_thr_calculation_basis) = create_signal(String::from("full"));
    let (employment_start_date, set_employment_start_date) = create_signal(String::new());

    let (accrual_period, set_accrual_period) = create_signal(String::new());
    let (accrual_result, set_accrual_result) = create_signal(None::<(i64, i64)>);
    let (accrual_history, set_accrual_history) = create_signal(Vec::<ThrAccrualHistoryRow>::new());

    let (report_month, set_report_month) = create_signal(String::new());
    let (report_rows, set_report_rows) = create_signal(Vec::<ThrReportRow>::new());

    {
        let navigate = navigate.clone();
        create_effect(move |_| {
            if !auth.is_authenticated.get() {
                navigate("/login", Default::default());
                return;
            }

            if let Some(user) = auth.user.get() {
                set_auth_checked.set(true);
                if user.role != "hr" {
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

    let is_hr = Signal::derive(move || {
        auth.user
            .get()
            .map(|u| u.role == "hr")
            .unwrap_or(false)
    });

    create_effect(move |_| {
        if auth.token.get().is_some() {
            if !is_hr.get() {
                return;
            }
            set_loading.set(true);
            spawn_local(async move {
                match fetch_resources().await {
                    Ok(res) => {
                        set_resources.set(res);
                        set_error.set(None);
                    }
                    Err(e) => set_error.set(Some(e)),
                }

                set_loading.set(false);
            });
        }
    });

    let selected_resource_view = Signal::derive(move || {
        let selected_id = selected_resource.get();
        resources
            .get()
            .into_iter()
            .find(|r| r.id == selected_id)
    });

    let save_config = move |_| {
        set_error.set(None);
        set_success.set(None);

        if current_access_token(&auth).is_none() {
            set_error.set(Some("Please login again".to_string()));
            return;
        }

        let resource_id = selected_resource.get();
        if resource_id.is_empty() {
            set_error.set(Some("Employee selection is required".to_string()));
            return;
        }

        let start_date_raw = employment_start_date.get();
        let start_date_trimmed = start_date_raw.trim().to_string();
        let start_date_payload = if start_date_trimmed.is_empty() {
            None
        } else {
            match NaiveDate::parse_from_str(&start_date_trimmed, "%Y-%m-%d") {
                Ok(date) => Some(date.format("%Y-%m-%d").to_string()),
                Err(_) => {
                    set_error.set(Some(
                        "Employment start date must be in YYYY-MM-DD format".to_string(),
                    ));
                    return;
                }
            }
        };

        let payload = json!({
            "thr_eligible": thr_eligible.get(),
            "thr_calculation_basis": thr_calculation_basis.get(),
            "thr_employment_start_date": start_date_payload,
        });

        set_loading.set(true);
        spawn_local(async move {
            match configure_thr(resource_id, payload).await {
                Ok(_) => set_success.set(Some("THR configuration saved successfully".to_string())),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    let run_accrual = move |_| {
        set_error.set(None);
        set_success.set(None);
        set_accrual_result.set(None);

        if current_access_token(&auth).is_none() {
            set_error.set(Some("Please login again".to_string()));
            return;
        }

        let period = accrual_period.get().trim().to_string();
        if !is_valid_year_month(&period) {
            set_error.set(Some("Accrual period must be in YYYY-MM format".to_string()));
            return;
        }

        set_loading.set(true);
        spawn_local(async move {
            match run_thr_monthly_accrual(&period).await {
                Ok((processed, skipped)) => {
                    set_accrual_result.set(Some((processed, skipped)));
                    set_success.set(Some(format!(
                        "Monthly accrual completed. Processed: {}, Skipped: {}",
                        processed, skipped
                    )));

                    let selected = selected_resource.get();
                    if !selected.is_empty() {
                        match fetch_thr_accrual_history(&selected).await {
                            Ok(rows) => set_accrual_history.set(rows),
                            Err(e) => set_error.set(Some(e)),
                        }
                    }
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    let generate_report = move |_| {
        set_error.set(None);
        set_success.set(None);

        if current_access_token(&auth).is_none() {
            set_error.set(Some("Please login again".to_string()));
            return;
        }

        let month = report_month.get().trim().to_string();
        if !is_valid_year_month(&month) {
            set_error.set(Some("Report month must be in YYYY-MM format".to_string()));
            return;
        }

        set_loading.set(true);
        spawn_local(async move {
            match fetch_thr_report(&month).await {
                Ok(rows) => {
                    let total_rows = rows.len();
                    set_report_rows.set(rows);
                    set_success.set(Some(format!(
                        "THR payout report generated. {} row(s) loaded.",
                        total_rows
                    )));
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
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
                        }
                            .into_view();
                    }

                    if !is_hr.get() {
                        return view! {
                            <div class="rounded-md bg-red-50 p-4 dark:bg-red-900/20 text-red-800 dark:text-red-200">
                                "Access denied. THR management is available to HR users only."
                            </div>
                        }
                            .into_view();
                    }

                    view! {
                        <div class="space-y-6">
                            <div class="flex items-center justify-between">
                                <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                                    "THR Management"
                                </h1>
                            </div>

                            {move || auth.user.get().map(|u| {
                                if u.role != "hr" {
                                    view! {
                                        <div class="rounded-md bg-yellow-50 p-4 dark:bg-yellow-900/20 text-yellow-800 dark:text-yellow-200">
                                            "Only HR users can manage THR data."
                                        </div>
                                    }
                                } else {
                                    view! { <div></div> }
                                }
                            })}

                            {move || error.get().map(|err| view! {
                                <div class="rounded-md bg-red-50 p-4 dark:bg-red-900/20 text-red-800 dark:text-red-200">{err}</div>
                            })}

                            {move || success.get().map(|msg| view! {
                                <div class="rounded-md bg-green-50 p-4 dark:bg-green-900/20 text-green-800 dark:text-green-200">{msg}</div>
                            })}

                            <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 space-y-4">
                                <h2 class="text-xl font-semibold text-gray-900 dark:text-white">"A. THR Configuration (Per Employee)"</h2>

                                <div>
                                    <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">"Employee"</label>
                                    <select
                                        class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700"
                                        prop:value=selected_resource
                                        on:change=move |ev| {
                                            let selected = event_target_value(&ev);
                                            set_selected_resource.set(selected.clone());
                                            set_success.set(None);
                                            set_error.set(None);
                                            set_accrual_result.set(None);

                                            if selected.is_empty() {
                                                set_thr_eligible.set(false);
                                                set_thr_calculation_basis.set(String::from("full"));
                                                set_employment_start_date.set(String::new());
                                                set_accrual_history.set(Vec::new());
                                                return;
                                            }

                                            if current_access_token(&auth).is_none() {
                                                set_error.set(Some("Please login again".to_string()));
                                                return;
                                            }

                                            set_loading.set(true);
                                            spawn_local(async move {
                                                match fetch_thr_config(&selected).await {
                                                    Ok(config) => {
                                                        set_thr_eligible.set(config.thr_eligible);
                                                        set_thr_calculation_basis.set(config.thr_calculation_basis);
                                                        set_employment_start_date.set(config.employment_start_date);
                                                    }
                                                    Err(e) => set_error.set(Some(e)),
                                                }

                                                match fetch_thr_accrual_history(&selected).await {
                                                    Ok(rows) => set_accrual_history.set(rows),
                                                    Err(e) => set_error.set(Some(e)),
                                                }

                                                set_loading.set(false);
                                            });
                                        }
                                    >
                                        <option value="">"Select employee"</option>
                                        <For
                                            each=move || resources.get()
                                            key=|r| r.id.clone()
                                            children=move |r| {
                                                view! {
                                                    <option value={r.id.clone()}>{format!("{} ({})", r.name, r.id)}</option>
                                                }
                                            }
                                        />
                                    </select>
                                </div>

                                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                    <div>
                                        <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">"Name"</label>
                                        <input
                                            class="w-full border rounded px-3 py-2 bg-gray-100 dark:bg-gray-700"
                                            readonly=true
                                            value=move || selected_resource_view.get().map(|r| r.name).unwrap_or_default()
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">"Department ID"</label>
                                        <input
                                            class="w-full border rounded px-3 py-2 bg-gray-100 dark:bg-gray-700"
                                            readonly=true
                                            value=move || selected_resource_view
                                                .get()
                                                .and_then(|r| r.department_id)
                                                .unwrap_or_else(|| "-".to_string())
                                        />
                                    </div>
                                </div>

                                <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                                    <div class="flex items-center gap-2 pt-8">
                                        <input
                                            id="thr_eligible"
                                            type="checkbox"
                                            class="h-4 w-4"
                                            prop:checked=thr_eligible
                                            on:change=move |ev| set_thr_eligible.set(event_target_checked(&ev))
                                        />
                                        <label for="thr_eligible" class="text-sm font-medium text-gray-700 dark:text-gray-300">
                                            "THR Eligible"
                                        </label>
                                    </div>

                                    <div>
                                        <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">"Calculation Basis"</label>
                                        <select
                                            class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700"
                                            prop:value=thr_calculation_basis
                                            on:change=move |ev| set_thr_calculation_basis.set(event_target_value(&ev))
                                        >
                                            <option value="full">"full"</option>
                                            <option value="prorated">"prorated"</option>
                                        </select>
                                    </div>

                                    <div>
                                        <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">"Employment Start Date"</label>
                                        <input
                                            class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700"
                                            prop:value=employment_start_date
                                            placeholder="YYYY-MM-DD"
                                            on:input=move |ev| set_employment_start_date.set(event_target_value(&ev))
                                        />
                                    </div>
                                </div>

                                <div class="flex gap-3 items-center">
                                    <button class="btn-primary" disabled=loading on:click=save_config>
                                        "Save"
                                    </button>
                                </div>
                            </div>

                            <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 space-y-4">
                                <h2 class="text-xl font-semibold text-gray-900 dark:text-white">"B. Run Monthly Accrual"</h2>

                                <div class="grid grid-cols-1 md:grid-cols-3 gap-4 items-end">
                                    <div>
                                        <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">"Accrual Period"</label>
                                        <input
                                            class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700"
                                            prop:value=accrual_period
                                            placeholder="YYYY-MM"
                                            on:input=move |ev| set_accrual_period.set(event_target_value(&ev))
                                        />
                                    </div>

                                    <div>
                                        <button class="btn-primary" disabled=loading on:click=run_accrual>
                                            "Run Accrual"
                                        </button>
                                    </div>
                                </div>

                                {move || accrual_result.get().map(|(processed, skipped)| view! {
                                    <div class="rounded-md bg-green-50 p-4 dark:bg-green-900/20 text-green-800 dark:text-green-200">
                                        {format!("Processed: {}, Skipped: {}", processed, skipped)}
                                    </div>
                                })}

                                <div class="pt-2">
                                    <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-3">"Accrual History"</h3>
                                    {move || {
                                        if selected_resource.get().is_empty() {
                                            view! {
                                                <p class="text-sm text-gray-500 dark:text-gray-400">
                                                    "Select an employee to view accrual history."
                                                </p>
                                            }
                                                .into_view()
                                        } else if accrual_history.get().is_empty() {
                                            view! {
                                                <p class="text-sm text-gray-500 dark:text-gray-400">
                                                    "No accrual history found for this employee."
                                                </p>
                                            }
                                                .into_view()
                                        } else {
                                            view! {
                                                <div class="overflow-x-auto">
                                                    <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                                        <thead class="bg-gray-50 dark:bg-gray-700">
                                                            <tr>
                                                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Period"</th>
                                                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Service Months"</th>
                                                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Basis"</th>
                                                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Accrual Amount (IDR)"</th>
                                                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Annual Entitlement (IDR)"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody class="divide-y divide-gray-200 dark:divide-gray-700 bg-white dark:bg-gray-800">
                                                            <For
                                                                each=move || accrual_history.get()
                                                                key=|row| format!("{}-{}", row.period, row.service_months)
                                                                children=move |row| {
                                                                    view! {
                                                                        <tr>
                                                                            <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">{row.period}</td>
                                                                            <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">{row.service_months}</td>
                                                                            <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">{row.basis}</td>
                                                                            <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">{format!("Rp {}", row.accrual_amount)}</td>
                                                                            <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">{format!("Rp {}", row.annual_entitlement)}</td>
                                                                        </tr>
                                                                    }
                                                                }
                                                            />
                                                        </tbody>
                                                    </table>
                                                </div>
                                            }
                                                .into_view()
                                        }
                                    }}
                                </div>
                            </div>

                            <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 space-y-4">
                                <h2 class="text-xl font-semibold text-gray-900 dark:text-white">"C. THR Payout Report"</h2>

                                <div class="grid grid-cols-1 md:grid-cols-3 gap-4 items-end">
                                    <div>
                                        <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">"Report Month"</label>
                                        <input
                                            class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700"
                                            prop:value=report_month
                                            placeholder="YYYY-MM"
                                            on:input=move |ev| set_report_month.set(event_target_value(&ev))
                                        />
                                    </div>
                                    <div>
                                        <button class="btn-primary" disabled=loading on:click=generate_report>
                                            "Generate Report"
                                        </button>
                                    </div>
                                </div>

                                {move || {
                                    if report_rows.get().is_empty() {
                                        view! {
                                            <p class="text-sm text-gray-500 dark:text-gray-400">
                                                "No report data loaded. Generate a report for a month."
                                            </p>
                                        }
                                            .into_view()
                                    } else {
                                        view! {
                                            <div class="overflow-x-auto">
                                                <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                                    <thead class="bg-gray-50 dark:bg-gray-700">
                                                        <tr>
                                                            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Resource ID"</th>
                                                            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Month"</th>
                                                            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Service Months"</th>
                                                            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Basis"</th>
                                                            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Basis Explanation"</th>
                                                            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"THR Basis Amount"</th>
                                                            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Annual Entitlement"</th>
                                                            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Accrued To Date"</th>
                                                            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Remaining Top-Up"</th>
                                                        </tr>
                                                    </thead>
                                                    <tbody class="divide-y divide-gray-200 dark:divide-gray-700 bg-white dark:bg-gray-800">
                                                        <For
                                                            each=move || report_rows.get()
                                                            key=|row| format!("{}-{}", row.resource_id, row.month)
                                                            children=move |row| {
                                                                view! {
                                                                    <tr>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">{row.resource_id}</td>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">{row.month}</td>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">{row.service_months}</td>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">{row.basis}</td>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">{row.basis_explanation}</td>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">{format!("Rp {}", row.thr_basis_amount)}</td>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">{format!("Rp {}", row.annual_entitlement)}</td>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">{format!("Rp {}", row.accrued_to_date)}</td>
                                                                        <td class="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">{format!("Rp {}", row.remaining_top_up)}</td>
                                                                    </tr>
                                                                }
                                                            }
                                                        />
                                                    </tbody>
                                                </table>
                                            </div>
                                        }
                                            .into_view()
                                    }
                                }}
                            </div>
                        </div>
                    }
                        .into_view()
                }}
            </main>

            <Footer/>
        </div>
    }
}

fn is_valid_year_month(input: &str) -> bool {
    let bytes = input.as_bytes();
    if bytes.len() != 7 {
        return false;
    }
    bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4] == b'-'
        && bytes[5].is_ascii_digit()
        && bytes[6].is_ascii_digit()
}

fn value_to_i64(value: &Value) -> Option<i64> {
    if let Some(v) = value.as_i64() {
        return Some(v);
    }

    value.as_str()?.parse::<i64>().ok()
}

async fn fetch_resources() -> Result<Vec<ResourceOption>, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/resources")
        .await
        .map_err(|e| format!("Failed to fetch resources: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch resources: {}", response.status()));
    }

    let values: Vec<Value> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse resources: {}", e))?;

    Ok(values
        .into_iter()
        .filter_map(|v| {
            Some(ResourceOption {
                id: v.get("id")?.as_str()?.to_string(),
                name: v.get("name")?.as_str()?.to_string(),
                department_id: v
                    .get("department_id")
                    .and_then(|d| d.as_str())
                    .map(|s| s.to_string()),
            })
        })
        .collect())
}

async fn fetch_thr_config(resource_id: &str) -> Result<ThrConfig, String> {
    let response = authenticated_get(&format!(
        "http://localhost:3000/api/v1/thr/config/{}",
        resource_id
    ))
    .await
    .map_err(|e| format!("Failed to fetch THR config: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch THR config: {}", response.status()));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse THR config: {}", e))?;

    let source = body.get("config").cloned().unwrap_or(body);

    Ok(ThrConfig {
        thr_eligible: source
            .get("thr_eligible")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        thr_calculation_basis: source
            .get("thr_calculation_basis")
            .and_then(|v| v.as_str())
            .unwrap_or("full")
            .to_string(),
        employment_start_date: source
            .get("employment_start_date")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

async fn configure_thr(resource_id: String, payload: Value) -> Result<(), String> {
    let response = authenticated_post_json(
        &format!("http://localhost:3000/api/v1/thr/configure/{}", resource_id),
        &payload,
    )
    .await
    .map_err(|e| format!("Failed to configure THR: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("THR configuration failed: {}", text))
    }
}

async fn run_thr_monthly_accrual(period: &str) -> Result<(i64, i64), String> {
    let payload = json!({
        "accrual_period": period,
    });

    let response = authenticated_post_json("http://localhost:3000/api/v1/thr/accrual/run", &payload)
        .await
        .map_err(|e| format!("Failed to run THR accrual: {}", e))?;

    if !response.status().is_success() {
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("THR accrual failed: {}", text));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse THR accrual response: {}", e))?;

    let processed = body
        .get("processed")
        .and_then(value_to_i64)
        .unwrap_or_default();
    let skipped = body
        .get("skipped")
        .and_then(value_to_i64)
        .unwrap_or_default();

    Ok((processed, skipped))
}

async fn fetch_thr_accrual_history(resource_id: &str) -> Result<Vec<ThrAccrualHistoryRow>, String> {
    let response = authenticated_get(&format!(
        "http://localhost:3000/api/v1/thr/accrual/{}",
        resource_id
    ))
    .await
    .map_err(|e| format!("Failed to fetch THR accrual history: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch THR accrual history: {}",
            response.status()
        ));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse THR accrual history: {}", e))?;

    let rows = body
        .get("accruals")
        .and_then(|v| v.as_array())
        .cloned()
        .or_else(|| body.as_array().cloned())
        .unwrap_or_default();

    Ok(rows
        .into_iter()
        .map(|row| ThrAccrualHistoryRow {
            period: row
                .get("period")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            service_months: row
                .get("service_months")
                .and_then(value_to_i64)
                .unwrap_or_default(),
            basis: row
                .get("basis")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            accrual_amount: row
                .get("accrual_amount")
                .and_then(value_to_i64)
                .unwrap_or_default(),
            annual_entitlement: row
                .get("annual_entitlement")
                .and_then(value_to_i64)
                .unwrap_or_default(),
        })
        .collect())
}

async fn fetch_thr_report(month: &str) -> Result<Vec<ThrReportRow>, String> {
    let response = authenticated_get(&format!(
        "http://localhost:3000/api/v1/thr/report?month={}",
        month
    ))
    .await
    .map_err(|e| format!("Failed to fetch THR payout report: {}", e))?;

    if !response.status().is_success() {
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Failed to fetch THR payout report: {}", text));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse THR payout report: {}", e))?;

    let rows = body
        .get("entries")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    Ok(rows
        .into_iter()
        .map(|row| ThrReportRow {
            resource_id: row
                .get("resource_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            month: row
                .get("month")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            service_months: row
                .get("service_months")
                .and_then(value_to_i64)
                .unwrap_or_default(),
            basis: row
                .get("calculation_basis")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            basis_explanation: row
                .get("calculation_basis_explanation")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            thr_basis_amount: row
                .get("thr_basis_amount")
                .and_then(value_to_i64)
                .unwrap_or_default(),
            annual_entitlement: row
                .get("annual_entitlement")
                .and_then(value_to_i64)
                .unwrap_or_default(),
            accrued_to_date: row
                .get("accrued_to_date")
                .and_then(value_to_i64)
                .unwrap_or_default(),
            remaining_top_up: row
                .get("remaining_top_up")
                .and_then(value_to_i64)
                .unwrap_or_default(),
        })
        .collect())
}
