use crate::auth::{
    authenticated_delete, authenticated_get, authenticated_post_json, authenticated_put_json,
    use_auth,
};
use crate::components::project_list::Project;
use crate::components::{project_form::ProjectFormData, ProjectForm, ProjectList};
use chrono::{Datelike, NaiveDate};
use gloo_timers::callback::Interval;
use leptos::either::{Either, EitherOf3};
use leptos::html;
use leptos::prelude::*;
use leptos_router::hooks::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectBudgetData {
    pub project_id: Uuid,
    pub project_name: String,
    pub client: Option<String>,
    pub total_budget_idr: i64,
    pub budget_hr_idr: i64,
    pub budget_software_idr: i64,
    pub budget_hardware_idr: i64,
    pub budget_overhead_idr: i64,
    pub hr_pct: f64,
    pub software_pct: f64,
    pub hardware_pct: f64,
    pub overhead_pct: f64,
    pub spent_to_date_idr: i64,
    pub remaining_idr: i64,
    pub spent_hr_idr: i64,
    pub spent_software_idr: i64,
    pub spent_hardware_idr: i64,
    pub spent_overhead_idr: i64,
    pub remaining_hr_idr: i64,
    pub remaining_software_idr: i64,
    pub remaining_hardware_idr: i64,
    pub remaining_overhead_idr: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectExpenseData {
    pub id: Uuid,
    pub project_id: Uuid,
    pub category: String,
    pub description: String,
    pub amount_idr: i64,
    pub expense_date: String, // NaiveDate serializes as string
    pub vendor: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default)]
struct ExpenseFormData {
    pub category: String,
    pub description: String,
    pub amount_idr: String,
    pub expense_date: String,
    pub vendor: String,
}

