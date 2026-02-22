use crate::auth::{authenticated_delete, authenticated_get, authenticated_post_json, authenticated_put_json, use_auth};
use crate::components::project_list::Project;
use crate::components::{project_form::ProjectFormData, Footer, Header, ProjectForm, ProjectList};
use chrono::NaiveDate;
use leptos::*;
use leptos_router::*;
use uuid::Uuid;

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
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let result = if let Some(project) = editing {
                update_project(project.id, form_data).await
            } else {
                create_project(form_data).await
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
                                description: p.description.unwrap_or_default(),
                                start_date: p.start_date.to_string(),
                                end_date: p.end_date.to_string(),
                                status: p.status,
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
                                            />
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

/// Create a new project
async fn create_project(form_data: ProjectFormData) -> Result<(), String> {
    let start_date = NaiveDate::parse_from_str(&form_data.start_date, "%Y-%m-%d")
        .map_err(|_| "Invalid start date".to_string())?;
    let end_date = NaiveDate::parse_from_str(&form_data.end_date, "%Y-%m-%d")
        .map_err(|_| "Invalid end date".to_string())?;

    let response = authenticated_post_json(
        "http://localhost:3000/api/v1/projects",
        &serde_json::json!({
            "name": form_data.name,
            "description": if form_data.description.is_empty() { None } else { Some(form_data.description) },
            "start_date": start_date,
            "end_date": end_date,
            "status": form_data.status,
            "project_manager_id": null
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
async fn update_project(id: Uuid, form_data: ProjectFormData) -> Result<(), String> {
    let start_date = NaiveDate::parse_from_str(&form_data.start_date, "%Y-%m-%d")
        .map_err(|_| "Invalid start date".to_string())?;
    let end_date = NaiveDate::parse_from_str(&form_data.end_date, "%Y-%m-%d")
        .map_err(|_| "Invalid end date".to_string())?;

    let response = authenticated_put_json(
        &format!("http://localhost:3000/api/v1/projects/{}", id),
        &serde_json::json!({
            "name": form_data.name,
            "description": if form_data.description.is_empty() { None } else { Some(form_data.description) },
            "start_date": start_date,
            "end_date": end_date,
            "status": form_data.status,
            "project_manager_id": null
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
