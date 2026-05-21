use crate::auth::{
    authenticated_delete, authenticated_get, authenticated_post_json, authenticated_put_json,
};
use crate::components::{DepartmentEditData, DepartmentForm, DepartmentFormData, HeadCandidate};
use leptos::either::{Either, EitherOf3};
use leptos::prelude::*;
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
    let (departments, set_departments) = signal(Vec::<Department>::new());
    let (head_candidates, set_head_candidates) = signal(Vec::<HeadCandidate>::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (show_form, set_show_form) = signal(false);
    let (editing_department, set_editing_department) = signal(Option::<Department>::None);
    let (form_submitting, set_form_submitting) = signal(false);
    let (deleting_id, set_deleting_id) = signal(Option::<String>::None);

    // Load departments and head candidates on mount
    Effect::new(move |_| {
        set_loading.set(true);
        leptos::task::spawn_local(async move {
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
        leptos::task::spawn_local(async move {
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
    let head_candidate_options = Memo::new(move |_| head_candidates.get());

    let editing_form_data = Signal::derive(move || {
        editing_department.get().map(|d| DepartmentEditData {
            id: d.id.to_string(),
            name: d.name.clone(),
            head_id: d.head_id.map(|id| id.to_string()).unwrap_or_default(),
        })
    });

    view! {
        <div class="space-y-4">
            <div class="page-header">
                <div>
                    <h1 class="text-xl font-semibold text-huly-caption">
                        "Department Management"
                    </h1>
                    <p class="text-huly-secondary mt-1">
                        "Manage departments and assign department heads"
                    </p>
                </div>

                <div class="flex items-center gap-2">
                    <button
                        class="btn-primary btn-press"
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
                    let is_edit = editing_department.get().is_some();
                    let title = if is_edit { "Edit Department" } else { "Create Department" };
                    Either::Left(view! {
                        <div class="card relative">
                            <h2 class="text-xl font-semibold text-huly-caption mb-4">
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
                                    Either::Left(view! {
                                        <div class="absolute inset-0 flex items-center justify-center bg-huly-back/70 rounded-lg">
                                            <div class="text-center">
                                                <div class="spinner mx-auto mb-2"></div>
                                                <p class="text-sm text-huly-secondary">"Saving..."</p>
                                            </div>
                                        </div>
                                    })
                                } else {
                                    Either::Right(view! { <div></div> })
                                }
                            }}
                        </div>
                    })
                } else {
                    Either::Right(view! { <div></div> })
                }
            }}

            <div class="panel">
                <div class="toolbar">
                    <h2 class="text-sm font-semibold text-huly-caption">"Department Management"</h2>
                </div>
                <div class="overflow-x-auto">
                    <table class="min-w-full divide-y divide-huly-divider">
                        <thead class="bg-huly-surface-2">
                            <tr>
                                <th class="th-cell-compact">"Department Name"</th>
                                <th class="th-cell-compact">"Head"</th>
                                <th class="th-cell-compact">"Actions"</th>
                            </tr>
                        </thead>
                        <tbody class="bg-huly-surface divide-y divide-huly-divider">
                            {move || {
                                if loading.get() {
                                    EitherOf3::A(view! {
                                        <tr>
                                            <td colspan="3" class="td-cell-compact">
                                                <div class="space-y-2 py-2">
                                                    <div class="skeleton-row"><div class="skeleton-text w-28"></div><div class="skeleton-text w-20"></div><div class="skeleton-text w-16"></div></div>
                                                    <div class="skeleton-row"><div class="skeleton-text w-24"></div><div class="skeleton-text w-16"></div><div class="skeleton-text w-20"></div></div>
                                                    <div class="skeleton-row"><div class="skeleton-text w-32"></div><div class="skeleton-text w-24"></div><div class="skeleton-text w-12"></div></div>
                                                </div>
                                            </td>
                                        </tr>
                                    })
                                } else if departments.get().is_empty() {
                                    EitherOf3::B(view! {
                                        <tr>
                                            <td colspan="3" class="td-cell-compact">
                                                <div class="empty-state py-8">
                                                    <svg class="w-10 h-10 text-huly-ghost mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1">
                                                        <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 21h16.5M4.5 3h15M5.25 3v18m13.5-18v18M9 6.75h1.5m-1.5 3h1.5m-1.5 3h1.5m3-6H15m-1.5 3H15m-1.5 3H15M9 21v-3.375c0-.621.504-1.125 1.125-1.125h3.75c.621 0 1.125.504 1.125 1.125V21" />
                                                    </svg>
                                                    <p class="text-huly-secondary text-sm">"No departments found."</p>
                                                    <p class="text-huly-muted text-xs mt-1">"Click 'Add Department' to create one."</p>
                                                </div>
                                            </td>
                                        </tr>
                                    })
                                } else {
                                    EitherOf3::C(departments.get().into_iter().map(|dept| {
                                        let dept_id = dept.id.to_string();
                                        let dept_for_edit = dept.clone();
                                        let head_display = dept.head_name.clone().unwrap_or_else(|| "Unassigned".to_string());

                                        view! {
                                            <tr class="table-row-hover">
                                                <td class="td-cell-compact whitespace-nowrap font-medium text-huly-caption">
                                                    {dept.name.clone()}
                                                </td>
                                                <td class="td-cell-compact whitespace-nowrap text-huly-muted">
                                                    {head_display}
                                                </td>
                                                <td class="td-cell-compact whitespace-nowrap text-huly-muted">
                                                    <div class="flex items-center gap-2">
                                                        <button
                                                            class="link"
                                                            on:click={
                                                                let d = dept_for_edit.clone();
                                                                move |_| {
                                                                    set_editing_department.set(Some(d.clone()));
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
                                                                    class="link-danger disabled:opacity-50 disabled:cursor-not-allowed"
                                                                    disabled=is_deleting
                                                                    on:click={
                                                                        let id = dept_id.clone();
                                                                        move |_| {
                                                                            let id_clone = id.clone();
                                                                            set_deleting_id.set(Some(id_clone.clone()));
                                                                            leptos::task::spawn_local(async move {
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
                                    }).collect_view())
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

/// Fetch head candidates from API
async fn fetch_head_candidates() -> Result<Vec<HeadCandidate>, String> {
    let response = authenticated_get("/api/v1/departments/head-candidates")
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
        "/api/v1/departments",
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
        &format!("/api/v1/departments/{}", id),
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

    let response = authenticated_delete(&format!("/api/v1/departments/{}", id))
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
