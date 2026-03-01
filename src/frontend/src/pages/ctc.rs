use crate::auth::{
    AuthContext, authenticated_get, authenticated_post_json, auth_token, clear_auth_storage,
    use_auth, validate_token,
};
use crate::components::{Footer, Header};
use chrono::DateTime;
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
struct ExistingCtcValues {
    base_salary: i64,
    hra_allowance: i64,
    medical_allowance: i64,
    transport_allowance: i64,
    meal_allowance: i64,
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
pub fn CtcManagement() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();
    let (auth_checked, set_auth_checked) = create_signal(false);
    let (auth_check_in_progress, set_auth_check_in_progress) = create_signal(false);

    let (resources, set_resources) = create_signal(Vec::<ResourceOption>::new());
    let (departments, set_departments) = create_signal(Vec::<(String, String)>::new());
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(None::<String>);
    let (success, set_success) = create_signal(None::<String>);

    let (selected_resource, set_selected_resource) = create_signal(String::new());
    let (base_salary, set_base_salary) = create_signal(String::new());
    let (hra_allowance, set_hra_allowance) = create_signal(String::from("0"));
    let (medical_allowance, set_medical_allowance) = create_signal(String::from("0"));
    let (transport_allowance, set_transport_allowance) = create_signal(String::from("0"));
    let (meal_allowance, set_meal_allowance) = create_signal(String::from("0"));
    let (risk_tier, set_risk_tier) = create_signal(String::from("1"));
    let (working_days, set_working_days) = create_signal(String::from("22"));
    let (preview, set_preview) = create_signal(None::<Value>);

    let (is_editing, set_is_editing) = create_signal(false);
    let (change_reason, set_change_reason) = create_signal(String::new());
    let (effective_date_policy, set_effective_date_policy) = create_signal(String::from("pro_rata"));
    let (history, set_history) = create_signal(Vec::<Value>::new());
    let (show_history, set_show_history) = create_signal(false);
    let (history_loading, set_history_loading) = create_signal(false);

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
                let loaded_resources = fetch_resources().await;
                let loaded_departments = fetch_departments().await;

                match (loaded_resources, loaded_departments) {
                    (Ok(res), Ok(depts)) => {
                        set_resources.set(res);
                        set_departments.set(depts);
                        set_error.set(None);
                    }
                    (Err(e), _) | (_, Err(e)) => set_error.set(Some(e)),
                }

