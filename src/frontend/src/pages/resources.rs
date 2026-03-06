use crate::auth::{
    authenticated_delete, authenticated_get, authenticated_post_json, authenticated_put_json,
    use_auth,
};
use crate::components::resource_list::Resource;
use crate::components::{
    resource_form::ResourceFormData, ResourceForm, ResourceList,
};
use leptos::*;
use leptos_router::*;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
struct Department {
    id: Uuid,
    name: String,
}

/// Resources page component
#[component]
pub fn Resources() -> impl IntoView {
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

    // Resource data
    let (resources, set_resources) = create_signal(Vec::new());
    let (departments, set_departments) = create_signal(Vec::new());
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);
    let (show_form, set_show_form) = create_signal(false);
    let (editing_resource, set_editing_resource) = create_signal(Option::<Resource>::None);

    // Load resources on mount
    create_effect(move |_| {
        set_loading.set(true);
        spawn_local(async move {
            match fetch_resources().await {
                Ok(data) => set_resources.set(data),
                Err(e) => set_error.set(Some(e)),
            }

            match fetch_departments().await {
                Ok(data) => set_departments.set(data),
                Err(e) => set_error.set(Some(e)),
            }

            set_loading.set(false);
        });
    });

    // Handle create/edit resource
    let handle_submit = move |form_data: ResourceFormData| {
        let editing = editing_resource.get();
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let result = if let Some(resource) = editing {
                update_resource(resource.id, form_data).await
            } else {
                create_resource(form_data).await
            };

            match result {
                Ok(_) => {
                    // Reload resources
                    match fetch_resources().await {
                        Ok(data) => {
                            set_resources.set(data);
                            set_show_form.set(false);
                            set_editing_resource.set(None);
                        }
                        Err(e) => set_error.set(Some(e)),
                    }
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    // Handle delete resource
    let handle_delete = move |id: Uuid| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            match delete_resource(id).await {
                Ok(_) => {
                    // Reload resources
                    match fetch_resources().await {
                        Ok(data) => set_resources.set(data),
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
        if let Some(resource) = resources.get().iter().find(|r| r.id == id).cloned() {
            set_editing_resource.set(Some(resource));
            set_show_form.set(true);
        }
    };

    // Handle cancel
    let handle_cancel = move |_| {
        set_show_form.set(false);
        set_editing_resource.set(None);
    };

    view! {
        <div class="h-full">

            <div class="page-container fade-in">
                <div class="space-y-4">
                    <div class="page-header">
                        <h1 class="text-xl font-semibold text-huly-caption">
                            "Resources"
                        </h1>
                        <button
                            class="btn-primary btn-press"
                            on:click=move |_| {
                                set_editing_resource.set(None);
                                set_show_form.set(true);
                            }
                        >
                            "Add Resource"
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
                            let initial_data = Signal::derive(move || {
                                editing_resource.get().map(|r| ResourceFormData {
                                    name: r.name,
                                    resource_type: r.resource_type,
                                    capacity: r.capacity,
                                    department_id: r.department_id,
                                    employment_start_date: r.employment_start_date,
                                })
                            });

                            view! {
                                <div class="card">
                                    <h2 class="text-xl font-semibold text-huly-caption mb-4">
                                        {if editing_resource.get().is_some() { "Edit Resource" } else { "Add Resource" }}
                                    </h2>
                                    <ResourceForm
                                        initial_data=initial_data
                                        departments=Signal::derive(move || {
                                            departments
                                                .get()
                                                .into_iter()
                                                .map(|d| (d.id.to_string(), d.name))
                                                .collect::<Vec<(String, String)>>()
                                        })
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
                                            <div class="space-y-3">
                                                <div class="toolbar"><div class="skeleton-text w-32 h-3"></div></div>
                                                <div class="panel overflow-hidden">
                                                    <div class="skeleton-row"><div class="skeleton-text w-28"></div><div class="skeleton-text w-20"></div><div class="skeleton-text w-16"></div><div class="skeleton-text w-24"></div></div>
                                                    <div class="skeleton-row"><div class="skeleton-text w-24"></div><div class="skeleton-text w-16"></div><div class="skeleton-text w-20"></div><div class="skeleton-text w-20"></div></div>
                                                    <div class="skeleton-row"><div class="skeleton-text w-32"></div><div class="skeleton-text w-12"></div><div class="skeleton-text w-24"></div><div class="skeleton-text w-16"></div></div>
                                                    <div class="skeleton-row"><div class="skeleton-text w-20"></div><div class="skeleton-text w-24"></div><div class="skeleton-text w-16"></div><div class="skeleton-text w-28"></div></div>
                                                </div>
                                            </div>
                                        }.into_view()
                                    } else if resources.get().is_empty() {
                                        view! {
                                            <div class="empty-state py-12">
                                                <svg class="w-12 h-12 text-huly-ghost mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1">
                                                    <path stroke-linecap="round" stroke-linejoin="round" d="M15 19.128a9.38 9.38 0 002.625.372 9.337 9.337 0 004.121-.952 4.125 4.125 0 00-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 018.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0111.964-3.07M12 6.375a3.375 3.375 0 11-6.75 0 3.375 3.375 0 016.75 0zm8.25 2.25a2.625 2.625 0 11-5.25 0 2.625 2.625 0 015.25 0z" />
                                                </svg>
                                                <p class="text-huly-secondary text-sm">"No resources found."</p>
                                                <p class="text-huly-muted text-xs mt-1">"Click 'Add Resource' to create one."</p>
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <div class="space-y-3">
                                                <div class="toolbar">
                                                    <h2 class="text-sm font-medium text-huly-secondary">"Resource List"</h2>
                                                </div>
                                                <div class="panel overflow-hidden slide-up">
                                                    <ResourceList
                                                        resources=resources.into()
                                                        on_edit=Callback::new(handle_edit)
                                                        on_delete=Callback::new(handle_delete)
                                                    />
                                                </div>
                                            </div>
                                        }.into_view()
                                    }
                                }}
                            </div> }.into_view()
                        }
                    }}
                </div>
            </div>

        </div>
    }
}