#[derive(Debug, Clone, Default)]
struct ExpenseEditData {
    pub category: String,
    pub description: String,
    pub amount_idr: String,
    pub expense_date: String,
    pub vendor: String,
    pub edit_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResourceCostData {
    pub project_id: Uuid,
    pub total_resource_cost_idr: i64,
    pub employees: Vec<EmployeeResourceCostData>,
    pub monthly_breakdown: Vec<MonthlyResourceCostData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmployeeResourceCostData {
    pub resource_id: Uuid,
    pub resource_name: String,
    pub daily_rate_idr: Option<i64>,
    pub days_allocated: i32,
    pub allocation_percentage: f64,
    pub total_cost_idr: i64,
    pub has_rate_change: bool,
    pub rate_change_note: Option<String>,
    pub missing_rate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MonthlyResourceCostData {
    pub month: String,
    pub working_days: i32,
    pub cost_idr: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectRevenueGridResponse {
    pub project_id: Uuid,
    pub year: i32,
    pub months: Vec<MonthRevenueEntry>,
    pub ytd_total_idr: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MonthRevenueEntry {
    pub month: u32,
    pub month_label: String,
    pub revenue_id: Option<Uuid>,
    pub amount_idr: i64,
    pub source_type: Option<String>,
    pub source_reference: Option<String>,
    pub entered_by: Option<Uuid>,
    pub entry_date: Option<String>, // NaiveDate serializes as string
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectPlDashboardResponse {
    pub project_id: Uuid,
    pub year: i32,
    pub total_revenue_idr: i64,
    pub total_cost_idr: i64,
    pub gross_profit_idr: i64,
    pub margin_pct: f64,
    pub target_margin_pct: f64,
    pub margin_alert_threshold_pct: f64,
    pub margin_alert: Option<String>,
    pub months: Vec<PlMonthEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlMonthEntry {
    pub month: u32,
    pub month_label: String,
    pub revenue_idr: i64,
    pub resource_cost_idr: i64,
    pub non_resource_cost_idr: i64,
    pub total_cost_idr: i64,
    pub gross_profit_idr: i64,
    pub margin_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectPlForecastResponse {
    pub project_id: Uuid,
    pub year: i32,
    pub as_of_date: String,
    pub elapsed_days: i64,
    pub total_project_days: i64,
    pub current_spend_idr: i64,
    pub current_revenue_idr: i64,
    pub burn_rate_idr_per_day: f64,
    pub projected_total_cost_idr: i64,
    pub remaining_cost_projection_idr: i64,
    pub forecast_margin_pct: f64,
    pub target_margin_pct: f64,
    pub variance_from_target_pct: f64,
    pub categories: Vec<ForecastCategoryEntry>,
    pub resource_drivers: Vec<ForecastResourceDriver>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ForecastCategoryEntry {
    pub category: String,
    pub budget_idr: i64,
    pub current_spend_idr: i64,
    pub projected_idr: i64,
    pub overrun_idr: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ForecastResourceDriver {
    pub resource_name: String,
    pub total_cost_idr: i64,
    pub share_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpsertProjectRevenueRequest {
    pub revenue_month: String,
    pub amount_idr: i64,
    pub override_erp: bool,
    pub source_reference: Option<String>,
}

#[derive(Debug, Clone)]
struct RevenueSavePayload {
    project_id: Uuid,
    request: UpsertProjectRevenueRequest,
}

#[component]
fn ExpenseFormPanel(
    category: ReadSignal<String>,
    set_category: WriteSignal<String>,
    amount: ReadSignal<String>,
    set_amount: WriteSignal<String>,
    description: ReadSignal<String>,
    set_description: WriteSignal<String>,
    date_value: ReadSignal<String>,
    set_date_value: WriteSignal<String>,
    vendor: ReadSignal<String>,
    set_vendor: WriteSignal<String>,
    edit_reason: ReadSignal<String>,
    set_edit_reason: WriteSignal<String>,
    is_editing: Signal<bool>,
    loading: ReadSignal<bool>,
    on_submit: Callback<leptos::ev::SubmitEvent>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    view! {
        <form
            class="space-y-4 mb-8 bg-huly-surface-2 p-4 rounded-lg"
            on:submit=move |ev| on_submit.run(ev)
        >
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                    <label class="label">"Category"</label>
                    <select
                        class="mt-1 select"
                        on:change=move |ev| set_category.set(event_target_value(&ev))
                        prop:value=category
                        required
                    >
                        <option value="" disabled>"Select category..."</option>
                        <option value="hr">"HR"</option>
                        <option value="software">"Software"</option>
                        <option value="hardware">"Hardware"</option>
                        <option value="overhead">"Overhead"</option>
                    </select>
                </div>
                <div>
                    <label class="label">"Amount (IDR)"</label>
                    <input
                        type="text"
                        inputmode="numeric"
                        pattern="[0-9]*"
                        autocomplete="off"
                        class="mt-1 input"
                        on:input=move |ev| set_amount.set(event_target_value(&ev))
                        prop:value=amount
                        required
                    />
                </div>
                <div class="md:col-span-2">
                    <label class="label">"Description"</label>
                    <input
                        type="text"
                        class="mt-1 input"
                        on:input=move |ev| set_description.set(event_target_value(&ev))
                        prop:value=description
                        required
                    />
                </div>
                <div>
                    <label class="label">"Date"</label>
                    <input
                        type="date"
                        class="mt-1 input"
                        on:input=move |ev| set_date_value.set(event_target_value(&ev))
                        prop:value=date_value
                        required
                    />
                </div>
                <div>
                    <label class="label">"Vendor (Optional)"</label>
                    <input
                        type="text"
                        class="mt-1 input"
                        on:input=move |ev| set_vendor.set(event_target_value(&ev))
                        prop:value=vendor
                    />
                </div>
                {move || if is_editing.get() {
                    Either::Left(view! {
                        <div class="md:col-span-2">
                            <label class="label">"Edit Reason"</label>
                            <input
                                type="text"
                                class="mt-1 input"
                                on:input=move |ev| set_edit_reason.set(event_target_value(&ev))
                                prop:value=edit_reason
                                required
                            />
                        </div>
                    })
                        
                } else {
                    Either::Right(view! { <div></div> })
                }}
            </div>
            <div class="flex justify-end space-x-3 pt-4">
                <button
                    type="button"
                    class="btn-secondary btn-press"
                    on:click=move |_| on_cancel.run(())
                >
                    "Cancel"
                </button>
                <button
                    type="submit"
                    class="btn-primary btn-press"
                    disabled=move || loading.get()
                >
                    {move || if is_editing.get() { "Update Expense" } else { "Save Expense" }}
                </button>
            </div>
        </form>
    }
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

/// Projects page component
#[component]
pub fn Projects() -> impl IntoView {
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

    // Project data
    let (projects, set_projects) = signal(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (show_form, set_show_form) = signal(false);
    let (editing_project, set_editing_project) = signal(Option::<Project>::None);
    let (selected_budget, set_selected_budget) = signal(Option::<ProjectBudgetData>::None);
    let (selected_project_for_expenses, set_selected_project_for_expenses) =
        signal(Option::<Project>::None);
    let (expenses, set_expenses) = signal(Vec::<ProjectExpenseData>::new());
    let (show_expense_form, set_show_expense_form) = signal(false);
    let (editing_expense, set_editing_expense) = signal(Option::<ProjectExpenseData>::None);
    let (resource_costs, set_resource_costs) = signal(Option::<ResourceCostData>::None);
    let (revenue_year, set_revenue_year) = signal(chrono::Utc::now().year());
    let (revenue_project_id, set_revenue_project_id) = signal(Option::<Uuid>::None);
    let (revenue_reload_nonce, set_revenue_reload_nonce) = signal(0u64);
    let (revenue_edit_month, set_revenue_edit_month) = signal(Option::<u32>::None);
    let (revenue_edit_amount, set_revenue_edit_amount) = signal(String::new());

    let (pnl_project_id, set_pnl_project_id) = signal(Option::<Uuid>::None);
    let (pnl_year, set_pnl_year) = signal(chrono::Utc::now().year());
    let (pnl_reload_nonce, set_pnl_reload_nonce) = signal(0u64);
    let pnl_target_ref: NodeRef<html::Input> = NodeRef::new();
    let pnl_alert_ref: NodeRef<html::Input> = NodeRef::new();
    let (pnl_hover_month, set_pnl_hover_month) = signal(Option::<u32>::None);
    let (show_forecast, set_show_forecast) = signal(false);

    let forecast_resource = LocalResource::new(move || async move {
        let project_id = pnl_project_id.get();
        let year = pnl_year.get();
        let _nonce = pnl_reload_nonce.get();
        let show = show_forecast.get();

        if !show {
            return None;
        }

        match project_id {
            Some(pid) => Some(fetch_pl_forecast(pid, year).await),
            None => None,
        }
    });

    Effect::new(move |_| {
        if show_forecast.get() && pnl_project_id.get().is_some() {
            let interval = Interval::new(30_000, move || {
                set_pnl_reload_nonce.update(|value| *value += 1);
            });
            // Store interval to prevent it from being dropped immediately.
            // Using StoredValue::new_local since Interval is !Send+!Sync on WASM.
            let _keep = StoredValue::new_local(Some(interval));
        }
    });

    let forecast_data = Signal::derive(move || {
        forecast_resource
            .get()
            .and_then(|result| result)
            .and_then(|result| result.ok())
    });

    let pnl_resource = LocalResource::new(move || async move {
        let project_id = pnl_project_id.get();
        let year = pnl_year.get();
        let _nonce = pnl_reload_nonce.get();

        match project_id {
            Some(pid) => Some(fetch_pl_dashboard(pid, year).await),
            None => None,
        }
    });

    let pnl_data = Signal::derive(move || {
        pnl_resource
            .get()
            .and_then(|result| result)
            .and_then(|result| result.ok())
    });

    Effect::new(move |_| {
        if let Some(Some(Err(e))) = pnl_resource.get() {
            set_error.set(Some(e));
        }
    });

    Effect::new(move |_| {
        if let Some(Some(Err(e))) = forecast_resource.get() {
            set_error.set(Some(e));
        }
    });

    let revenue_grid_resource = LocalResource::new(move || async move {
        let project_id = revenue_project_id.get();
        let year = revenue_year.get();
        let _reload_nonce = revenue_reload_nonce.get();

        match project_id {
            Some(pid) => Some(fetch_project_revenue(pid, year).await),
            None => None,
        }
    });

    let revenue_grid = Signal::derive(move || {
        revenue_grid_resource
            .get()
            .and_then(|result| result)
            .and_then(|result| result.ok())
    });

    let revenue_save_action = Action::new_local(move |payload: &RevenueSavePayload| {
        let payload = payload.clone();
        async move { upsert_project_revenue(payload.project_id, payload.request).await }
    });

    let revenue_saving = Signal::derive(move || revenue_save_action.pending().get());

    Effect::new(move |_| {
        if let Some(Some(Err(e))) = revenue_grid_resource.get() {
            set_error.set(Some(e));
        }
    });

    Effect::new(move |_| {
        if let Some(result) = revenue_save_action.value().get() {
            match result {
                Ok(_) => {
                    set_revenue_edit_month.set(None);
                    set_revenue_edit_amount.set(String::new());
                    set_revenue_reload_nonce.update(|value| *value += 1);
                    set_pnl_reload_nonce.update(|value| *value += 1);
                }
                Err(e) => set_error.set(Some(e)),
            }
        }
    });

    #[derive(Debug, Clone)]
    struct PlSettingsPayload {
        project_id: Uuid,
        target_margin_pct: f64,
        margin_alert_threshold_pct: f64,
    }

    let pnl_settings_action = Action::new_local(move |payload: &PlSettingsPayload| {
        let payload = payload.clone();
        async move {
            update_pl_settings(
                payload.project_id,
                payload.target_margin_pct,
                payload.margin_alert_threshold_pct,
            )
            .await
        }
    });

    Effect::new(move |_| {
        if let Some(result) = pnl_settings_action.value().get() {
            match result {
                Ok(_) => set_pnl_reload_nonce.update(|v| *v += 1),
                Err(e) => set_error.set(Some(e)),
            }
        }
    });

    let (expense_category, set_expense_category) = signal(String::from("hr"));
    let (expense_description, set_expense_description) = signal(String::new());
    let (expense_amount, set_expense_amount) = signal(String::new());
    let (expense_date, set_expense_date) = signal(String::new());
    let (expense_vendor, set_expense_vendor) = signal(String::new());
    let (expense_edit_reason, set_expense_edit_reason) = signal(String::new());

    // Load projects on mount
    Effect::new(move |_| {
        set_loading.set(true);
        leptos::task::spawn_local(async move {
            match fetch_projects().await {
                Ok(data) => {
                    set_projects.set(data);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    });

    // Handle create/edit project
    let handle_submit = move |form_data: ProjectFormData| {
        let editing = editing_project.get();
        let current_user_id = auth.user.get().map(|u| u.id);
        leptos::task::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let result = if let Some(project) = editing {
                let manager_id_for_update = project.project_manager_id.or(current_user_id);
                update_project(project.id, form_data, manager_id_for_update).await
            } else {
                create_project(form_data, current_user_id).await
            };

            match result {
                Ok(_) => {
                    // Reload projects
                    match fetch_projects().await {
                        Ok(data) => {
                            set_projects.set(data);
                            set_show_form.set(false);
                            set_editing_project.set(None);
                        }
                        Err(e) => set_error.set(Some(e)),
                    }
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    // Handle delete project
    let handle_delete = move |id: Uuid| {
        leptos::task::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            match delete_project(id).await {
                Ok(_) => {
                    // Reload projects
                    match fetch_projects().await {
                        Ok(data) => set_projects.set(data),
                        Err(e) => set_error.set(Some(e)),
                    }
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    // Handle edit click
    let handle_edit = move |id: Uuid| {
        if let Some(project) = projects.get().iter().find(|p| p.id == id).cloned() {
            set_editing_project.set(Some(project));
            set_show_form.set(true);
        }
    };

    let handle_view_budget = move |id: Uuid| {
        leptos::task::spawn_local(async move {
            match fetch_project_budget(id).await {
                Ok(data) => set_selected_budget.set(Some(data)),
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    // Handle cancel
    let handle_cancel = move |_| {
        set_show_form.set(false);
        set_editing_project.set(None);
    };

    let handle_view_resource_costs = move |id: Uuid| {
        leptos::task::spawn_local(async move {
            match fetch_resource_costs(id).await {
                Ok(data) => set_resource_costs.set(Some(data)),
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let handle_view_revenue = move |id: Uuid| {
        set_revenue_project_id.set(Some(id));
        set_revenue_edit_month.set(None);
        set_revenue_edit_amount.set(String::new());
    };

    let handle_view_pnl = move |id: Uuid| {
        set_pnl_project_id.set(Some(id));
        set_pnl_year.set(chrono::Utc::now().year());
    };

    let handle_revenue_save = move |month: u32| {
        let project_id = match revenue_project_id.get() {
            Some(id) => id,
            None => return,
        };
        let amount_str = revenue_edit_amount.get();
        let amount: i64 = match amount_str.trim().parse() {
            Ok(v) if v >= 0 => v,
            _ => {
                set_error.set(Some("Amount must be a non-negative integer".to_string()));
                return;
            }
        };
        let year = revenue_year.get();
        let revenue_month = format!("{}-{:02}", year, month);
        let override_erp = revenue_grid
            .get()
            .and_then(|g| g.months.iter().find(|m| m.month == month).cloned())
            .and_then(|m| m.source_type)
            .map(|s| s == "erp_synced")
            .unwrap_or(false);
        let req = UpsertProjectRevenueRequest {
            revenue_month,
            amount_idr: amount,
            override_erp,
            source_reference: None,
        };
        revenue_save_action.dispatch(RevenueSavePayload {
            project_id,
            request: req,
        });
    };

    let handle_view_expenses = move |id: Uuid| {
        if let Some(project) = projects.get().iter().find(|p| p.id == id).cloned() {
            set_selected_project_for_expenses.set(Some(project));
            set_show_expense_form.set(false);
            leptos::task::spawn_local(async move {
                set_loading.set(true);
                match fetch_project_expenses(id).await {
                    Ok(data) => {
                        let mut sorted_data = data;
                        sorted_data.sort_by(|a, b| b.expense_date.cmp(&a.expense_date));
                        set_expenses.set(sorted_data);
                    }
                    Err(e) => set_error.set(Some(e)),
                }
                set_loading.set(false);
            });
        }
    };

    let handle_cancel_expense = move || {
        set_show_expense_form.set(false);
        set_editing_expense.set(None);
    };

    let reset_expense_form = move || {
        set_expense_category.set(String::from("hr"));
        set_expense_description.set(String::new());
        set_expense_amount.set(String::new());
        set_expense_date.set(String::new());
        set_expense_vendor.set(String::new());
        set_expense_edit_reason.set(String::new());
    };

    let handle_submit_expense = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let project_id = match selected_project_for_expenses.get() {
            Some(p) => p.id,
            None => return,
        };

        let editing = editing_expense.get();
        let is_edit = editing.is_some();
        let expense_id = editing.map(|e| e.id);

        let form_data = ExpenseEditData {
            category: expense_category.get(),
            description: expense_description.get(),
            amount_idr: expense_amount.get(),
            expense_date: expense_date.get(),
            vendor: expense_vendor.get(),
            edit_reason: expense_edit_reason.get(),
        };

        leptos::task::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let result = if is_edit {
                update_project_expense(project_id, expense_id.unwrap(), form_data).await
            } else {
                create_project_expense(
                    project_id,
                    ExpenseFormData {
                        category: form_data.category,
                        description: form_data.description,
                        amount_idr: form_data.amount_idr,
                        expense_date: form_data.expense_date,
                        vendor: form_data.vendor,
                    },
                )
                .await
            };

            match result {
                Ok(_) => {
                    match fetch_project_expenses(project_id).await {
                        Ok(data) => {
                            let mut sorted_data = data;
                            sorted_data.sort_by(|a, b| b.expense_date.cmp(&a.expense_date));
                            set_expenses.set(sorted_data);
                            set_show_expense_form.set(false);
                            set_editing_expense.set(None);
                        }
                        Err(e) => set_error.set(Some(e)),
                    }

                    match fetch_projects().await {
                        Ok(data) => set_projects.set(data),
                        Err(e) => set_error.set(Some(e)),
                    }

                    if let Some(budget) = selected_budget.get() {
                        if budget.project_id == project_id {
                            match fetch_project_budget(project_id).await {
                                Ok(data) => set_selected_budget.set(Some(data)),
                                Err(_) => {}
                            }
                        }
                    }

                    set_pnl_reload_nonce.update(|value| *value += 1);
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    let handle_delete_expense = move |expense_id: Uuid| {
        let window = web_sys::window().expect("no window");
        let confirmed = window
            .confirm_with_message(
                "Are you sure you want to delete this expense? This action cannot be undone.",
            )
            .unwrap_or(false);
        if !confirmed {
            return;
        }
        let project_id = match selected_project_for_expenses.get() {
            Some(p) => p.id,
            None => return,
        };

        leptos::task::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            match delete_project_expense(project_id, expense_id).await {
                Ok(_) => {
                    match fetch_project_expenses(project_id).await {
                        Ok(data) => {
                            let mut sorted_data = data;
                            sorted_data.sort_by(|a, b| b.expense_date.cmp(&a.expense_date));
                            set_expenses.set(sorted_data);
                        }
                        Err(e) => set_error.set(Some(e)),
                    }

                    match fetch_projects().await {
                        Ok(data) => set_projects.set(data),
                        Err(e) => set_error.set(Some(e)),
                    }

                    if let Some(budget) = selected_budget.get() {
                        if budget.project_id == project_id {
                            match fetch_project_budget(project_id).await {
                                Ok(data) => set_selected_budget.set(Some(data)),
                                Err(_) => {}
                            }
                        }
                    }

                    set_pnl_reload_nonce.update(|value| *value += 1);
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    let handle_edit_expense_click = move |expense: ProjectExpenseData| {
        set_expense_category.set(expense.category.clone());
        set_expense_description.set(expense.description.clone());
        set_expense_amount.set(expense.amount_idr.to_string());
        set_expense_date.set(expense.expense_date.clone());
        set_expense_vendor.set(expense.vendor.clone().unwrap_or_default());
        set_expense_edit_reason.set(String::new());
        set_editing_expense.set(Some(expense));
        set_show_expense_form.set(true);
    };

    let is_editing_expense = Signal::derive(move || editing_expense.get().is_some());
    let expense_submit_callback = Callback::new(handle_submit_expense);
    let expense_cancel_callback = Callback::new(move |_| handle_cancel_expense());

    view! {
        <div class="h-full">

            <div class="page-container fade-in">
                <div class="space-y-4">
                    <div class="page-header">
                        <h1 class="text-xl font-semibold text-huly-caption">
                            "Projects"
                        </h1>
                        <button
                            class="btn-primary btn-press"
                            on:click=move |_| {
                                set_editing_project.set(None);
                                set_show_form.set(true);
                            }
                        >
                            "Add Project"
                        </button>
                    </div>

                    {move || error.get().map(|err| {
                        view! {
                            <div class="alert-error">
                                <div class="flex">
                                    <div class="ml-3">
                                        <h3 class="text-sm font-medium text-negative-default">
                                            {err}
                                        </h3>
                                    </div>
                                </div>
                            </div>
                        }
                    })}

                    {move || {
                        if show_form.get() {
                            let initial_data = editing_project.get().map(|p| ProjectFormData {
                                name: p.name,
                                client: p.client.unwrap_or_default(),
                                description: p.description.unwrap_or_default(),
                                start_date: p.start_date.to_string(),
                                end_date: p.end_date.to_string(),
                                status: p.status,
                                total_budget_idr: p.total_budget_idr.to_string(),
                                budget_hr_idr: p.budget_hr_idr.to_string(),
                                budget_software_idr: p.budget_software_idr.to_string(),
                                budget_hardware_idr: p.budget_hardware_idr.to_string(),
                                budget_overhead_idr: p.budget_overhead_idr.to_string(),
                            });

                            Either::Left(view! {
                                <div class="card">
                                    <h2 class="text-xl font-semibold text-huly-caption mb-4">
                                        {if editing_project.get().is_some() { "Edit Project" } else { "Add Project" }}
                                    </h2>
                                    <ProjectForm
                                        initial_data=initial_data.unwrap_or_default()
                                        on_submit=Callback::new(handle_submit)
                                        on_cancel=Callback::new(handle_cancel)
                                    />
                                </div>
                            })
                        } else {
                            Either::Right(view! { <div>
                                {move || {
                                    if loading.get() {
                                        EitherOf3::A(view! {
                                            <div class="space-y-3">
                                                <div class="toolbar"><div class="skeleton-text w-32 h-3"></div></div>
                                                <div class="panel overflow-hidden">
                                                    <div class="skeleton-row"><div class="skeleton-text w-28"></div><div class="skeleton-text w-20"></div><div class="skeleton-text w-16"></div><div class="skeleton-text w-24"></div></div>
                                                    <div class="skeleton-row"><div class="skeleton-text w-24"></div><div class="skeleton-text w-16"></div><div class="skeleton-text w-20"></div><div class="skeleton-text w-20"></div></div>
                                                    <div class="skeleton-row"><div class="skeleton-text w-32"></div><div class="skeleton-text w-12"></div><div class="skeleton-text w-24"></div><div class="skeleton-text w-16"></div></div>
                                                </div>
                                            </div>
                                        })
                                    } else if projects.get().is_empty() {
                                        EitherOf3::B(view! {
                                            <div class="empty-state py-12">
                                                <svg class="w-12 h-12 text-huly-ghost mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1">
                                                    <path stroke-linecap="round" stroke-linejoin="round" d="M9 12h3.75M9 15h3.75M9 18h3.75m3 .75H18a2.25 2.25 0 002.25-2.25V6.108c0-1.135-.845-2.098-1.976-2.192a48.424 48.424 0 00-1.123-.08m-5.801 0c-.065.21-.1.433-.1.664 0 .414.336.75.75.75h4.5a.75.75 0 00.75-.75 2.25 2.25 0 00-.1-.664m-5.8 0A2.251 2.251 0 0113.5 2.25H15c1.012 0 1.867.668 2.15 1.586m-5.8 0c-.376.023-.75.05-1.124.08C9.095 4.01 8.25 4.973 8.25 6.108V8.25m0 0H4.875c-.621 0-1.125.504-1.125 1.125v11.25c0 .621.504 1.125 1.125 1.125h9.75c.621 0 1.125-.504 1.125-1.125V9.375c0-.621-.504-1.125-1.125-1.125H8.25z" />
                                                </svg>
                                                <p class="text-huly-secondary text-sm">"No projects found."</p>
                                                <p class="text-huly-muted text-xs mt-1">"Click 'Add Project' to create one."</p>
                                            </div>
                                        })
                                    } else {
                                        EitherOf3::C(view! {
                                            <ProjectList
                                                projects=projects.into()
                                                on_edit=Callback::new(handle_edit)
                                                on_delete=Callback::new(handle_delete)
                                                on_view_budget=Callback::new(handle_view_budget)
                                                on_view_expenses=Callback::new(handle_view_expenses)
                                                on_view_resource_costs=Callback::new(handle_view_resource_costs)
                                                on_view_revenue=Callback::new(handle_view_revenue)
                                                on_view_pnl=Callback::new(handle_view_pnl)
                                            />
                                            {move || {
                                                selected_budget.get().map(|budget| {
                                                    view! {
                                                        <div class="panel mt-6">
                                                            <div class="toolbar">
                                                                <h2 class="text-xl font-semibold text-huly-caption">
                                                                    {format!("Budget Summary - {}", budget.project_name)}
                                                                </h2>
                                                                <button
                                                                    class="text-huly-ghost hover:text-huly-content"
                                                                    on:click=move |_| set_selected_budget.set(None)
                                                                >
                                                                    "Close"
                                                                </button>
                                                            </div>

                                                            <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
                                                                <div class="bg-primary-600/10 rounded-lg p-4">
                                                                    <p class="text-sm text-primary-400">"Total Budget"</p>
                                                                    <p class="text-2xl font-bold text-huly-caption">{format_idr(budget.total_budget_idr)}</p>
                                                                </div>
                                                                <div class="bg-positive-default/10 rounded-lg p-4">
                                                                    <p class="text-sm text-positive-default">"Spent"</p>
                                                                    <p class="text-2xl font-bold text-huly-caption">{format_idr(budget.spent_to_date_idr)}</p>
                                                                </div>
                                                                <div class="bg-accent-orange/10 rounded-lg p-4">
                                                                    <p class="text-sm text-accent-orange">"Remaining"</p>
                                                                    <p class="text-2xl font-bold text-huly-caption">{format_idr(budget.remaining_idr)}</p>
                                                                </div>
                                                            </div>

                                                            <div class="toolbar">
                                                                <h3 class="text-sm font-medium text-huly-secondary">"Category Breakdown"</h3>
                                                            </div>
                                                            <div class="overflow-x-auto">
                                                                <table class="min-w-full divide-y divide-huly-divider">
                                                                    <thead class="table-head">
                                                                        <tr>
                                                                            <th class="th-cell-compact">"Category"</th>
                                                                            <th class="th-cell-compact text-right">"Total"</th>
                                                                            <th class="th-cell-compact text-right">"Spent"</th>
                                                                            <th class="th-cell-compact text-right">"Remaining"</th>
                                                                            <th class="th-cell-compact text-right">"%"</th>
                                                                        </tr>
                                                                    </thead>
                                                                    <tbody class="divide-y divide-huly-divider">
                                                                        <tr>
                                                                            <td class="td-cell-compact text-huly-secondary">"HR"</td>
                                                                            <td class="td-cell-compact text-right font-medium text-huly-caption">{format_idr(budget.budget_hr_idr)}</td>
                                                                            <td class="td-cell-compact text-right text-huly-content">{format_idr(budget.spent_hr_idr)}</td>
                                                                            <td class="td-cell-compact text-right text-huly-content">{format_idr(budget.remaining_hr_idr)}</td>
                                                                            <td class="td-cell-compact text-right text-huly-muted">{format!("{:.1}%", budget.hr_pct)}</td>
                                                                        </tr>
                                                                        <tr>
                                                                            <td class="td-cell-compact text-huly-secondary">"Software"</td>
                                                                            <td class="td-cell-compact text-right font-medium text-huly-caption">{format_idr(budget.budget_software_idr)}</td>
                                                                            <td class="td-cell-compact text-right text-huly-content">{format_idr(budget.spent_software_idr)}</td>
                                                                            <td class="td-cell-compact text-right text-huly-content">{format_idr(budget.remaining_software_idr)}</td>
                                                                            <td class="td-cell-compact text-right text-huly-muted">{format!("{:.1}%", budget.software_pct)}</td>
                                                                        </tr>
                                                                        <tr>
                                                                            <td class="td-cell-compact text-huly-secondary">"Hardware"</td>
                                                                            <td class="td-cell-compact text-right font-medium text-huly-caption">{format_idr(budget.budget_hardware_idr)}</td>
                                                                            <td class="td-cell-compact text-right text-huly-content">{format_idr(budget.spent_hardware_idr)}</td>
                                                                            <td class="td-cell-compact text-right text-huly-content">{format_idr(budget.remaining_hardware_idr)}</td>
                                                                            <td class="td-cell-compact text-right text-huly-muted">{format!("{:.1}%", budget.hardware_pct)}</td>
                                                                        </tr>
                                                                        <tr>
                                                                            <td class="td-cell-compact text-huly-secondary">"Overhead"</td>
                                                                            <td class="td-cell-compact text-right font-medium text-huly-caption">{format_idr(budget.budget_overhead_idr)}</td>
                                                                            <td class="td-cell-compact text-right text-huly-content">{format_idr(budget.spent_overhead_idr)}</td>
                                                                            <td class="td-cell-compact text-right text-huly-content">{format_idr(budget.remaining_overhead_idr)}</td>
                                                                            <td class="td-cell-compact text-right text-huly-muted">{format!("{:.1}%", budget.overhead_pct)}</td>
                                                                        </tr>
                                                                    </tbody>
                                                                </table>
                                                            </div>
                                                        </div>
                                                    }
                                                })
                                            }}
                                            {move || {
                                                resource_costs.get().map(|costs| {
                                                    view! {
                                                        <div class="panel mt-6">
                                                            <div class="toolbar">
                                                                <h2 class="text-xl font-semibold text-huly-caption">
                                                                    "Resource Costs"
                                                                </h2>
                                                                <div class="flex items-center space-x-4">
                                                                    <span class="text-lg font-bold text-huly-caption">{format_idr(costs.total_resource_cost_idr)}</span>
                                                                    <button
                                                                        class="text-huly-ghost hover:text-huly-content"
                                                                        on:click=move |_| set_resource_costs.set(None)
                                                                    >
                                                                        "Close"
                                                                    </button>
                                                                </div>
                                                            </div>

                                                            // Employee table
                                                            <div class="overflow-x-auto mb-6">
                                                                <table class="min-w-full divide-y divide-huly-divider">
                                                                    <thead class="table-head">
                                                                        <tr>
                                                                            <th class="th-cell-compact">"Employee"</th>
                                                                            <th class="th-cell-compact text-right">"Daily Rate"</th>
                                                                            <th class="th-cell-compact text-right">"Days Allocated"</th>
                                                                            <th class="th-cell-compact text-right">"Total Cost"</th>
                                                                        </tr>
                                                                    </thead>
                                                                    <tbody class="divide-y divide-huly-divider">
                                                                        {costs.employees.into_iter().map(|emp| {
                                                                            let rate_display = if emp.missing_rate {
                                                                                "Rate unavailable".to_string()
                                                                            } else {
                                                                                emp.daily_rate_idr.map(|r| format_idr(r)).unwrap_or_else(|| "N/A".to_string())
                                                                            };
                                                                            let rate_class = if emp.missing_rate {
                                                                                "td-cell-compact text-right text-accent-orange italic"
                                                                            } else {
                                                                                "td-cell-compact text-right text-huly-caption"
                                                                            };
                                                                            let note = emp.rate_change_note.clone();
                                                                            view! {
                                                                                <tr class="table-row-hover">
                                                                                    <td class="td-cell-compact text-huly-caption">
                                                                                        {emp.resource_name}
                                                                                        {if emp.has_rate_change {
                                                                                            Either::Left(view! { <span class="ml-2 badge-warning">"Rate Changed"</span> })
                                                                                        } else {
                                                                                            Either::Right(view! { <span></span> })
                                                                                        }}
                                                                                    </td>
                                                                                    <td class=rate_class>{rate_display}</td>
                                                                                    <td class="td-cell-compact text-right text-huly-caption">{emp.days_allocated}</td>
                                                                                    <td class="td-cell-compact text-right font-medium text-huly-caption">{format_idr(emp.total_cost_idr)}</td>
                                                                                </tr>
                                                                                {note.map(|n| view! {
                                                                                    <tr class="bg-warning-default/10">
                                                                                        <td colspan="4" class="px-4 py-1 text-xs text-warning-default italic">{n}</td>
                                                                                    </tr>
                                                                                })}
                                                                            }
                                                                        }).collect_view()}
                                                                    </tbody>
                                                                </table>
                                                            </div>

                                                            // Monthly breakdown
                                                            {if !costs.monthly_breakdown.is_empty() {
                                                                Either::Left(view! {
                                                                    <div>
                                                                        <h3 class="text-lg font-medium text-huly-caption mb-3">"Monthly Breakdown"</h3>
                                                                        <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
                                                                            {costs.monthly_breakdown.into_iter().map(|m| {
                                                                                view! {
                                                                                    <div class="bg-huly-surface-2 rounded-lg p-3">
                                                                                        <p class="text-sm font-medium text-huly-secondary">{m.month}</p>
                                                                                        <p class="text-lg font-bold text-huly-caption">{format_idr(m.cost_idr)}</p>
                                                                                        <p class="text-xs text-huly-muted">{format!("{} working days", m.working_days)}</p>
                                                                                    </div>
                                                                                }
                                                                            }).collect_view()}
                                                                        </div>
                                                                    </div>
                                                                })
                                                            } else {
                                                                Either::Right(view! { <div></div> })
                                                            }}
                                                        </div>
                                                    }
                                                })
                                            }}
                                        })
                                    }
                                }}
                                {move || {
                                    revenue_grid.get().map(|grid| {
                                        let year = grid.year;
                                        let ytd = grid.ytd_total_idr;
                                        view! {
                                            <div class="panel mt-6">
                                                            <div class="toolbar">
                                                    <h2 class="text-xl font-semibold text-huly-caption">
                                                        "Revenue"
                                                    </h2>
                                                    <div class="flex items-center space-x-4">
                                                        <button
                                                            class="text-huly-secondary hover:text-huly-caption px-2"
                                                            on:click=move |_| {
                                                                let new_year = revenue_year.get() - 1;
                                                                set_revenue_year.set(new_year);
                                                            }
                                                        >
                                                            "◀"
                                                        </button>
                                                        <span class="text-lg font-bold text-huly-caption">{year}</span>
                                                        <button
                                                            class="text-huly-secondary hover:text-huly-caption px-2"
                                                            on:click=move |_| {
                                                                let new_year = revenue_year.get() + 1;
                                                                set_revenue_year.set(new_year);
                                                            }
                                                        >
                                                            "▶"
                                                        </button>
                                                        <button
                                                            class="text-huly-ghost hover:text-huly-content"
                                                            on:click=move |_| {
                                                                set_revenue_project_id.set(None);
                                                                set_revenue_edit_month.set(None);
                                                                set_revenue_edit_amount.set(String::new());
                                                            }
                                                        >
                                                            "Close"
                                                        </button>
                                                    </div>
                                                </div>

                                                <div class="overflow-x-auto mb-4">
                                                    <table class="min-w-full divide-y divide-huly-divider">
                                                        <thead class="table-head">
                                                            <tr>
                                                                <th class="th-cell-compact">"Month"</th>
                                                                <th class="th-cell-compact text-right">"Amount (IDR)"</th>
                                                                <th class="th-cell-compact text-center">"Source"</th>
                                                                <th class="th-cell-compact text-center">"Entry Date"</th>
                                                                <th class="th-cell-compact text-center">"Entered By"</th>
                                                                <th class="th-cell-compact text-right">"Actions"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody class="divide-y divide-huly-divider">
                                                            {grid.months.into_iter().map(|entry| {
                                                                let month_num = entry.month;
                                                                let is_erp = entry.source_type.as_deref() == Some("erp_synced");
                                                                let has_data = entry.revenue_id.is_some();
                                                                let source_badge = match entry.source_type.as_deref() {
                                                                    Some("manual") => view! { <span class="badge-positive">"Manual"</span> },
                                                                    Some("erp_synced") => view! { <span class="badge-primary">"ERP Synced"</span> },
                                                                    Some("manual_override") => view! { <span class="badge-warning">"Override"</span> },
                                                                    _ => view! { <span class="text-xs text-huly-ghost">"—"</span> },
                                                                };
                                                                let entry_date_str = entry.entry_date.unwrap_or_else(|| "—".to_string());
                                                                let entered_by_str = entry
                                                                    .entered_by
                                                                    .map(|user_id| user_id.to_string())
                                                                    .unwrap_or_else(|| "—".to_string());
                                                                view! {
                                                                    <tr class="table-row-hover">
                                                                        <td class="td-cell-compact font-medium text-huly-caption">{entry.month_label}</td>
                                                                        <td class="td-cell-compact text-right text-huly-caption">
                                                                            {move || {
                                                                                if revenue_edit_month.get() == Some(month_num) {
                                                                                    Either::Left(view! {
                                                                                        <input
                                                                                            type="number"
                                                                                            class="input w-32 text-right"
                                                                                            prop:value=revenue_edit_amount
                                                                                            on:input=move |ev| set_revenue_edit_amount.set(event_target_value(&ev))
                                                                                        />
                                                                                    })
                                                                                } else {
                                                                                    Either::Right(view! { <span>{format_idr(entry.amount_idr)}</span> })
                                                                                }
                                                                            }}
                                                                        </td>
                                                                        <td class="td-cell-compact text-center">{source_badge}</td>
                                                                        <td class="td-cell-compact text-center text-huly-muted">{entry_date_str}</td>
                                                                        <td class="td-cell-compact text-center font-mono text-huly-muted">{entered_by_str}</td>
                                                                        <td class="td-cell-compact text-right font-medium">
                                                                            {move || {
                                                                                if revenue_edit_month.get() == Some(month_num) {
                                                                                    EitherOf3::A(view! {
                                                                                        <button
                                                                                            class="link mr-2"
                                                                                            prop:disabled=revenue_saving
                                                                                            on:click=move |_| handle_revenue_save(month_num)
                                                                                        >
                                                                                            "Save"
                                                                                        </button>
                                                                                        <button
                                                                                            class="text-huly-muted hover:text-huly-content"
                                                                                            on:click=move |_| set_revenue_edit_month.set(None)
                                                                                        >
                                                                                            "Cancel"
                                                                                        </button>
                                                                                    })
                                                                                } else if is_erp {
                                                                                    EitherOf3::B(view! {
                                                                                        <button
                                                                                            class="link-warning"
                                                                                            on:click=move |_| {
                                                                                                set_revenue_edit_month.set(Some(month_num));
                                                                                                set_revenue_edit_amount.set(entry.amount_idr.to_string());
                                                                                            }
                                                                                        >
                                                                                            "Override"
                                                                                        </button>
                                                                                    })
                                                                                } else {
                                                                                    EitherOf3::C(view! {
                                                                                        <button
                                                                                            class="link"
                                                                                            on:click=move |_| {
                                                                                                set_revenue_edit_month.set(Some(month_num));
                                                                                                set_revenue_edit_amount.set(if has_data { entry.amount_idr.to_string() } else { String::new() });
                                                                                            }
                                                                                        >
                                                                                            {if has_data { "Edit" } else { "Enter" }}
                                                                                        </button>
                                                                                    })
                                                                                }
                                                                            }}
                                                                        </td>
                                                                    </tr>
                                                                }
                                                            }).collect_view()}
                                                        </tbody>
                                                    </table>
                                                </div>

                                                <div class="flex justify-between items-center p-3 bg-primary-600/10 rounded">
                                                    <span class="text-sm font-medium text-primary-300">"Year-to-Date Total"</span>
                                                    <span class="text-lg font-bold text-primary-200">{format_idr(ytd)}</span>
                                                </div>
                                            </div>
                                        }
                                    })
                                }}
                                {move || {
                                    pnl_data.get().map(|pnl| {
                                        let year = pnl.year;
                                        let margin_color = if pnl.margin_alert.is_some() {
                                            "text-negative-default"
                                        } else {
                                            "text-positive-default"
                                        };

                                        let months_for_chart = pnl.months.clone();
                                        let months_for_tooltip = pnl.months.clone();
                                        let months_for_table = pnl.months.clone();

                                        let mut max_val: i64 = 1;
                                        for m in &pnl.months {
                                            if m.revenue_idr > max_val { max_val = m.revenue_idr; }
                                            if m.total_cost_idr > max_val { max_val = m.total_cost_idr; }
                                        }

                                        let chart_height = 150.0;

                                        view! {
                                            <div class="panel mt-6">
                                                {pnl.margin_alert.clone().map(|alert| view! {
                                                    <div class="alert-error mb-4 flex items-center">
                                                        <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path></svg>
                                                        <span class="font-medium">"Alert: "</span> <span class="ml-1">{alert}</span>
                                                    </div>
                                                })}

                                                <div class="toolbar">
                                                    <h2 class="text-xl font-semibold text-huly-caption">
                                                        "P&L Dashboard"
                                                    </h2>
                                                    <div class="flex items-center space-x-4">
                                                        <button
                                                            class="text-huly-secondary hover:text-huly-caption px-2"
                                                            on:click=move |_| set_pnl_year.update(|y| *y -= 1)
                                                        >
                                                            "◀"
                                                        </button>
                                                        <span class="text-lg font-bold text-huly-caption">{year}</span>
                                                        <button
                                                            class="text-huly-secondary hover:text-huly-caption px-2"
                                                            on:click=move |_| set_pnl_year.update(|y| *y += 1)
                                                        >
                                                            "▶"
                                                        </button>
                                                        <button
                                                            class=move || if show_forecast.get() {
                                                                "px-3 py-1 text-sm font-medium rounded-md bg-primary-600 text-huly-caption"
                                                            } else {
                                                                "px-3 py-1 text-sm font-medium rounded-md bg-huly-btn-default text-huly-content border border-huly-btn-border hover:bg-huly-btn-hover"
                                                            }
                                                            on:click=move |_| set_show_forecast.update(|v| *v = !*v)
                                                            aria-pressed=move || show_forecast.get().to_string()
                                                        >
                                                            "Forecast"
                                                        </button>
                                                        <button
                                                            class="text-huly-ghost hover:text-huly-content"
                                                            on:click=move |_| set_pnl_project_id.set(None)
                                                        >
                                                            "Close"
                                                        </button>
                                                    </div>
                                                </div>

                                                <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
                                                    <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                        <p class="text-sm text-huly-muted mb-1">"Revenue"</p>
                                                        <p class="text-xl font-bold text-huly-caption">{format_idr(pnl.total_revenue_idr)}</p>
                                                    </div>
                                                    <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                        <p class="text-sm text-huly-muted mb-1">"Total Costs"</p>
                                                        <p class="text-xl font-bold text-huly-caption">{format_idr(pnl.total_cost_idr)}</p>
                                                    </div>
                                                    <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                        <p class="text-sm text-huly-muted mb-1">"Gross Profit"</p>
                                                        <p class="text-xl font-bold text-huly-caption">{format_idr(pnl.gross_profit_idr)}</p>
                                                    </div>
                                                    <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                        <p class="text-sm text-huly-muted mb-1">"Margin %"</p>
                                                        <p class=format!("text-xl font-bold {}", margin_color)>{format!("{:.1}%", pnl.margin_pct)}</p>
                                                    </div>
                                                </div>

                                                <div class="mb-6 border border-huly-divider rounded-lg p-4">
                                                    <h3 class="text-sm font-medium text-huly-content mb-4">"Revenue vs Cost (Monthly)"</h3>
                                                    <div class="w-full overflow-x-auto">
                                                        <svg
                                                            width="100%"
                                                            height="160"
                                                            viewBox="0 0 1200 160"
                                                            preserveAspectRatio="none"
                                                            role="img"
                                                            aria-label="Monthly revenue versus total cost chart"
                                                        >
                                                            {months_for_chart.iter().enumerate().map(|(i, m)| {
                                                                let x_offset = i as f64 * 100.0 + 20.0;
                                                                let rev_h = (m.revenue_idr as f64 / max_val as f64 * chart_height).max(1.0);
                                                                let cost_h = (m.total_cost_idr as f64 / max_val as f64 * chart_height).max(1.0);

                                                                let rev_y = 150.0 - rev_h;
                                                                let cost_y = 150.0 - cost_h;
                                                                let month_num = m.month;
                                                                let month_label = m.month_label.clone();
                                                                let month_label_for_rev = month_label.clone();
                                                                let month_label_for_cost = month_label.clone();

                                                                view! {
                                                                    <g transform=format!("translate({}, 0)", x_offset)>
                                                                        <rect
                                                                            x="0"
                                                                            y=rev_y
                                                                            width="20"
                                                                            height=rev_h
                                                                            fill="#10b981"
                                                                            rx="2"
                                                                            ry="2"
                                                                            opacity="0.8"
                                                                            tabindex="0"
                                                                            focusable="true"
                                                                            aria-label=format!(
                                                                                "{} revenue {}",
                                                                                month_label_for_rev,
                                                                                format_idr(m.revenue_idr)
                                                                            )
                                                                            on:mouseenter=move |_| set_pnl_hover_month.set(Some(month_num))
                                                                            on:mouseleave=move |_| set_pnl_hover_month.set(None)
                                                                            on:focus=move |_| set_pnl_hover_month.set(Some(month_num))
                                                                            on:blur=move |_| set_pnl_hover_month.set(None)
                                                                        />
                                                                        <rect
                                                                            x="22"
                                                                            y=cost_y
                                                                            width="20"
                                                                            height=cost_h
                                                                            fill="#ef4444"
                                                                            rx="2"
                                                                            ry="2"
                                                                            opacity="0.8"
                                                                            tabindex="0"
                                                                            focusable="true"
                                                                            aria-label=format!(
                                                                                "{} total cost {}",
                                                                                month_label_for_cost,
                                                                                format_idr(m.total_cost_idr)
                                                                            )
                                                                            on:mouseenter=move |_| set_pnl_hover_month.set(Some(month_num))
                                                                            on:mouseleave=move |_| set_pnl_hover_month.set(None)
                                                                            on:focus=move |_| set_pnl_hover_month.set(Some(month_num))
                                                                            on:blur=move |_| set_pnl_hover_month.set(None)
                                                                        />
                                                                        <text x="21" y="160" text-anchor="middle" class="text-xs fill-huly-muted text-[10px]">{month_label}</text>
                                                                    </g>
                                                                }
                                                            }).collect_view()}
                                                        </svg>
                                                        {move || {
                                                            pnl_hover_month
                                                                .get()
                                                                .and_then(|hovered| {
                                                                    months_for_tooltip
                                                                        .iter()
                                                                        .find(|entry| entry.month == hovered)
                                                                        .cloned()
                                                                })
                                                                .map(|entry| {
                                                                    view! {
                                                                        <div class="mt-3 bg-huly-tooltip-bg text-huly-content rounded-md px-3 py-2 text-xs space-y-1" role="status" aria-live="polite">
                                                                            <p><span class="font-semibold">"Revenue: "</span>{format_idr(entry.revenue_idr)}</p>
                                                                            <p><span class="font-semibold">"Resource Costs: "</span>{format_idr(entry.resource_cost_idr)}</p>
                                                                            <p><span class="font-semibold">"Non-Resource Costs: "</span>{format_idr(entry.non_resource_cost_idr)}</p>
                                                                            <p><span class="font-semibold">"Margin: "</span>{format!("{:.1}%", entry.margin_pct)}</p>
                                                                        </div>
                                                                    }
                                                                })
                                                        }}
                                                    </div>
                                                </div>

                                                <div class="overflow-x-auto mb-6">
                                                    <table class="min-w-full divide-y divide-huly-divider">
                                                        <thead class="table-head">
                                                            <tr>
                                                                <th class="th-cell-compact">"Month"</th>
                                                                <th class="th-cell-compact text-right">"Revenue"</th>
                                                                <th class="th-cell-compact text-right">"Res. Cost"</th>
                                                                <th class="th-cell-compact text-right">"Non-Res. Cost"</th>
                                                                <th class="th-cell-compact text-right">"Total Cost"</th>
                                                                <th class="th-cell-compact text-right">"Gross Profit"</th>
                                                                <th class="th-cell-compact text-right">"Margin %"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody class="divide-y divide-huly-divider">
                                                            {months_for_table.into_iter().map(|m| {
                                                                let m_color = if m.revenue_idr > 0 && (pnl.target_margin_pct - m.margin_pct) > pnl.margin_alert_threshold_pct {
                                                                    "text-negative-default font-medium"
                                                                } else {
                                                                    "text-huly-caption"
                                                                };
                                                                view! {
                                                                    <tr class="table-row-hover">
                                                                        <td class="td-cell-compact font-medium text-huly-caption">{m.month_label}</td>
                                                                        <td class="td-cell-compact text-right text-huly-secondary">{format_idr(m.revenue_idr)}</td>
                                                                        <td class="td-cell-compact text-right text-huly-secondary">{format_idr(m.resource_cost_idr)}</td>
                                                                        <td class="td-cell-compact text-right text-huly-secondary">{format_idr(m.non_resource_cost_idr)}</td>
                                                                        <td class="td-cell-compact text-right text-huly-secondary">{format_idr(m.total_cost_idr)}</td>
                                                                        <td class="td-cell-compact text-right font-medium text-huly-caption">{format_idr(m.gross_profit_idr)}</td>
                                                                        <td class=format!("td-cell-compact text-right {}", m_color)>{format!("{:.1}%", m.margin_pct)}</td>
                                                                    </tr>
                                                                }
                                                            }).collect_view()}
                                                        </tbody>
                                                    </table>
                                                </div>

                                                <div class="bg-huly-surface-2 p-4 rounded-lg border border-huly-divider">
                                                    <h3 class="text-sm font-medium text-huly-caption mb-3">"Target Margin Settings"</h3>
                                                    <div class="flex items-end space-x-4">
                                                        <div>
                                                            <label class="label text-xs">"Target Margin (%)"</label>
                                                            <input type="number" step="0.1" node_ref=pnl_target_ref class="input w-32" prop:value=pnl.target_margin_pct.to_string() />
                                                        </div>
                                                        <div>
                                                            <label class="label text-xs">"Alert Threshold (%)"</label>
                                                            <input type="number" step="0.1" node_ref=pnl_alert_ref class="input w-32" prop:value=pnl.margin_alert_threshold_pct.to_string() />
                                                        </div>
                                                        <button
                                                            class="btn-primary btn-press"
                                                            on:click=move |_| {
                                                                let t_val = pnl_target_ref.get().map(|i| i.value()).unwrap_or_default();
                                                                let a_val = pnl_alert_ref.get().map(|i| i.value()).unwrap_or_default();
                                                                let target_pct: f64 = match t_val.trim().parse() {
                                                                    Ok(v) => v,
                                                                    Err(_) => {
                                                                        set_error.set(Some("Target margin must be a valid number".to_string()));
                                                                        return;
                                                                    }
                                                                };
                                                                let alert_pct: f64 = match a_val.trim().parse() {
                                                                    Ok(v) => v,
                                                                    Err(_) => {
                                                                        set_error.set(Some("Alert threshold must be a valid number".to_string()));
                                                                        return;
                                                                    }
                                                                };

                                                                if !(0.0..=100.0).contains(&target_pct) {
                                                                    set_error.set(Some("Target margin must be between 0 and 100".to_string()));
                                                                    return;
                                                                }
                                                                if !(0.0..=100.0).contains(&alert_pct) {
                                                                    set_error.set(Some("Alert threshold must be between 0 and 100".to_string()));
                                                                    return;
                                                                }

                                                                pnl_settings_action.dispatch(PlSettingsPayload {
                                                                    project_id: pnl.project_id,
                                                                    target_margin_pct: target_pct,
                                                                    margin_alert_threshold_pct: alert_pct,
                                                                });
                                                            }
                                                        >
                                                            "Save Settings"
                                                        </button>
                                                    </div>
                                                </div>

                                                {move || {
                                                    if !show_forecast.get() {
                                                        return None;
                                                    }
                                                    forecast_data.get().map(|fc| {
                                                        let variance_color = if fc.variance_from_target_pct >= 0.0 {
                                                            "text-positive-default"
                                                        } else {
                                                            "text-negative-default"
                                                        };
                                                        let margin_color = if fc.forecast_margin_pct >= fc.target_margin_pct {
                                                            "text-positive-default"
                                                        } else {
                                                            "text-negative-default"
                                                        };
                                                        let categories = fc.categories.clone();
                                                        let drivers = fc.resource_drivers.clone();

                                                        view! {
                                                            <div class="panel mt-6 border-l-4 border-primary-500">
                                                                <h3 class="text-lg font-semibold text-huly-caption mb-1">"Profitability Forecast"</h3>
                                                                <p class="text-xs text-huly-muted mb-4">
                                                                    {format!("As of {} · {} of {} project days elapsed", fc.as_of_date, fc.elapsed_days, fc.total_project_days)}
                                                                </p>

                                                                <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
                                                                    <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                                        <p class="text-sm text-huly-muted mb-1">"Current Spend"</p>
                                                                        <p class="text-xl font-bold text-huly-caption">{format_idr(fc.current_spend_idr)}</p>
                                                                    </div>
                                                                    <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                                        <p class="text-sm text-huly-muted mb-1">"Projected Total"</p>
                                                                        <p class="text-xl font-bold text-huly-caption">{format_idr(fc.projected_total_cost_idr)}</p>
                                                                    </div>
                                                                    <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                                        <p class="text-sm text-huly-muted mb-1">"Forecast Margin"</p>
                                                                        <p class=format!("text-xl font-bold {}", margin_color)>{format!("{:.1}%", fc.forecast_margin_pct)}</p>
                                                                    </div>
                                                                    <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                                        <p class="text-sm text-huly-muted mb-1">"Variance from Target"</p>
                                                                        <p class=format!("text-xl font-bold {}", variance_color)>{format!("{:+.1}%", fc.variance_from_target_pct)}</p>
                                                                    </div>
                                                                </div>

                                                                <div class="mb-6">
                                                                    <div class="toolbar">
                                                                        <h4 class="text-sm font-medium text-huly-secondary">"Cost Category Forecast"</h4>
                                                                    </div>
                                                                    <div class="overflow-x-auto">
                                                                        <table class="min-w-full divide-y divide-huly-divider">
                                                                            <thead class="table-head">
                                                                                <tr>
                                                                                    <th class="th-cell-compact">"Category"</th>
                                                                                    <th class="th-cell-compact text-right">"Budget"</th>
                                                                                    <th class="th-cell-compact text-right">"Current Spend"</th>
                                                                                    <th class="th-cell-compact text-right">"Projected"</th>
                                                                                    <th class="th-cell-compact text-right">"Overrun"</th>
                                                                                    <th class="th-cell-compact text-center">"Status"</th>
                                                                                </tr>
                                                                            </thead>
                                                                            <tbody class="divide-y divide-huly-divider">
                                                                                {categories.into_iter().map(|cat| {
                                                                                    let status_badge = if cat.overrun_idr > 0 {
                                                                                        ("Overrun", "badge-negative")
                                                                                    } else if cat.budget_idr > 0 && cat.projected_idr as f64 > cat.budget_idr as f64 * 0.9 {
                                                                                        ("At Risk", "badge-warning")
                                                                                    } else {
                                                                                        ("On Track", "badge-positive")
                                                                                    };
                                                                                    view! {
                                                                                        <tr class="table-row-hover">
                                                                                            <td class="td-cell-compact font-medium text-huly-caption capitalize">{cat.category}</td>
                                                                                            <td class="td-cell-compact text-right text-huly-secondary">{format_idr(cat.budget_idr)}</td>
                                                                                            <td class="td-cell-compact text-right text-huly-secondary">{format_idr(cat.current_spend_idr)}</td>
                                                                                            <td class="td-cell-compact text-right text-huly-secondary">{format_idr(cat.projected_idr)}</td>
                                                                                            <td class="td-cell-compact text-right font-medium text-negative-default">
                                                                                                {if cat.overrun_idr > 0 { format_idr(cat.overrun_idr) } else { "—".to_string() }}
                                                                                            </td>
                                                                                            <td class="td-cell-compact text-center">
                                                                                                <span class=format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", status_badge.1)>
                                                                                                    {status_badge.0}
                                                                                                </span>
                                                                                            </td>
                                                                                        </tr>
                                                                                    }
                                                                                }).collect_view()}
                                                                            </tbody>
                                                                        </table>
                                                                    </div>
                                                                </div>

                                                                {if !drivers.is_empty() {
                                                                    Some(view! {
                                                                        <div>
                                                                            <h4 class="text-sm font-medium text-huly-content mb-3">"Top Resource Cost Drivers"</h4>
                                                                            <div class="space-y-2">
                                                                                {drivers.into_iter().map(|d| {
                                                                                    view! {
                                                                                        <div class="flex items-center justify-between py-2 px-3 bg-huly-surface-2 rounded">
                                                                                            <span class="text-sm font-medium text-huly-caption">{d.resource_name}</span>
                                                                                            <div class="flex items-center space-x-4">
                                                                                                <span class="text-sm text-huly-secondary">{format_idr(d.total_cost_idr)}</span>
                                                                                                <span class="text-sm font-medium text-primary-400">{format!("{:.1}%", d.share_pct)}</span>
                                                                                            </div>
                                                                                        </div>
                                                                                    }
                                                                                }).collect_view()}
                                                                            </div>
                                                                        </div>
                                                                    })
                                                                } else {
                                                                    None
                                                                }}
                                                            </div>
                                                        }
                                                    })
                                                }}
                                            </div>
                                        }
                                    })
                                }}
                                {move || {
                                    selected_project_for_expenses.get().map(|project| {
                                        view! {
                                            <div class="panel mt-6">
                                                <div class="toolbar">
                                                    <h2 class="text-xl font-semibold text-huly-caption">
                                                        {format!("Expenses - {}", project.name)}
                                                    </h2>
                                                    <div class="space-x-2">
                                                        <button
                                                            class="btn-primary text-sm"
                                                            on:click=move |_| {
                                                                reset_expense_form();
                                                                set_editing_expense.set(None);
                                                                set_show_expense_form.set(true);
                                                            }
                                                        >
                                                            "Add Expense"
                                                        </button>
                                                        <button
                                                            class="text-huly-ghost hover:text-huly-content"
                                                            on:click=move |_| set_selected_project_for_expenses.set(None)
                                                        >
                                                            "Close"
                                                        </button>
                                                    </div>
                                                </div>

                                                {move || if show_expense_form.get() {
                                                    Either::Left(view! {
                                                        <ExpenseFormPanel
                                                            category=expense_category
                                                            set_category=set_expense_category
                                                            amount=expense_amount
                                                            set_amount=set_expense_amount
                                                            description=expense_description
                                                            set_description=set_expense_description
                                                            date_value=expense_date
                                                            set_date_value=set_expense_date
                                                            vendor=expense_vendor
                                                            set_vendor=set_expense_vendor
                                                            edit_reason=expense_edit_reason
                                                            set_edit_reason=set_expense_edit_reason
                                                            is_editing=is_editing_expense
                                                            loading=loading
                                                            on_submit=expense_submit_callback
                                                            on_cancel=expense_cancel_callback
                                                        />
                                                    })
                                                        
                                                } else {
                                                    Either::Right(view! { <div></div> })
                                                }}

                                                <div class="overflow-x-auto">
                                                    <table class="min-w-full divide-y divide-huly-divider">
                                                        <thead class="table-head">
                                                            <tr>
                                                                <th class="th-cell-compact">"Date"</th>
                                                                <th class="th-cell-compact">"Category"</th>
                                                                <th class="th-cell-compact">"Description"</th>
                                                                <th class="th-cell-compact">"Vendor"</th>
                                                                <th class="th-cell-compact text-right">"Amount"</th>
                                                                <th class="th-cell-compact text-right">"Actions"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody class="bg-huly-surface divide-y divide-huly-divider">
                                                            {move || {
                                                                if expenses.get().is_empty() {
                                                                    Either::Left(view! {
                                                                        <tr>
                                                                            <td colspan="6" class="td-cell-compact text-huly-muted text-center">
                                                                                "No expenses found for this project."
                                                                            </td>
                                                                        </tr>
                                                                    })
                                                                } else {
                                                                    Either::Right(expenses.get().into_iter().map(|expense| {
                                                                        let exp_id = expense.id;
                                                                        let exp_clone = expense.clone();
                                                                        view! {
                                                                            <tr class="table-row-hover">
                                                                                <td class="td-cell-compact text-huly-muted">{expense.expense_date}</td>
                                                                                <td class="td-cell-compact text-huly-caption capitalize">{expense.category}</td>
                                                                                <td class="td-cell-compact text-huly-caption max-w-xs truncate">{expense.description}</td>
                                                                                <td class="td-cell-compact text-huly-muted">{expense.vendor.unwrap_or_default()}</td>
                                                                                <td class="td-cell-compact font-medium text-right text-huly-caption">{format_idr(expense.amount_idr)}</td>
                                                                                <td class="td-cell-compact text-right">
                                                                                    <button
                                                                                        class="link mr-3"
                                                                                        on:click=move |_| handle_edit_expense_click(exp_clone.clone())
                                                                                    >
                                                                                        "Edit"
                                                                                    </button>
                                                                                    <button
                                                                                        class="link-danger"
                                                                                        on:click=move |_| handle_delete_expense(exp_id)
                                                                                    >
                                                                                        "Delete"
                                                                                    </button>
                                                                                </td>
                                                                            </tr>
                                                                        }
                                                                    }).collect_view())
                                                                }
                                                            }}
                                                        </tbody>
                                                    </table>
                                                </div>
                                            </div>
                                        }})
                                }}
                            </div> })
                        }
                    }}
                </div>
            </div>

        </div>
    }
}

/// Fetch all projects from API
async fn fetch_projects() -> Result<Vec<Project>, String> {
    let response = authenticated_get("/api/v1/projects")
        .await
        .map_err(|e| format!("Failed to fetch projects: {}", e))?;

    if response.status().is_success() {
        response
            .json::<Vec<Project>>()
            .await
            .map_err(|e| format!("Failed to parse projects: {}", e))
    } else {
        Err(format!("Failed to fetch projects: {}", response.status()))
    }
}

async fn fetch_project_budget(project_id: Uuid) -> Result<ProjectBudgetData, String> {
    let response = authenticated_get(&format!("/api/v1/projects/{}/budget", project_id))
        .await
        .map_err(|e| format!("Failed to fetch budget: {}", e))?;

    if response.status().is_success() {
        response
            .json::<ProjectBudgetData>()
            .await
            .map_err(|e| format!("Failed to parse budget: {}", e))
    } else {
        Err(format!("Failed to fetch budget: {}", response.status()))
    }
}

fn parse_budget_input(raw: &str, field_name: &str) -> Result<i64, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(0);
    }

    let value = trimmed
        .parse::<i64>()
        .map_err(|_| format!("{} must be a whole number", field_name))?;
    if value < 0 {
        return Err(format!("{} cannot be negative", field_name));
    }

    Ok(value)
}

/// Create a new project
async fn create_project(
    form_data: ProjectFormData,
    current_user_id: Option<Uuid>,
) -> Result<(), String> {
    let project_manager_id =
        current_user_id.ok_or_else(|| "Unable to determine authenticated user".to_string())?;

    let start_date = NaiveDate::parse_from_str(&form_data.start_date, "%Y-%m-%d")
        .map_err(|_| "Invalid start date".to_string())?;
    let end_date = NaiveDate::parse_from_str(&form_data.end_date, "%Y-%m-%d")
        .map_err(|_| "Invalid end date".to_string())?;
    if end_date < start_date {
        return Err("End date must be on or after start date".to_string());
    }

    let total_budget_idr = parse_budget_input(&form_data.total_budget_idr, "Total budget")?;
    let budget_hr_idr = parse_budget_input(&form_data.budget_hr_idr, "HR budget")?;
    let budget_software_idr =
        parse_budget_input(&form_data.budget_software_idr, "Software budget")?;
    let budget_hardware_idr =
        parse_budget_input(&form_data.budget_hardware_idr, "Hardware budget")?;
    let budget_overhead_idr =
        parse_budget_input(&form_data.budget_overhead_idr, "Overhead budget")?;

    if total_budget_idr <= 0 {
        return Err("Total budget must be greater than 0".to_string());
    }
    let budget_sum =
        budget_hr_idr + budget_software_idr + budget_hardware_idr + budget_overhead_idr;
    if budget_sum != total_budget_idr {
        return Err(format!(
            "Budget categories sum ({}) must equal total budget ({})",
            budget_sum, total_budget_idr
        ));
    }

    let response = authenticated_post_json(
        "/api/v1/projects",
        &serde_json::json!({
            "name": form_data.name,
            "client": if form_data.client.is_empty() { None } else { Some(form_data.client) },
            "description": if form_data.description.is_empty() { None } else { Some(form_data.description) },
            "start_date": start_date,
            "end_date": end_date,
            "status": "Active",
            "project_manager_id": project_manager_id,
            "total_budget_idr": total_budget_idr,
            "budget_hr_idr": budget_hr_idr,
            "budget_software_idr": budget_software_idr,
            "budget_hardware_idr": budget_hardware_idr,
            "budget_overhead_idr": budget_overhead_idr
        }),
    )
        .await
        .map_err(|e| format!("Failed to create project: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to create project: {}", error_text))
    }
}

/// Update an existing project
async fn update_project(
    id: Uuid,
    form_data: ProjectFormData,
    project_manager_id: Option<Uuid>,
) -> Result<(), String> {
    let start_date = NaiveDate::parse_from_str(&form_data.start_date, "%Y-%m-%d")
        .map_err(|_| "Invalid start date".to_string())?;
    let end_date = NaiveDate::parse_from_str(&form_data.end_date, "%Y-%m-%d")
        .map_err(|_| "Invalid end date".to_string())?;
    if end_date < start_date {
        return Err("End date must be on or after start date".to_string());
    }

    let total_budget_idr = parse_budget_input(&form_data.total_budget_idr, "Total budget")?;
    let budget_hr_idr = parse_budget_input(&form_data.budget_hr_idr, "HR budget")?;
    let budget_software_idr =
        parse_budget_input(&form_data.budget_software_idr, "Software budget")?;
    let budget_hardware_idr =
        parse_budget_input(&form_data.budget_hardware_idr, "Hardware budget")?;
    let budget_overhead_idr =
        parse_budget_input(&form_data.budget_overhead_idr, "Overhead budget")?;

    if total_budget_idr <= 0 {
        return Err("Total budget must be greater than 0".to_string());
    }
    let budget_sum =
        budget_hr_idr + budget_software_idr + budget_hardware_idr + budget_overhead_idr;
    if budget_sum != total_budget_idr {
        return Err(format!(
            "Budget categories sum ({}) must equal total budget ({})",
            budget_sum, total_budget_idr
        ));
    }

    let response = authenticated_put_json(
        &format!("/api/v1/projects/{}", id),
        &serde_json::json!({
            "name": form_data.name,
            "client": if form_data.client.is_empty() { None } else { Some(form_data.client) },
            "description": if form_data.description.is_empty() { None } else { Some(form_data.description) },
            "start_date": start_date,
            "end_date": end_date,
            "status": form_data.status,
            "project_manager_id": project_manager_id,
            "total_budget_idr": total_budget_idr,
            "budget_hr_idr": budget_hr_idr,
            "budget_software_idr": budget_software_idr,
            "budget_hardware_idr": budget_hardware_idr,
            "budget_overhead_idr": budget_overhead_idr
        }),
    )
        .await
        .map_err(|e| format!("Failed to update project: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to update project: {}", error_text))
    }
}

/// Delete a project
async fn delete_project(id: Uuid) -> Result<(), String> {
    let response = authenticated_delete(&format!("/api/v1/projects/{}", id))
        .await
        .map_err(|e| format!("Failed to delete project: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to delete project: {}", error_text))
    }
}

async fn fetch_project_expenses(project_id: Uuid) -> Result<Vec<ProjectExpenseData>, String> {
    let response = authenticated_get(&format!("/api/v1/projects/{}/expenses", project_id))
        .await
        .map_err(|e| format!("Failed to fetch expenses: {}", e))?;

    if response.status().is_success() {
        response
            .json::<Vec<ProjectExpenseData>>()
            .await
            .map_err(|e| format!("Failed to parse expenses: {}", e))
    } else {
        Err(format!("Failed to fetch expenses: {}", response.status()))
    }
}

async fn create_project_expense(project_id: Uuid, data: ExpenseFormData) -> Result<(), String> {
    let amount_idr = parse_budget_input(&data.amount_idr, "Amount")?;
    if amount_idr <= 0 {
        return Err("Amount must be greater than 0".to_string());
    }
    let _ = NaiveDate::parse_from_str(&data.expense_date, "%Y-%m-%d")
        .map_err(|_| "Invalid expense date".to_string())?;

    if data.category.trim().is_empty() {
        return Err("Category is required".to_string());
    }
    if data.description.trim().is_empty() {
        return Err("Description is required".to_string());
    }

    let response = authenticated_post_json(
        &format!("/api/v1/projects/{}/expenses", project_id),
        &serde_json::json!({
            "category": data.category,
            "description": data.description,
            "amount_idr": amount_idr,
            "expense_date": data.expense_date,
            "vendor": if data.vendor.trim().is_empty() { None } else { Some(data.vendor.trim().to_string()) }
        }),
    )
    .await
    .map_err(|e| format!("Failed to create expense: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to create expense: {}", error_text))
    }
}

async fn update_project_expense(
    project_id: Uuid,
    expense_id: Uuid,
    data: ExpenseEditData,
) -> Result<(), String> {
    let amount_idr = parse_budget_input(&data.amount_idr, "Amount")?;
    if amount_idr <= 0 {
        return Err("Amount must be greater than 0".to_string());
    }
    let _ = NaiveDate::parse_from_str(&data.expense_date, "%Y-%m-%d")
        .map_err(|_| "Invalid expense date".to_string())?;

    if data.category.trim().is_empty() {
        return Err("Category is required".to_string());
    }
    if data.description.trim().is_empty() {
        return Err("Description is required".to_string());
    }
    if data.edit_reason.trim().is_empty() {
        return Err("Edit reason is required".to_string());
    }

    let response = authenticated_put_json(
        &format!("/api/v1/projects/{}/expenses/{}", project_id, expense_id),
        &serde_json::json!({
            "category": data.category,
            "description": data.description,
            "amount_idr": amount_idr,
            "expense_date": data.expense_date,
            "vendor": if data.vendor.trim().is_empty() { "".to_string() } else { data.vendor.trim().to_string() },
            "edit_reason": data.edit_reason
        }),
    )
    .await
    .map_err(|e| format!("Failed to update expense: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to update expense: {}", error_text))
    }
}

async fn delete_project_expense(project_id: Uuid, expense_id: Uuid) -> Result<(), String> {
    let response = authenticated_delete(&format!(
        "/api/v1/projects/{}/expenses/{}",
        project_id, expense_id
    ))
    .await
    .map_err(|e| format!("Failed to delete expense: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to delete expense: {}", error_text))
    }
}

async fn fetch_resource_costs(project_id: Uuid) -> Result<ResourceCostData, String> {
    let response = authenticated_get(&format!("/api/v1/projects/{}/resource-costs", project_id))
        .await
        .map_err(|e| format!("Failed to fetch resource costs: {}", e))?;

    if response.status().is_success() {
        response
            .json::<ResourceCostData>()
            .await
            .map_err(|e| format!("Failed to parse resource costs: {}", e))
    } else {
        Err(format!(
            "Failed to fetch resource costs: {}",
            response.status()
        ))
    }
}

async fn fetch_project_revenue(
    project_id: Uuid,
    year: i32,
) -> Result<ProjectRevenueGridResponse, String> {
    let response = authenticated_get(&format!(
        "/api/v1/projects/{}/revenue?year={}",
        project_id, year
    ))
    .await
    .map_err(|e| format!("Failed to fetch revenue: {}", e))?;

    if response.status().is_success() {
        response
            .json::<ProjectRevenueGridResponse>()
            .await
            .map_err(|e| format!("Failed to parse revenue: {}", e))
    } else {
        Err(format!("Failed to fetch revenue: {}", response.status()))
    }
}

async fn upsert_project_revenue(
    project_id: Uuid,
    req: UpsertProjectRevenueRequest,
) -> Result<(), String> {
    let response =
        authenticated_post_json(&format!("/api/v1/projects/{}/revenue", project_id), &req)
            .await
            .map_err(|e| format!("Failed to save revenue: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to save revenue: {}", error_text))
    }
}

async fn fetch_pl_dashboard(
    project_id: Uuid,
    year: i32,
) -> Result<ProjectPlDashboardResponse, String> {
    let response = authenticated_get(&format!("/api/v1/projects/{}/pl?year={}", project_id, year))
        .await
        .map_err(|e| format!("Failed to fetch P&L: {}", e))?;

    if response.status().is_success() {
        response
            .json::<ProjectPlDashboardResponse>()
            .await
            .map_err(|e| format!("Failed to parse P&L: {}", e))
    } else {
        Err(format!("Failed to fetch P&L: {}", response.status()))
    }
}

async fn update_pl_settings(
    project_id: Uuid,
    target_margin_pct: f64,
    margin_alert_threshold_pct: f64,
) -> Result<serde_json::Value, String> {
    let response = authenticated_put_json(
        &format!("/api/v1/projects/{}/pl/settings", project_id),
        &serde_json::json!({
            "target_margin_pct": target_margin_pct,
            "margin_alert_threshold_pct": margin_alert_threshold_pct,
        }),
    )
    .await
    .map_err(|e| format!("Failed to update P&L settings: {}", e))?;

    if response.status().is_success() {
        response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Failed to parse settings response: {}", e))
    } else {
        Err(format!("Failed to update settings: {}", response.status()))
    }
}

async fn fetch_pl_forecast(
    project_id: Uuid,
    year: i32,
) -> Result<ProjectPlForecastResponse, String> {
    let response = authenticated_get(&format!(
        "/api/v1/projects/{}/pl/forecast?year={}",
        project_id, year
    ))
    .await
    .map_err(|e| format!("Failed to fetch forecast: {}", e))?;

    if response.status().is_success() {
        response
            .json::<ProjectPlForecastResponse>()
            .await
            .map_err(|e| format!("Failed to parse forecast: {}", e))
    } else {
        Err(format!("Failed to fetch forecast: {}", response.status()))
    }
}
