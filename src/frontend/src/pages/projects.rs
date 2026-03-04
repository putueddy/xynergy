use crate::auth::{
    authenticated_delete, authenticated_get, authenticated_post_json, authenticated_put_json,
    use_auth,
};
use crate::components::project_list::Project;
use crate::components::{project_form::ProjectFormData, Footer, Header, ProjectForm, ProjectList};
use chrono::NaiveDate;
use leptos::*;
use leptos_router::*;
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectExpenseData {
    pub id: Uuid,
    pub project_id: Uuid,
    pub category: String,
    pub description: String,
    pub amount_idr: i64,
    pub expense_date: String,  // NaiveDate serializes as string
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
            class="space-y-4 mb-8 bg-gray-50 dark:bg-gray-700/50 p-4 rounded-lg"
            on:submit=move |ev| on_submit.call(ev)
        >
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">"Category"</label>
                    <select
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
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
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">"Amount (IDR)"</label>
                    <input
                        type="text"
                        inputmode="numeric"
                        pattern="[0-9]*"
                        autocomplete="off"
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                        on:input=move |ev| set_amount.set(event_target_value(&ev))
                        prop:value=amount
                        required
                    />
                </div>
                <div class="md:col-span-2">
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">"Description"</label>
                    <input
                        type="text"
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                        on:input=move |ev| set_description.set(event_target_value(&ev))
                        prop:value=description
                        required
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">"Date"</label>
                    <input
                        type="date"
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                        on:input=move |ev| set_date_value.set(event_target_value(&ev))
                        prop:value=date_value
                        required
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">"Vendor (Optional)"</label>
                    <input
                        type="text"
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                        on:input=move |ev| set_vendor.set(event_target_value(&ev))
                        prop:value=vendor
                    />
                </div>
                {move || if is_editing.get() {
                    view! {
                        <div class="md:col-span-2">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">"Edit Reason"</label>
                            <input
                                type="text"
                                class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                on:input=move |ev| set_edit_reason.set(event_target_value(&ev))
                                prop:value=edit_reason
                                required
                            />
                        </div>
                    }
                        .into_view()
                } else {
                    view! { <div></div> }.into_view()
                }}
            </div>
            <div class="flex justify-end space-x-3 pt-4">
                <button
                    type="button"
                    class="btn-secondary"
                    on:click=move |_| on_cancel.call(())
                >
                    "Cancel"
                </button>
                <button
                    type="submit"
                    class="btn-primary"
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
        create_effect(move |_| {
            if !auth.is_authenticated.get() {
                navigate("/login", Default::default());
            }
        });
    }

    // Project data
    let (projects, set_projects) = create_signal(Vec::new());
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);
    let (show_form, set_show_form) = create_signal(false);
    let (editing_project, set_editing_project) = create_signal(Option::<Project>::None);
    let (selected_budget, set_selected_budget) = create_signal(Option::<ProjectBudgetData>::None);
    let (selected_project_for_expenses, set_selected_project_for_expenses) = create_signal(Option::<Project>::None);
    let (expenses, set_expenses) = create_signal(Vec::<ProjectExpenseData>::new());
    let (show_expense_form, set_show_expense_form) = create_signal(false);
    let (editing_expense, set_editing_expense) = create_signal(Option::<ProjectExpenseData>::None);
    let (resource_costs, set_resource_costs) = create_signal(Option::<ResourceCostData>::None);

    let (expense_category, set_expense_category) = create_signal(String::from("hr"));
    let (expense_description, set_expense_description) = create_signal(String::new());
    let (expense_amount, set_expense_amount) = create_signal(String::new());
    let (expense_date, set_expense_date) = create_signal(String::new());
    let (expense_vendor, set_expense_vendor) = create_signal(String::new());
    let (expense_edit_reason, set_expense_edit_reason) = create_signal(String::new());

    // Load projects on mount
    create_effect(move |_| {
        set_loading.set(true);
        spawn_local(async move {
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
        spawn_local(async move {
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
        spawn_local(async move {
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
        spawn_local(async move {
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
        spawn_local(async move {
            match fetch_resource_costs(id).await {
                Ok(data) => set_resource_costs.set(Some(data)),
                Err(e) => set_error.set(Some(e)),
            }
        });
    };


    let handle_view_expenses = move |id: Uuid| {
        if let Some(project) = projects.get().iter().find(|p| p.id == id).cloned() {
            set_selected_project_for_expenses.set(Some(project));
            set_show_expense_form.set(false);
            spawn_local(async move {
                set_loading.set(true);
                match fetch_project_expenses(id).await {
                    Ok(data) => {
                        let mut sorted_data = data;
                        sorted_data.sort_by(|a, b| b.expense_date.cmp(&a.expense_date));
                        set_expenses.set(sorted_data);
                    },
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

        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let result = if is_edit {
                update_project_expense(project_id, expense_id.unwrap(), form_data).await
            } else {
                create_project_expense(project_id, ExpenseFormData {
                    category: form_data.category,
                    description: form_data.description,
                    amount_idr: form_data.amount_idr,
                    expense_date: form_data.expense_date,
                    vendor: form_data.vendor,
                }).await
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
                        },
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
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    let handle_delete_expense = move |expense_id: Uuid| {
        let project_id = match selected_project_for_expenses.get() {
            Some(p) => p.id,
            None => return,
        };
        
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            match delete_project_expense(project_id, expense_id).await {
                Ok(_) => {
                    match fetch_project_expenses(project_id).await {
                        Ok(data) => {
                            let mut sorted_data = data;
                            sorted_data.sort_by(|a, b| b.expense_date.cmp(&a.expense_date));
                            set_expenses.set(sorted_data);
                        },
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
        <div class="min-h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
            <Header/>

            <main class="flex-grow max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12 w-full">
                <div class="space-y-6">
                    <div class="flex items-center justify-between">
                        <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                            "Projects"
                        </h1>
                        <button
                            class="btn-primary"
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
                            <div class="rounded-md bg-red-50 p-4 dark:bg-red-900/20">
                                <div class="flex">
                                    <div class="ml-3">
                                        <h3 class="text-sm font-medium text-red-800 dark:text-red-200">
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

                            view! {
                                <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6">
                                    <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">
                                        {if editing_project.get().is_some() { "Edit Project" } else { "Add Project" }}
                                    </h2>
                                    <ProjectForm
                                        initial_data=initial_data.unwrap_or_default()
                                        on_submit=Callback::new(handle_submit)
                                        on_cancel=Callback::new(handle_cancel)
                                    />
                                </div>
                            }.into_view()
                        } else {
                            view! { <div>
                                {move || {
                                    if loading.get() {
                                        view! {
                                            <div class="text-center py-12">
                                                <div class="spinner mx-auto mb-4"></div>
                                                <p class="text-gray-600 dark:text-gray-400">"Loading projects..."</p>
                                            </div>
                                        }.into_view()
                                    } else if projects.get().is_empty() {
                                        view! {
                                            <div class="text-center py-12 bg-white dark:bg-gray-800 rounded-lg shadow">
                                                <p class="text-gray-600 dark:text-gray-400">"No projects found."</p>
                                                <p class="text-sm text-gray-500 dark:text-gray-500 mt-2">"Click 'Add Project' to create one."</p>
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <ProjectList
                                                projects=projects.into()
                                                on_edit=Callback::new(handle_edit)
                                                on_delete=Callback::new(handle_delete)
                                                on_view_budget=Callback::new(handle_view_budget)
                                                on_view_expenses=Callback::new(handle_view_expenses)
                                                on_view_resource_costs=Callback::new(handle_view_resource_costs)
                                            />
                                            {move || {
                                                selected_budget.get().map(|budget| {
                                                    view! {
                                                        <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 mt-6">
                                                            <div class="flex items-center justify-between mb-4">
                                                                <h2 class="text-xl font-semibold text-gray-900 dark:text-white">
                                                                    {format!("Budget Summary - {}", budget.project_name)}
                                                                </h2>
                                                                <button
                                                                    class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                                                                    on:click=move |_| set_selected_budget.set(None)
                                                                >
                                                                    "Close"
                                                                </button>
                                                            </div>

                                                            <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
                                                                <div class="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-4">
                                                                    <p class="text-sm text-blue-600 dark:text-blue-400">"Total Budget"</p>
                                                                    <p class="text-2xl font-bold text-blue-900 dark:text-blue-100">{format_idr(budget.total_budget_idr)}</p>
                                                                </div>
                                                                <div class="bg-green-50 dark:bg-green-900/20 rounded-lg p-4">
                                                                    <p class="text-sm text-green-600 dark:text-green-400">"Spent"</p>
                                                                    <p class="text-2xl font-bold text-green-900 dark:text-green-100">{format_idr(budget.spent_to_date_idr)}</p>
                                                                </div>
                                                                <div class="bg-orange-50 dark:bg-orange-900/20 rounded-lg p-4">
                                                                    <p class="text-sm text-orange-600 dark:text-orange-400">"Remaining"</p>
                                                                    <p class="text-2xl font-bold text-orange-900 dark:text-orange-100">{format_idr(budget.remaining_idr)}</p>
                                                                </div>
                                                            </div>

                                                            <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-3">"Category Breakdown"</h3>
                                                            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                                                <div class="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-700 rounded">
                                                                    <span class="text-sm text-gray-600 dark:text-gray-300">"HR"</span>
                                                                    <div class="text-right">
                                                                        <span class="text-sm font-medium text-gray-900 dark:text-white">{format_idr(budget.budget_hr_idr)}</span>
                                                                        <span class="text-xs text-gray-500 dark:text-gray-400 ml-2">{format!("{:.1}%", budget.hr_pct)}</span>
                                                                    </div>
                                                                </div>
                                                                <div class="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-700 rounded">
                                                                    <span class="text-sm text-gray-600 dark:text-gray-300">"Software"</span>
                                                                    <div class="text-right">
                                                                        <span class="text-sm font-medium text-gray-900 dark:text-white">{format_idr(budget.budget_software_idr)}</span>
                                                                        <span class="text-xs text-gray-500 dark:text-gray-400 ml-2">{format!("{:.1}%", budget.software_pct)}</span>
                                                                    </div>
                                                                </div>
                                                                <div class="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-700 rounded">
                                                                    <span class="text-sm text-gray-600 dark:text-gray-300">"Hardware"</span>
                                                                    <div class="text-right">
                                                                        <span class="text-sm font-medium text-gray-900 dark:text-white">{format_idr(budget.budget_hardware_idr)}</span>
                                                                        <span class="text-xs text-gray-500 dark:text-gray-400 ml-2">{format!("{:.1}%", budget.hardware_pct)}</span>
                                                                    </div>
                                                                </div>
                                                                <div class="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-700 rounded">
                                                                    <span class="text-sm text-gray-600 dark:text-gray-300">"Overhead"</span>
                                                                    <div class="text-right">
                                                                        <span class="text-sm font-medium text-gray-900 dark:text-white">{format_idr(budget.budget_overhead_idr)}</span>
                                                                        <span class="text-xs text-gray-500 dark:text-gray-400 ml-2">{format!("{:.1}%", budget.overhead_pct)}</span>
                                                                    </div>
                                                                </div>
                                                            </div>
                                                        </div>
                                                    }
                                                })
                                            }}
                                            {move || {
                                                resource_costs.get().map(|costs| {
                                                    view! {
                                                        <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 mt-6">
                                                            <div class="flex items-center justify-between mb-4">
                                                                <h2 class="text-xl font-semibold text-gray-900 dark:text-white">
                                                                    "Resource Costs"
                                                                </h2>
                                                                <div class="flex items-center space-x-4">
                                                                    <span class="text-lg font-bold text-gray-900 dark:text-white">{format_idr(costs.total_resource_cost_idr)}</span>
                                                                    <button
                                                                        class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                                                                        on:click=move |_| set_resource_costs.set(None)
                                                                    >
                                                                        "Close"
                                                                    </button>
                                                                </div>
                                                            </div>

                                                            // Employee table
                                                            <div class="overflow-x-auto mb-6">
                                                                <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                                                    <thead class="bg-gray-50 dark:bg-gray-700">
                                                                        <tr>
                                                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Employee"</th>
                                                                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Daily Rate"</th>
                                                                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Days"</th>
                                                                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Allocation"</th>
                                                                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Total Cost"</th>
                                                                        </tr>
                                                                    </thead>
                                                                    <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                                                                        {costs.employees.into_iter().map(|emp| {
                                                                            let rate_display = if emp.missing_rate {
                                                                                "Rate unavailable".to_string()
                                                                            } else {
                                                                                emp.daily_rate_idr.map(|r| format_idr(r)).unwrap_or_else(|| "N/A".to_string())
                                                                            };
                                                                            let rate_class = if emp.missing_rate {
                                                                                "px-4 py-3 whitespace-nowrap text-sm text-right text-amber-600 dark:text-amber-400 italic"
                                                                            } else {
                                                                                "px-4 py-3 whitespace-nowrap text-sm text-right text-gray-900 dark:text-white"
                                                                            };
                                                                            let note = emp.rate_change_note.clone();
                                                                            view! {
                                                                                <tr class="hover:bg-gray-50 dark:hover:bg-gray-700">
                                                                                    <td class="px-4 py-3 whitespace-nowrap text-sm text-gray-900 dark:text-white">
                                                                                        {emp.resource_name}
                                                                                        {if emp.has_rate_change {
                                                                                            view! { <span class="ml-2 inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200">"Rate Changed"</span> }.into_view()
                                                                                        } else {
                                                                                            view! { <span></span> }.into_view()
                                                                                        }}
                                                                                    </td>
                                                                                    <td class=rate_class>{rate_display}</td>
                                                                                    <td class="px-4 py-3 whitespace-nowrap text-sm text-right text-gray-900 dark:text-white">{emp.days_allocated}</td>
                                                                                    <td class="px-4 py-3 whitespace-nowrap text-sm text-right text-gray-900 dark:text-white">{format!("{:.0}%", emp.allocation_percentage)}</td>
                                                                                    <td class="px-4 py-3 whitespace-nowrap text-sm text-right font-medium text-gray-900 dark:text-white">{format_idr(emp.total_cost_idr)}</td>
                                                                                </tr>
                                                                                {note.map(|n| view! {
                                                                                    <tr class="bg-yellow-50 dark:bg-yellow-900/10">
                                                                                        <td colspan="5" class="px-4 py-1 text-xs text-yellow-700 dark:text-yellow-300 italic">{n}</td>
                                                                                    </tr>
                                                                                })}
                                                                            }
                                                                        }).collect_view()}
                                                                    </tbody>
                                                                </table>
                                                            </div>

                                                            // Monthly breakdown
                                                            {if !costs.monthly_breakdown.is_empty() {
                                                                view! {
                                                                    <div>
                                                                        <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-3">"Monthly Breakdown"</h3>
                                                                        <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
                                                                            {costs.monthly_breakdown.into_iter().map(|m| {
                                                                                view! {
                                                                                    <div class="bg-gray-50 dark:bg-gray-700 rounded-lg p-3">
                                                                                        <p class="text-sm font-medium text-gray-600 dark:text-gray-300">{m.month}</p>
                                                                                        <p class="text-lg font-bold text-gray-900 dark:text-white">{format_idr(m.cost_idr)}</p>
                                                                                        <p class="text-xs text-gray-500 dark:text-gray-400">{format!("{} working days", m.working_days)}</p>
                                                                                    </div>
                                                                                }
                                                                            }).collect_view()}
                                                                        </div>
                                                                    </div>
                                                                }.into_view()
                                                            } else {
                                                                view! { <div></div> }.into_view()
                                                            }}
                                                        </div>
                                                    }
                                                })
                                            }}
                                        }.into_view()
                                    }
                                }}
                                {move || {
                                    selected_project_for_expenses.get().map(|project| {
                                        view! {
                                            <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 mt-6">
                                                <div class="flex items-center justify-between mb-4">
                                                    <h2 class="text-xl font-semibold text-gray-900 dark:text-white">
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
                                                            class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                                                            on:click=move |_| set_selected_project_for_expenses.set(None)
                                                        >
                                                            "Close"
                                                        </button>
                                                    </div>
                                                </div>

                                                {move || if show_expense_form.get() {
                                                    view! {
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
                                                    }
                                                        .into_view()
                                                } else {
                                                    view! { <div></div> }.into_view()
                                                }}

                                                <div class="overflow-x-auto">
                                                    <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                                        <thead class="bg-gray-50 dark:bg-gray-700">
                                                            <tr>
                                                                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Date"</th>
                                                                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Category"</th>
                                                                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Description"</th>
                                                                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Vendor"</th>
                                                                <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Amount"</th>
                                                                <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Actions"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                                                            {move || {
                                                                if expenses.get().is_empty() {
                                                                    view! {
                                                                        <tr>
                                                                            <td colspan="6" class="px-4 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400 text-center">
                                                                                "No expenses found for this project."
                                                                            </td>
                                                                        </tr>
                                                                    }.into_view()
                                                                } else {
                                                                    expenses.get().into_iter().map(|expense| {
                                                                        let exp_id = expense.id;
                                                                        let exp_clone = expense.clone();
                                                                        view! {
                                                                            <tr class="hover:bg-gray-50 dark:hover:bg-gray-700">
                                                                                <td class="px-4 py-3 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">{expense.expense_date}</td>
                                                                                <td class="px-4 py-3 whitespace-nowrap text-sm text-gray-900 dark:text-white capitalize">{expense.category}</td>
                                                                                <td class="px-4 py-3 text-sm text-gray-900 dark:text-white max-w-xs truncate">{expense.description}</td>
                                                                                <td class="px-4 py-3 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">{expense.vendor.unwrap_or_default()}</td>
                                                                                <td class="px-4 py-3 whitespace-nowrap text-sm font-medium text-right text-gray-900 dark:text-white">{format_idr(expense.amount_idr)}</td>
                                                                                <td class="px-4 py-3 whitespace-nowrap text-right text-sm font-medium">
                                                                                    <button
                                                                                        class="text-blue-600 hover:text-blue-900 dark:text-blue-400 dark:hover:text-blue-300 mr-3"
                                                                                        on:click=move |_| handle_edit_expense_click(exp_clone.clone())
                                                                                    >
                                                                                        "Edit"
                                                                                    </button>
                                                                                    <button
                                                                                        class="text-red-600 hover:text-red-900 dark:text-red-400 dark:hover:text-red-300"
                                                                                        on:click=move |_| handle_delete_expense(exp_id)
                                                                                    >
                                                                                        "Delete"
                                                                                    </button>
                                                                                </td>
                                                                            </tr>
                                                                        }
                                                                    }).collect_view()
                                                                }
                                                            }}
                                                        </tbody>
                                                    </table>
                                                </div>
                                            </div>
                                        }
                                    })
                                }}
                            </div> }.into_view()
                        }
                    }}
                </div>
            </main>

            <Footer/>
        </div>
    }
}

/// Fetch all projects from API
async fn fetch_projects() -> Result<Vec<Project>, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/projects")
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
    let response = authenticated_get(&format!(
        "http://localhost:3000/api/v1/projects/{}/budget",
        project_id
    ))
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
async fn create_project(form_data: ProjectFormData, current_user_id: Option<Uuid>) -> Result<(), String> {
    let project_manager_id = current_user_id
        .ok_or_else(|| "Unable to determine authenticated user".to_string())?;

    let start_date = NaiveDate::parse_from_str(&form_data.start_date, "%Y-%m-%d")
        .map_err(|_| "Invalid start date".to_string())?;
    let end_date = NaiveDate::parse_from_str(&form_data.end_date, "%Y-%m-%d")
        .map_err(|_| "Invalid end date".to_string())?;
    if end_date < start_date {
        return Err("End date must be on or after start date".to_string());
    }

    let total_budget_idr = parse_budget_input(&form_data.total_budget_idr, "Total budget")?;
    let budget_hr_idr = parse_budget_input(&form_data.budget_hr_idr, "HR budget")?;
    let budget_software_idr = parse_budget_input(&form_data.budget_software_idr, "Software budget")?;
    let budget_hardware_idr = parse_budget_input(&form_data.budget_hardware_idr, "Hardware budget")?;
    let budget_overhead_idr = parse_budget_input(&form_data.budget_overhead_idr, "Overhead budget")?;

    if total_budget_idr <= 0 {
        return Err("Total budget must be greater than 0".to_string());
    }
    let budget_sum = budget_hr_idr + budget_software_idr + budget_hardware_idr + budget_overhead_idr;
    if budget_sum != total_budget_idr {
        return Err(format!(
            "Budget categories sum ({}) must equal total budget ({})",
            budget_sum, total_budget_idr
        ));
    }

    let response = authenticated_post_json(
        "http://localhost:3000/api/v1/projects",
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
    let budget_software_idr = parse_budget_input(&form_data.budget_software_idr, "Software budget")?;
    let budget_hardware_idr = parse_budget_input(&form_data.budget_hardware_idr, "Hardware budget")?;
    let budget_overhead_idr = parse_budget_input(&form_data.budget_overhead_idr, "Overhead budget")?;

    if total_budget_idr <= 0 {
        return Err("Total budget must be greater than 0".to_string());
    }
    let budget_sum = budget_hr_idr + budget_software_idr + budget_hardware_idr + budget_overhead_idr;
    if budget_sum != total_budget_idr {
        return Err(format!(
            "Budget categories sum ({}) must equal total budget ({})",
            budget_sum, total_budget_idr
        ));
    }

    let response = authenticated_put_json(
        &format!("http://localhost:3000/api/v1/projects/{}", id),
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
    let response = authenticated_delete(&format!("http://localhost:3000/api/v1/projects/{}", id))
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
    let response = authenticated_get(&format!(
        "http://localhost:3000/api/v1/projects/{}/expenses",
        project_id
    ))
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
        &format!("http://localhost:3000/api/v1/projects/{}/expenses", project_id),
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
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to create expense: {}", error_text))
    }
}

async fn update_project_expense(project_id: Uuid, expense_id: Uuid, data: ExpenseEditData) -> Result<(), String> {
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
        &format!("http://localhost:3000/api/v1/projects/{}/expenses/{}", project_id, expense_id),
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
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to update expense: {}", error_text))
    }
}

async fn delete_project_expense(project_id: Uuid, expense_id: Uuid) -> Result<(), String> {
    let response = authenticated_delete(&format!(
        "http://localhost:3000/api/v1/projects/{}/expenses/{}",
        project_id, expense_id
    ))
    .await
    .map_err(|e| format!("Failed to delete expense: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to delete expense: {}", error_text))
    }
}

async fn fetch_resource_costs(project_id: Uuid) -> Result<ResourceCostData, String> {
    let response = authenticated_get(&format!(
        "http://localhost:3000/api/v1/projects/{}/resource-costs",
        project_id
    ))
    .await
    .map_err(|e| format!("Failed to fetch resource costs: {}", e))?;

    if response.status().is_success() {
        response
            .json::<ResourceCostData>()
            .await
            .map_err(|e| format!("Failed to parse resource costs: {}", e))
    } else {
        Err(format!("Failed to fetch resource costs: {}", response.status()))
    }
}