/// Fetch all resources from API
async fn fetch_resources() -> Result<Vec<Resource>, String> {
    let response = authenticated_get("/api/v1/resources")
        .await
        .map_err(|e| format!("Failed to fetch resources: {}", e))?;

    if response.status().is_success() {
        response
            .json::<Vec<Resource>>()
            .await
            .map_err(|e| format!("Failed to parse resources: {}", e))
    } else {
        Err(format!("Failed to fetch resources: {}", response.status()))
    }
}

async fn fetch_departments() -> Result<Vec<Department>, String> {
    let response = authenticated_get("/api/v1/departments")
        .await
        .map_err(|e| format!("Failed to fetch departments: {}", e))?;

    if response.status().is_success() {
        response
            .json::<Vec<Department>>()
            .await
            .map_err(|e| format!("Failed to parse departments: {}", e))
    } else {
        Err(format!(
            "Failed to fetch departments: {}",
            response.status()
        ))
    }
}

/// Create a new resource
async fn create_resource(form_data: ResourceFormData) -> Result<(), String> {
    let response = authenticated_post_json(
        "/api/v1/resources",
        &serde_json::json!({
            "name": form_data.name,
            "resource_type": form_data.resource_type,
            "capacity": form_data.capacity,
            "department_id": form_data.department_id,
            "employment_start_date": form_data.employment_start_date,
            "skills": null
        }),
    )
    .await
    .map_err(|e| format!("Failed to create resource: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to create resource: {}", error_text))
    }
}

/// Update an existing resource
async fn update_resource(id: Uuid, form_data: ResourceFormData) -> Result<(), String> {
    let response = authenticated_put_json(
        &format!("/api/v1/resources/{}", id),
        &serde_json::json!({
            "name": form_data.name,
            "resource_type": form_data.resource_type,
            "capacity": form_data.capacity,
            "department_id": form_data.department_id,
            "employment_start_date": form_data.employment_start_date,
            "skills": null
        }),
    )
    .await
    .map_err(|e| format!("Failed to update resource: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to update resource: {}", error_text))
    }
}

/// Delete a resource
async fn delete_resource(id: Uuid) -> Result<(), String> {
    let response = authenticated_delete(&format!("/api/v1/resources/{}", id))
        .await
        .map_err(|e| format!("Failed to delete resource: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("Failed to delete resource: {}", response.status()))
    }
}
