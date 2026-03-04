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
                                        }.into_view()
                                    }
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