                set_loading.set(false);
            });
        }
    });

    let parse_whole_number = |label: &str, input: &str| -> Result<i64, String> {
        if input.contains('.') {
            return Err(format!("{} must be a whole number (IDR)", label));
        }
        let parsed = input
            .parse::<i64>()
            .map_err(|_| format!("{} must be a valid number", label))?;
        if parsed < 0 {
            return Err(format!("{} must be non-negative", label));
        }
        Ok(parsed)
    };

    let calculate_bpjs = move |_| {
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

        let base = match parse_whole_number("Base salary", &base_salary.get()) {
            Ok(v) if v > 0 => v,
            Ok(_) => {
                set_error.set(Some("Base salary must be positive".to_string()));
                return;
            }
            Err(e) => {
                set_error.set(Some(e));
                return;
            }
        };

        let hra = match parse_whole_number("HRA allowance", &hra_allowance.get()) {
            Ok(v) => v,
            Err(e) => {
                set_error.set(Some(e));
                return;
            }
        };
        let medical = match parse_whole_number("Medical allowance", &medical_allowance.get()) {
            Ok(v) => v,
            Err(e) => {
                set_error.set(Some(e));
                return;
            }
        };
        let transport = match parse_whole_number("Transport allowance", &transport_allowance.get()) {
            Ok(v) => v,
            Err(e) => {
                set_error.set(Some(e));
                return;
            }
        };
        let meal = match parse_whole_number("Meal allowance", &meal_allowance.get()) {
            Ok(v) => v,
            Err(e) => {
                set_error.set(Some(e));
                return;
            }
        };

        let tier = risk_tier.get().parse::<i32>().unwrap_or(1);
        let days = working_days.get().parse::<i32>().unwrap_or(22);

        set_loading.set(true);
        spawn_local(async move {
            let payload = json!({
                "resource_id": resource_id,
                "base_salary": base,
                "hra_allowance": hra,
                "medical_allowance": medical,
                "transport_allowance": transport,
                "meal_allowance": meal,
                "working_days_per_month": days,
                "risk_tier": tier
            });

            match calculate_bpjs_preview(payload).await {
                Ok(data) => set_preview.set(Some(data)),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    let save_ctc = move |_| {
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

        let base = match parse_whole_number("Base salary", &base_salary.get()) {
            Ok(v) if v > 0 => v,
            Ok(_) => {
                set_error.set(Some("Base salary must be positive".to_string()));
                return;
            }
            Err(e) => {
                set_error.set(Some(e));
                return;
            }
        };

        let hra = match parse_whole_number("HRA allowance", &hra_allowance.get()) {
            Ok(v) => v,
            Err(e) => {
                set_error.set(Some(e));
                return;
            }
        };
        let medical = match parse_whole_number("Medical allowance", &medical_allowance.get()) {
            Ok(v) => v,
            Err(e) => {
                set_error.set(Some(e));
                return;
            }
        };
        let transport = match parse_whole_number("Transport allowance", &transport_allowance.get()) {
            Ok(v) => v,
            Err(e) => {
                set_error.set(Some(e));
                return;
            }
        };
        let meal = match parse_whole_number("Meal allowance", &meal_allowance.get()) {
            Ok(v) => v,
            Err(e) => {
                set_error.set(Some(e));
                return;
            }
        };

        let tier = risk_tier.get().parse::<i32>().unwrap_or(1);
        let days = working_days.get().parse::<i32>().unwrap_or(22);

        let d_is_editing = is_editing.get();
        let reason = change_reason.get();
        let policy = effective_date_policy.get();

        if d_is_editing && reason.trim().is_empty() {
            set_error.set(Some("Change reason is required when editing".to_string()));
            return;
        }

        set_loading.set(true);
        spawn_local(async move {
            if d_is_editing {
                let payload = json!({
                    "components": {
                        "base_salary": base,
                        "hra_allowance": hra,
                        "medical_allowance": medical,
                        "transport_allowance": transport,
                        "meal_allowance": meal,
                        "working_days_per_month": days,
                        "risk_tier": tier,
                    },
                    "reason": reason,
                    "effective_date_policy": policy
                });

                match update_ctc_record(resource_id.clone(), payload).await {
                    Ok(_) => {
                        set_success.set(Some("CTC changes saved successfully".to_string()));
                        if let Ok(hist) = fetch_ctc_history(&resource_id).await {
                            set_history.set(hist);
                        }
                    }
                    Err(e) => set_error.set(Some(e)),
                }
            } else {
                let payload = json!({
                    "resource_id": resource_id,
                    "base_salary": base,
                    "hra_allowance": hra,
                    "medical_allowance": medical,
                    "transport_allowance": transport,
                    "meal_allowance": meal,
                    "working_days_per_month": days,
                    "risk_tier": tier
                });

                match create_ctc_record(payload).await {
                    Ok(_) => {
                        set_success.set(Some("CTC record created with status Active".to_string()));
                    }
                    Err(e) => {
                        if e.contains("already exists") {
                            set_is_editing.set(true);
                            set_error.set(Some(
                                "CTC already exists for this employee. Switched to edit mode; add Change Reason and click Save again.".to_string(),
                            ));
                        } else {
                            set_error.set(Some(e));
                        }
                    }
                }
            }
            set_loading.set(false);
        });
    };

    let selected_resource_view = Signal::derive(move || {
        let selected_id = selected_resource.get();
        resources
            .get()
            .into_iter()
            .find(|r| r.id == selected_id)
    });

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
                                "Access denied. CTC management is available to HR users only."
                            </div>
                        }
                            .into_view();
                    }

                    view! {
                <div class="space-y-6">
                    <div class="flex items-center justify-between">
                        <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                            "CTC Management - Add Employee"
                        </h1>
                    </div>

                    {move || auth.user.get().map(|u| {
                        if u.role != "hr" {
                            view! {
                                <div class="rounded-md bg-yellow-50 p-4 dark:bg-yellow-900/20 text-yellow-800 dark:text-yellow-200">
                                    "Only HR users can create CTC records."
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
                        <div>
                            <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">"Employee ID"</label>
                            <select
                                class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700"
                                prop:value=selected_resource
                                on:change=move |ev| {
                                    let selected = event_target_value(&ev);
                                    set_selected_resource.set(selected.clone());
                                    set_preview.set(None);
                                    set_success.set(None);
                                    set_error.set(None);

                                    if selected.is_empty() {
                                        set_base_salary.set(String::new());
                                        set_hra_allowance.set(String::from("0"));
                                        set_medical_allowance.set(String::from("0"));
                                        set_transport_allowance.set(String::from("0"));
                                        set_meal_allowance.set(String::from("0"));
                                        set_risk_tier.set(String::from("1"));
                                        set_working_days.set(String::from("22"));
                                        
                                        set_is_editing.set(false);
                                        set_change_reason.set(String::new());
                                        set_history.set(Vec::new());
                                        set_show_history.set(false);
                                        set_history_loading.set(false);
                                        return;
                                    }

                                    if current_access_token(&auth).is_none() {
                                        set_error.set(Some("Please login again".to_string()));
                                        return;
                                    }

                                    let selected_for_load = selected;
                                    set_is_editing.set(false);
                                    set_change_reason.set(String::new());
                                    set_show_history.set(false);
                                    set_history_loading.set(false);
                                    set_loading.set(true);
                                    spawn_local(async move {
                                        match fetch_existing_ctc(&selected_for_load).await {
                                            Ok(Some(existing)) => {
                                                set_is_editing.set(true);
                                                set_base_salary.set(existing.base_salary.to_string());
                                                set_hra_allowance.set(existing.hra_allowance.to_string());
                                                set_medical_allowance
                                                    .set(existing.medical_allowance.to_string());
                                                set_transport_allowance
                                                    .set(existing.transport_allowance.to_string());
                                                set_meal_allowance.set(existing.meal_allowance.to_string());
                                                
                                                // History is fetched on demand when user clicks "View History"
                                                set_history.set(Vec::new());
                                            }
                                            Ok(None) => {
                                                set_is_editing.set(false);
                                                set_history.set(Vec::new());
                                                set_base_salary.set(String::new());
                                                set_hra_allowance.set(String::from("0"));
                                                set_medical_allowance.set(String::from("0"));
                                                set_transport_allowance.set(String::from("0"));
                                                set_meal_allowance.set(String::from("0"));
                                            }
                                            Err(e) => {
                                                set_is_editing.set(false);
                                                set_error.set(Some(e));
                                            }
                                        }

                                        set_risk_tier.set(String::from("1"));
                                        set_working_days.set(String::from("22"));
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
                                <input class="w-full border rounded px-3 py-2 bg-gray-100 dark:bg-gray-700" readonly=true
                                    value=move || selected_resource_view.get().map(|r| r.name).unwrap_or_default() />
                            </div>
                            <div>
                                <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">"Department"</label>
                                <input class="w-full border rounded px-3 py-2 bg-gray-100 dark:bg-gray-700" readonly=true
                                    value=move || {
                                        if let Some(res) = selected_resource_view.get() {
                                            if let Some(dept_id) = res.department_id {
                                                departments
                                                    .get()
                                                    .into_iter()
                                                    .find(|(id, _)| *id == dept_id)
                                                    .map(|(_, name)| name)
                                                    .unwrap_or(dept_id)
                                            } else {
                                                "-".to_string()
                                            }
                                        } else {
                                            String::new()
                                        }
                                    }
                                />
                            </div>
                        </div>

                        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <MoneyInput label="Base Salary" value=base_salary set_value=set_base_salary/>
                            <MoneyInput label="HRA Allowance" value=hra_allowance set_value=set_hra_allowance/>
                            <MoneyInput label="Medical Allowance" value=medical_allowance set_value=set_medical_allowance/>
                            <MoneyInput label="Transport Allowance" value=transport_allowance set_value=set_transport_allowance/>
                            <MoneyInput label="Meal Allowance" value=meal_allowance set_value=set_meal_allowance/>
                            <div>
                                <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">"Risk Tier"</label>
                                <select class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700"
                                    prop:value=risk_tier
                                    on:change=move |ev| set_risk_tier.set(event_target_value(&ev))>
                                    <option value="1">"1 - Low"</option>
                                    <option value="2">"2 - Medium"</option>
                                    <option value="3">"3 - High"</option>
                                    <option value="4">"4 - Very High"</option>
                                </select>
                            </div>
                            <div>
                                <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">"Working Days"</label>
                                <input
                                    class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700"
                                    prop:value=working_days
                                    on:input=move |ev| set_working_days.set(event_target_value(&ev))
                                />
                            </div>
                        </div>

                        {move || is_editing.get().then(|| view! {
                            <div class="space-y-4 pt-4 border-t border-gray-200 dark:border-gray-700">
                                <h3 class="text-lg font-medium text-gray-900 dark:text-white">"Update Information"</h3>
                                
                                <div>
                                    <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">"Change Reason (Required)"</label>
                                    <textarea
                                        class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                        rows="2"
                                        placeholder="Explain why these changes are being made..."
                                        prop:value=change_reason
                                        on:input=move |ev| set_change_reason.set(event_target_value(&ev))
                                    ></textarea>
                                </div>
                                
                                <div>
                                    <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">"Effective Date Policy"</label>
                                    <select class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700"
                                        prop:value=effective_date_policy
                                        on:change=move |ev| set_effective_date_policy.set(event_target_value(&ev))>
                                        <option value="pro_rata">"Pro Rata Calculation (Immediate)"</option>
                                        <option value="effective_first_of_month">"Effective First of Month"</option>
                                    </select>
                                </div>
                            </div>
                        })}

                        <div class="flex gap-3 items-center mt-4">
                            <button class="btn-secondary" disabled=loading on:click=calculate_bpjs>
                                "Calculate BPJS"
                            </button>
                            <button class="btn-primary" disabled=loading on:click=save_ctc>
                                "Save"
                            </button>
                            
                            {move || is_editing.get().then(|| view! {
                                <button class="ml-auto text-blue-600 dark:text-blue-400 font-medium hover:underline text-sm"
                                    on:click=move |_| {
                                        let should_show = !show_history.get();
                                        set_show_history.set(should_show);

                                        if should_show {
                                            let selected = selected_resource.get();
                                            if selected.is_empty() {
                                                return;
                                            }

                                            set_history_loading.set(true);
                                            spawn_local(async move {
                                                match fetch_ctc_history(&selected).await {
                                                    Ok(hist) => set_history.set(hist),
                                                    Err(e) => set_error.set(Some(e)),
                                                }
                                                set_history_loading.set(false);
                                            });
                                        }
                                    }>
                                    {move || if show_history.get() { "Hide History" } else { "View History" }}
                                </button>
                            })}
                        </div>
                    </div>

                    {move || preview.get().map(|p| view! {
                        <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 space-y-2">
                            <h2 class="text-xl font-semibold text-gray-900 dark:text-white">"Calculation Preview"</h2>
                            <p class="text-gray-900 dark:text-white">{format!("BPJS Kesehatan Employer: {}", p["bpjs"]["kesehatan"]["employer"].as_i64().unwrap_or(0))}</p>
                            <p class="text-gray-900 dark:text-white">{format!("BPJS Kesehatan Employee: {}", p["bpjs"]["kesehatan"]["employee"].as_i64().unwrap_or(0))}</p>
                            <p class="text-gray-900 dark:text-white">{format!("BPJS Ketenagakerjaan Employer: {}", p["bpjs"]["ketenagakerjaan"]["employer"].as_i64().unwrap_or(0))}</p>
                            <p class="text-gray-900 dark:text-white">{format!("BPJS Ketenagakerjaan Employee: {}", p["bpjs"]["ketenagakerjaan"]["employee"].as_i64().unwrap_or(0))}</p>
                            <p class="text-gray-900 dark:text-white">{format!("Total Monthly CTC: {}", p["total_monthly_ctc"].as_i64().unwrap_or(0))}</p>
                            <p class="text-gray-900 dark:text-white">{format!("Daily Rate: {:.2}", p["daily_rate"].as_f64().unwrap_or(0.0))}</p>
                            <p class="text-gray-900 dark:text-white">{format!("THR Monthly Accrual: {}", p["thr_monthly_accrual"].as_i64().unwrap_or(0))}</p>
                        </div>
                    })}

                    {move || show_history.get().then(|| view! {
                        <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 space-y-4">
                            <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">"Revision History"</h2>
                            {move || history_loading.get().then(|| view! {
                                <p class="text-sm text-gray-500 dark:text-gray-400">"Loading revision history..."</p>
                            })}
                            {move || (!history_loading.get() && history.get().is_empty()).then(|| view! {
                                <p class="text-sm text-gray-500 dark:text-gray-400">"No revision history yet."</p>
                            })}
                            <div class="relative border-l border-gray-200 dark:border-gray-700 ml-3">
                                <For
                                    each=move || history.get()
                                    key=|h| h["revision_number"].as_i64().unwrap_or_default()
                                    children=move |h| {
                                        let date = h["date"].as_str().unwrap_or("").to_string();
                                        let reason = h["reason"].as_str().unwrap_or("No reason provided").to_string();
                                        let policy = h["policy"].as_str().unwrap_or("").to_string();
                                        let rev_num = h["revision_number"].as_i64().unwrap_or(0);
                                        
                                        let diffs = h["diffs"].as_array().cloned().unwrap_or_default();
                                        
                                        view! {
                                            <div class="mb-8 ml-6">
                                                <div class="flex justify-between items-start mb-2">
                                                    <div class="flex items-center gap-2">
                                                        <span class="inline-block w-3 h-3 bg-blue-500 rounded-full border border-white dark:border-gray-900 shrink-0"></span>
                                                        <span class="font-medium text-blue-600 dark:text-blue-400">
                                                            {format!(
                                                                "v{} - {}",
                                                                rev_num,
                                                                date.get(0..10).unwrap_or(&date)
                                                            )}
                                                        </span>
                                                        <span class="px-2 py-0.5 rounded text-xs bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-200">
                                                            {policy.replace("_", " ")}
                                                        </span>
                                                    </div>
                                                </div>
                                                <div class="text-gray-800 dark:text-gray-200 text-sm italic mb-2">
                                                    "Reason: "{reason}
                                                </div>
                                                <div class="bg-gray-50 dark:bg-gray-700 rounded-md p-3 text-sm overflow-x-auto">
                                                    <table class="w-full text-left">
                                                        <thead>
                                                            <tr class="text-gray-500 dark:text-gray-400">
                                                                <th class="pb-2 font-medium">"Field"</th>
                                                                <th class="pb-2 font-medium break-all w-1/3">"Old"</th>
                                                                <th class="pb-2 font-medium break-all w-1/3">"New"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody class="text-gray-900 dark:text-gray-100 font-mono text-xs">
                                                            <For
                                                                each=move || diffs.clone()
                                                                key=|d| d["field"].as_str().unwrap_or_default().to_string()
                                                                children=move |d| {
                                                                    let field = d["field"].as_str().unwrap_or("").to_string();
                                                                    let old_val = d["old_value"].to_string();
                                                                    let new_val = d["new_value"].to_string();
                                                                    view! {
                                                                        <tr class="border-t border-gray-200 dark:border-gray-600">
                                                                            <td class="py-1">{field}</td>
                                                                            <td class="py-1 text-red-500 line-through truncate">{old_val}</td>
                                                                            <td class="py-1 text-green-500 truncate">{new_val}</td>
                                                                        </tr>
                                                                    }
                                                                }
                                                            />
                                                        </tbody>
                                                    </table>
                                                </div>
                                            </div>
                                        }
                                    }
                                />
                            </div>
                        </div>
                    })}
                </div>
                    }
                        .into_view()
                }}
            </main>

            <Footer/>
        </div>
    }
}

#[component]
fn MoneyInput(
    #[prop(into)] label: String,
    value: ReadSignal<String>,
    set_value: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div>
            <label class="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">{label}</label>
            <input
                class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700"
                prop:value=value
                on:input=move |ev| set_value.set(event_target_value(&ev))
                placeholder="Whole number IDR"
            />
        </div>
    }
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

async fn fetch_departments() -> Result<Vec<(String, String)>, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/departments")
        .await
        .map_err(|e| format!("Failed to fetch departments: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch departments: {}", response.status()));
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

async fn calculate_bpjs_preview(payload: Value) -> Result<Value, String> {
    let response = authenticated_post_json("http://localhost:3000/api/v1/ctc/calculate", &payload)
        .await
        .map_err(|e| format!("Failed to calculate BPJS: {}", e))?;

    if response.status().is_success() {
        response
            .json::<Value>()
            .await
            .map_err(|e| format!("Failed to parse preview response: {}", e))
    } else {
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("BPJS calculation failed: {}", text))
    }
}

async fn create_ctc_record(payload: Value) -> Result<(), String> {
    let response = crate::auth::authenticated_post_json("http://localhost:3000/api/v1/ctc", &payload)
        .await
        .map_err(|e| format!("Failed to create CTC record: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("CTC creation failed: {}", text))
    }
}

async fn update_ctc_record(resource_id: String, payload: Value) -> Result<(), String> {
    let response = crate::auth::authenticated_put_json(&format!("http://localhost:3000/api/v1/ctc/{}/components", resource_id), &payload)
        .await
        .map_err(|e| format!("Failed to update CTC record: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("CTC update failed: {}", text))
    }
}

async fn fetch_ctc_history(resource_id: &str) -> Result<Vec<Value>, String> {
    let response = crate::auth::authenticated_get(&format!("http://localhost:3000/api/v1/ctc/{}/history", resource_id))
        .await
        .map_err(|e| format!("Failed to fetch history: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch history: {}", response.status()));
    }

    let parsed: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse history: {}", e))?;

    let mut history = parsed
        .get("history")
        .and_then(|h| h.as_array())
        .cloned()
        .unwrap_or_default();

    history.sort_by(|a, b| {
        let a_ts = a
            .get("date")
            .and_then(|d| d.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok());
        let b_ts = b
            .get("date")
            .and_then(|d| d.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok());

        match (a_ts, b_ts) {
            (Some(a_dt), Some(b_dt)) => b_dt.cmp(&a_dt),
            _ => {
                let a_rev = a
                    .get("revision_number")
                    .and_then(|r| r.as_i64())
                    .unwrap_or_default();
                let b_rev = b
                    .get("revision_number")
                    .and_then(|r| r.as_i64())
                    .unwrap_or_default();
                b_rev.cmp(&a_rev)
            }
        }
    });

    Ok(history)
}

fn value_to_i64(value: &Value) -> Option<i64> {
    if let Some(v) = value.as_i64() {
        return Some(v);
    }

    value.as_str()?.parse::<i64>().ok()
}

async fn fetch_existing_ctc(resource_id: &str) -> Result<Option<ExistingCtcValues>, String> {
    let response = authenticated_get(&format!("http://localhost:3000/api/v1/ctc/{}/components", resource_id))
        .await
        .map_err(|e| format!("Failed to fetch CTC details: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch CTC details: {}", response.status()));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse CTC details: {}", e))?;

    let components = body
        .get("components")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let base_salary = components
        .get("base_salary")
        .and_then(value_to_i64);

    if base_salary.is_none() {
        return Ok(None);
    }

    Ok(Some(ExistingCtcValues {
        base_salary: base_salary.unwrap_or(0),
        hra_allowance: components
            .get("hra_allowance")
            .and_then(value_to_i64)
            .unwrap_or(0),
        medical_allowance: components
            .get("medical_allowance")
            .and_then(value_to_i64)
            .unwrap_or(0),
        transport_allowance: components
            .get("transport_allowance")
            .and_then(value_to_i64)
            .unwrap_or(0),
        meal_allowance: components
            .get("meal_allowance")
            .and_then(value_to_i64)
            .unwrap_or(0),
    }))
}
