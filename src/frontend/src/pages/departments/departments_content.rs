use crate::auth::{authenticated_delete, authenticated_get, authenticated_post_json, authenticated_put_json};
use crate::components::{DepartmentEditData, DepartmentForm, DepartmentFormData, HeadCandidate};
use leptos::*;
use serde::Deserialize;
use uuid::Uuid;

/// Department data structure
#[derive(Debug, Clone, Deserialize)]
pub struct Department {
    pub id: Uuid,
    pub name: String,
    pub head_id: Option<Uuid>,
    pub head_name: Option<String>,
}

/// Departments content component (without header/footer)
#[component]
pub fn DepartmentsContent() -> impl IntoView {
    let (departments, set_departments) = create_signal(Vec::<Department>::new());
    let (head_candidates, set_head_candidates) = create_signal(Vec::<HeadCandidate>::new());
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);
    let (show_form, set_show_form) = create_signal(false);
    let (editing_department, set_editing_department) = create_signal(Option::<Department>::None);
    let (form_submitting, set_form_submitting) = create_signal(false);
    let (deleting_id, set_deleting_id) = create_signal(Option::<String>::None);

    // Load departments and head candidates on mount
    create_effect(move |_| {
        set_loading.set(true);
        spawn_local(async move {
            // Load departments
            match fetch_departments().await {
                Ok(data) => set_departments.set(data),
                Err(e) => set_error.set(Some(e)),
            }

            // Load head candidates
            match fetch_head_candidates().await {
                Ok(data) => set_head_candidates.set(data),
                Err(e) => set_error.set(Some(e)),
            }

            set_loading.set(false);
        });
    });

    // Handle form submission
    let handle_submit = move |form_data: DepartmentFormData| {
        let editing_id = editing_department.get().map(|d| d.id);
        spawn_local(async move {
            set_form_submitting.set(true);
            set_error.set(None);

            let result = if let Some(dept_id) = editing_id {
                update_department(dept_id.to_string(), form_data).await
            } else {
                create_department(form_data).await
            };

            match result {
                Ok(_) => match fetch_departments().await {
                    Ok(data) => {
                        set_departments.set(data);
                        set_show_form.set(false);
                        set_editing_department.set(None);
                    }
                    Err(e) => set_error.set(Some(e)),
                },
                Err(e) => set_error.set(Some(e)),
            }
            set_form_submitting.set(false);
        });
    };

    let handle_cancel = move |_| {
        set_show_form.set(false);
        set_editing_department.set(None);
    };

    // Convert head candidates for form
    let head_candidate_options = create_memo(move |_| head_candidates.get());

    let editing_form_data = Signal::derive(move || {
        editing_department.get().map(|d| DepartmentEditData {
            id: d.id.to_string(),
            name: d.name.clone(),
            head_id: d.head_id.map(|id| id.to_string()).unwrap_or_default(),
        })
    });

    view! {
        <div class="space-y-6">
            <div class="flex items-center justify-between">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                        "Department Management"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400 mt-1">
                        "Manage departments and assign department heads"
                    </p>
                </div>

                <div class="flex items-center space-x-3">
                    <button
                        class="btn-primary"
                        on:click=move |_| {
                            set_editing_department.set(None);
                            set_show_form.set(true);
                        }
                    >
                        "Add Department"
                    </button>
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
                    let is_edit = editing_department.get().is_some();
                    let title = if is_edit { "Edit Department" } else { "Create Department" };
                    view! {
                        <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 relative">
                            <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">
                                {title}
                            </h2>
                            {move || {
                                let editing_data = editing_form_data.get();
                                view! {
                                    <DepartmentForm
                                        head_candidates=head_candidate_options.get()
                                        editing_department=editing_data.clone()
                                        on_submit=Callback::new(handle_submit)
                                        on_cancel=Callback::new(handle_cancel)
                                        is_submitting=form_submitting.get()
                                    />
                                }
                            }}
                            {move || {
                                if form_submitting.get() {
                                    view! {
                                        <div class="absolute inset-0 flex items-center justify-center bg-white/70 dark:bg-gray-800/70 rounded-lg">
                                            <div class="text-center">
                                                <div class="spinner mx-auto mb-2"></div>
                                                <p class="text-sm text-gray-600 dark:text-gray-400">"Saving..."</p>
                                            </div>
                                        </div>
                                    }.into_view()
                                } else {
                                    view! { <div></div> }.into_view()
                                }
                            }}
                        </div>
                    }.into_view()
                } else {
                    view! { <div></div> }.into_view()
                }
            }}

            <div class="bg-white dark:bg-gray-800 shadow rounded-lg overflow-hidden">
                <div class="overflow-x-auto">
                    <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                        <thead class="bg-gray-50 dark:bg-gray-700">
                            <tr>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Department Name"</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Head"</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Actions"</th>
                            </tr>
                        </thead>
                        <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                            {move || {
                                if loading.get() {
                                    view! {
                                        <tr>
                                            <td colspan="3" class="px-6 py-4 text-center text-gray-500 dark:text-gray-400">
                                                <div class="flex items-center justify-center">
                                                    <div class="spinner mr-2"></div>
                                                    "Loading departments..."
                                                </div>
                                            </td>
                                        </tr>
                                    }.into_view()
                                } else if departments.get().is_empty() {
                                    view! {
                                        <tr>
                                            <td colspan="3" class="px-6 py-4 text-center text-gray-500 dark:text-gray-400">
                                                "No departments found."
                                            </td>
                                        </tr>
                                    }.into_view()
                                } else {
                                    departments.get().into_iter().map(|dept| {
                                        let dept_id = dept.id.to_string();
                                        let dept_for_edit = dept.clone();
                                        let head_display = dept.head_name.clone().unwrap_or_else(|| "Unassigned".to_string());

                                        view! {
                                            <tr class="hover:bg-gray-50 dark:hover:bg-gray-700">
                                                <td class="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900 dark:text-white">
                                                    {dept.name.clone()}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                    {head_display}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                    <div class="flex items-center space-x-2">
                                                        <button
                                                            class="text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
                                                            on:click={
                                                                let dept = dept_for_edit.clone();
                                                                move |_| {
                                                                    set_editing_department.set(Some(dept.clone()));
                                                                    set_show_form.set(true);
                                                                }
                                                            }
                                                        >
                                                            "Edit"
                                                        </button>
                                                        {move || {
                                                            let is_deleting = deleting_id.get() == Some(dept_id.clone());
                                                            view! {
                                                                <button
                                                                    class="text-red-600 hover:text-red-800 dark:text-red-400 dark:hover:text-red-300 disabled:opacity-50 disabled:cursor-not-allowed"
                                                                    disabled=is_deleting
                                                                    on:click={
                                                                        let id = dept_id.clone();
                                                                        move |_| {
                                                                            let id_clone = id.clone();
                                                                            set_deleting_id.set(Some(id_clone.clone()));
                                                                            spawn_local(async move {
                                                                                set_error.set(None);

                                                                                match delete_department(id_clone).await {
                                                                                    Ok(_) => {
                                                                                        match fetch_departments().await {
                                                                                            Ok(data) => set_departments.set(data),
                                                                                            Err(e) => set_error.set(Some(e)),
                                                                                        }
                                                                                    }
                                                                                    Err(e) => set_error.set(Some(e)),
                                                                                }
                                                                                set_deleting_id.set(None);
                                                                            });
                                                                        }
                                                                    }
                                                                >
                                                                    {if is_deleting { "Deleting..." } else { "Delete" }}
                                                                </button>
                                                            }
                                                        }}
                                                    </div>
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
        </div>
    }
}

/// Fetch all departments from API
async fn fetch_departments() -> Result<Vec<Department>, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/departments")
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

/// Fetch head candidates from API
async fn fetch_head_candidates() -> Result<Vec<HeadCandidate>, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/departments/head-candidates")
        .await
        .map_err(|e| format!("Failed to fetch head candidates: {}", e))?;

    if response.status().is_success() {
        response
            .json::<Vec<HeadCandidate>>()
            .await
            .map_err(|e| format!("Failed to parse head candidates: {}", e))
    } else {
        Err(format!(
            "Failed to fetch head candidates: {}",
            response.status()
        ))
    }
}

/// Create a new department
async fn create_department(form_data: DepartmentFormData) -> Result<(), String> {
    let head_id = if form_data.head_id.is_empty() {
        None
    } else {
        Some(
            form_data
                .head_id
                .parse::<Uuid>()
                .map_err(|_| "Invalid head ID")?,
        )
    };

    let response = authenticated_post_json(
        "http://localhost:3000/api/v1/departments",
        &serde_json::json!({
            "name": form_data.name,
            "head_id": head_id,
        }),
    )
        .await
        .map_err(|e| format!("Failed to create department: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to create department: {}", error_text))
    }
}

/// Update an existing department
async fn update_department(dept_id: String, form_data: DepartmentFormData) -> Result<(), String> {
    let id = dept_id
        .parse::<Uuid>()
        .map_err(|_| "Invalid department ID")?;

    let head_id = if form_data.head_id.is_empty() {
        None
    } else {
        Some(
            form_data
                .head_id
                .parse::<Uuid>()
                .map_err(|_| "Invalid head ID")?,
        )
    };

    let response = authenticated_put_json(
        &format!("http://localhost:3000/api/v1/departments/{}", id),
        &serde_json::json!({
            "name": form_data.name,
            "head_id": head_id,
        }),
    )
        .await
        .map_err(|e| format!("Failed to update department: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to update department: {}", error_text))
    }
}

/// Delete a department
async fn delete_department(dept_id: String) -> Result<(), String> {
    let id = dept_id
        .parse::<Uuid>()
        .map_err(|_| "Invalid department ID")?;

    let response = authenticated_delete(&format!("http://localhost:3000/api/v1/departments/{}", id))
        .await
        .map_err(|e| format!("Failed to delete department: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to delete department: {}", error_text))
    }
}
