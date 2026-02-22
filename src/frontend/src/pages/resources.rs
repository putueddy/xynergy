use crate::auth::{authenticated_delete, authenticated_get, authenticated_post_json, authenticated_put_json, use_auth};
use crate::components::resource_list::Resource;
use crate::components::{
    resource_form::ResourceFormData, Footer, Header, ResourceForm, ResourceList,
};
use leptos::*;
use leptos_router::*;
use uuid::Uuid;

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
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);
    let (show_form, set_show_form) = create_signal(false);
    let (editing_resource, set_editing_resource) = create_signal(Option::<Resource>::None);

    // Load resources on mount
    create_effect(move |_| {
        set_loading.set(true);
        spawn_local(async move {
            match fetch_resources().await {
                Ok(data) => {
                    set_resources.set(data);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
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
        <div class="min-h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
            <Header/>

            <main class="flex-grow max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12 w-full">
                <div class="space-y-6">
                    <div class="flex items-center justify-between">
                        <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                            "Resources"
                        </h1>
                        <button
                            class="btn-primary"
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
                            let initial_data = Signal::derive(move || {
                                editing_resource.get().map(|r| ResourceFormData {
                                    name: r.name,
                                    resource_type: r.resource_type,
                                    capacity: r.capacity,
                                    department_id: r.department_id,
                                })
                            });

                            view! {
                                <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6">
                                    <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">
                                        {if editing_resource.get().is_some() { "Edit Resource" } else { "Add Resource" }}
                                    </h2>
                                    <ResourceForm
                                        initial_data=initial_data
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
                                                <p class="text-gray-600 dark:text-gray-400">"Loading resources..."</p>
                                            </div>
                                        }.into_view()
                                    } else if resources.get().is_empty() {
                                        view! {
                                            <div class="text-center py-12 bg-white dark:bg-gray-800 rounded-lg shadow">
                                                <p class="text-gray-600 dark:text-gray-400">"No resources found."</p>
                                                <p class="text-sm text-gray-500 dark:text-gray-500 mt-2">"Click 'Add Resource' to create one."</p>
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <ResourceList
                                                resources=resources.into()
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

/// Fetch all resources from API
async fn fetch_resources() -> Result<Vec<Resource>, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/resources")
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

/// Create a new resource
async fn create_resource(form_data: ResourceFormData) -> Result<(), String> {
    let response = authenticated_post_json(
        "http://localhost:3000/api/v1/resources",
        &serde_json::json!({
            "name": form_data.name,
            "resource_type": form_data.resource_type,
            "capacity": form_data.capacity,
            "department_id": form_data.department_id,
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
        &format!("http://localhost:3000/api/v1/resources/{}", id),
        &serde_json::json!({
            "name": form_data.name,
            "resource_type": form_data.resource_type,
            "capacity": form_data.capacity,
            "department_id": form_data.department_id,
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
    let response = authenticated_delete(&format!("http://localhost:3000/api/v1/resources/{}", id))
        .await
        .map_err(|e| format!("Failed to delete resource: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("Failed to delete resource: {}", response.status()))
    }
}
