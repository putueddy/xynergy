use leptos::*;
use leptos_router::*;
use crate::auth::use_auth;
use crate::components::{Header, Footer, GanttChart, AllocationForm, AllocationFormData, ResourceOption, ProjectOption};
use crate::gantt::GanttTask;
use uuid::Uuid;
use serde::Deserialize;

/// Allocation data structure
#[derive(Debug, Clone, Deserialize)]
pub struct Allocation {
    pub id: Uuid,
    pub project_id: Uuid,
    pub resource_id: Uuid,
    pub start_date: String,
    pub end_date: String,
    pub allocation_percentage: f64,
    pub project_name: String,
    pub resource_name: String,
}

/// Resource data for dropdown
#[derive(Debug, Clone, Deserialize)]
pub struct Resource {
    pub id: Uuid,
    pub name: String,
}

/// Project data for dropdown
#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
}

/// Allocations page component
#[component]
pub fn Allocations() -> impl IntoView {
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
    
    // Data signals
    let (allocations, set_allocations) = create_signal(Vec::new());
    let (resources, set_resources) = create_signal(Vec::new());
    let (projects, set_projects) = create_signal(Vec::new());
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);
    let (view_mode, set_view_mode) = create_signal("Week");
    let (show_form, set_show_form) = create_signal(false);
    
    // Load data on mount
    create_effect(move |_| {
        set_loading.set(true);
        spawn_local(async move {
            // Load allocations
            match fetch_allocations().await {
                Ok(data) => set_allocations.set(data),
                Err(e) => set_error.set(Some(e)),
            }
            
            // Load resources
            match fetch_resources().await {
                Ok(data) => set_resources.set(data),
                Err(e) => set_error.set(Some(e)),
            }
            
            // Load projects
            match fetch_projects().await {
                Ok(data) => set_projects.set(data),
                Err(e) => set_error.set(Some(e)),
            }
            
            set_loading.set(false);
        });
    });
    
    // Convert allocations to Gantt tasks
    let (gantt_tasks, _set_gantt_tasks) = create_signal(Vec::<GanttTask>::new());
    
    create_effect(move |_| {
        let tasks: Vec<GanttTask> = allocations.get()
            .into_iter()
            .map(|a| GanttTask {
                id: a.id.to_string(),
                name: format!("{} - {}", a.resource_name, a.project_name),
                start: a.start_date,
                end: a.end_date,
                progress: a.allocation_percentage,
                custom_class: Some(format!("allocation-{}", a.resource_id)),
            })
            .collect();
        _set_gantt_tasks.set(tasks);
    });
    
    // Handle form submission
    let handle_submit = move |form_data: AllocationFormData| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            
            match create_allocation(form_data).await {
                Ok(_) => {
                    // Reload allocations
                    match fetch_allocations().await {
                        Ok(data) => {
                            set_allocations.set(data);
                            set_show_form.set(false);
                        }
                        Err(e) => set_error.set(Some(e)),
                    }
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };
    
    let handle_cancel = move |_| {
        set_show_form.set(false);
    };
    
    // Convert to option types for form
    let resource_options = create_memo(move |_| {
        resources.get().into_iter().map(|r| ResourceOption {
            id: r.id,
            name: r.name,
        }).collect::<Vec<_>>()
    });
    
    let project_options = create_memo(move |_| {
        projects.get().into_iter().map(|p| ProjectOption {
            id: p.id,
            name: p.name,
        }).collect::<Vec<_>>()
    });
    
    view! {
        <div class="min-h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
            <Header/>
            
            <main class="flex-grow max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12 w-full">
                <div class="space-y-6">
                    <div class="flex items-center justify-between">
                        <div>
                            <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                                "Resource Allocations"
                            </h1>
                            <p class="text-gray-600 dark:text-gray-400 mt-1">
                                "Visual timeline of resource assignments to projects"
                            </p>
                        </div>
                        
                        <div class="flex items-center space-x-3">
                            <button
                                class="btn-primary"
                                on:click=move |_| set_show_form.set(true)
                            >
                                "Add Allocation"
                            </button>
                            
                            <label class="text-sm font-medium text-gray-700 dark:text-gray-300">"View:"</label>
                            <select
                                class="rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                prop:value=view_mode
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    set_view_mode.set(Box::leak(value.into_boxed_str()));
                                }
                            >
                                <option value="Quarter Day">"Quarter Day"</option>
                                <option value="Half Day">"Half Day"</option>
                                <option value="Day">"Day"</option>
                                <option value="Week">"Week"</option>
                                <option value="Month">"Month"</option>
                            </select>
                        </div>
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
                            view! {
                                <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6">
                                    <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">
                                        "Create Allocation"
                                    </h2>
                                    <AllocationForm
                                        resources=resource_options.into()
                                        projects=project_options.into()
                                        on_submit=Callback::new(handle_submit)
                                        on_cancel=Callback::new(handle_cancel)
                                    />
                                </div>
                            }.into_view()
                        } else {
                            view! { <div></div> }.into_view()
                        }
                    }}
                    
                    {move || {
                        if loading.get() {
                            view! {
                                <div class="text-center py-12 bg-white dark:bg-gray-800 rounded-lg shadow">
                                    <div class="spinner mx-auto mb-4"></div>
                                    <p class="text-gray-600 dark:text-gray-400">"Loading allocations..."</p>
                                </div>
                            }.into_view()
                        } else if allocations.get().is_empty() {
                            view! {
                                <div class="text-center py-12 bg-white dark:bg-gray-800 rounded-lg shadow">
                                    <p class="text-gray-600 dark:text-gray-400">"No allocations found."</p>
                                    <p class="text-sm text-gray-500 dark:text-gray-500 mt-2">"Click 'Add Allocation' to create one."</p>
                                </div>
                            }.into_view()
                        } else {
                            view! {
                                <div class="space-y-6">
                                    <GanttChart
                                        tasks=gantt_tasks.into()
                                        view_mode=view_mode.get()
                                    />
                                    
                                    <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6">
                                        <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                                            "Allocation Details"
                                        </h2>
                                        <div class="overflow-x-auto">
                                            <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                                <thead class="bg-gray-50 dark:bg-gray-700">
                                                    <tr>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Resource"</th>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Project"</th>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Start Date"</th>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"End Date"</th>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Allocation"</th>
                                                    </tr>
                                                </thead>
                                                <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                                                    {move || allocations.get().into_iter().map(|allocation| {
                                                        view! {
                                                            <tr class="hover:bg-gray-50 dark:hover:bg-gray-700">
                                                                <td class="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900 dark:text-white">
                                                                    {allocation.resource_name}
                                                                </td>
                                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                                    {allocation.project_name}
                                                                </td>
                                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                                    {allocation.start_date}
                                                                </td>
                                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                                    {allocation.end_date}</td>
                                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                                    {format!("{:.0}%", allocation.allocation_percentage)}
                                                                </td>
                                                            </tr>
                                                        }
                                                    }).collect_view()}
                                                </tbody>
                                            </table>
                                        </div>
                                    </div>
                                </div>
                            }.into_view()
                        }
                    }}
                </div>
            </main>
            
            <Footer/>
        </div>
    }
}

/// Fetch all allocations from API
async fn fetch_allocations() -> Result<Vec<Allocation>, String> {
    let response = reqwest::get("http://localhost:3000/api/v1/allocations")
        .await
        .map_err(|e| format!("Failed to fetch allocations: {}", e))?;
    
    if response.status().is_success() {
        response.json::<Vec<Allocation>>()
            .await
            .map_err(|e| format!("Failed to parse allocations: {}", e))
    } else {
        Err(format!("Failed to fetch allocations: {}", response.status()))
    }
}

/// Fetch all resources from API
async fn fetch_resources() -> Result<Vec<Resource>, String> {
    let response = reqwest::get("http://localhost:3000/api/v1/resources")
        .await
        .map_err(|e| format!("Failed to fetch resources: {}", e))?;
    
    if response.status().is_success() {
        // Parse as generic JSON and extract id and name
        let json_data: Vec<serde_json::Value> = response.json()
            .await
            .map_err(|e| format!("Failed to parse resources: {}", e))?;
        
        let resources: Vec<Resource> = json_data
            .into_iter()
            .filter_map(|v| {
                Some(Resource {
                    id: v.get("id")?.as_str()?.parse().ok()?,
                    name: v.get("name")?.as_str()?.to_string(),
                })
            })
            .collect();
        
        Ok(resources)
    } else {
        Err(format!("Failed to fetch resources: {}", response.status()))
    }
}

/// Fetch all projects from API
async fn fetch_projects() -> Result<Vec<Project>, String> {
    let response = reqwest::get("http://localhost:3000/api/v1/projects")
        .await
        .map_err(|e| format!("Failed to fetch projects: {}", e))?;
    
    if response.status().is_success() {
        // Parse as generic JSON and extract id and name
        let json_data: Vec<serde_json::Value> = response.json()
            .await
            .map_err(|e| format!("Failed to parse projects: {}", e))?;
        
        let projects: Vec<Project> = json_data
            .into_iter()
            .filter_map(|v| {
                Some(Project {
                    id: v.get("id")?.as_str()?.parse().ok()?,
                    name: v.get("name")?.as_str()?.to_string(),
                })
            })
            .collect();
        
        Ok(projects)
    } else {
        Err(format!("Failed to fetch projects: {}", response.status()))
    }
}

/// Create a new allocation
async fn create_allocation(form_data: AllocationFormData) -> Result<(), String> {
    let resource_id = form_data.resource_id.parse::<Uuid>()
        .map_err(|_| "Invalid resource ID")?;
    let project_id = form_data.project_id.parse::<Uuid>()
        .map_err(|_| "Invalid project ID")?;
    let allocation_percentage = form_data.allocation_percentage.parse::<f64>()
        .map_err(|_| "Invalid allocation percentage")?;
    
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:3000/api/v1/allocations")
        .json(&serde_json::json!({
            "resource_id": resource_id,
            "project_id": project_id,
            "start_date": form_data.start_date,
            "end_date": form_data.end_date,
            "allocation_percentage": allocation_percentage,
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to create allocation: {}", e))?;
    
    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to create allocation: {}", error_text))
    }
}
