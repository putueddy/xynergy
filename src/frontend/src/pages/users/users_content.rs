use crate::auth::{
    authenticated_delete, authenticated_get, authenticated_post_json, authenticated_put_json,
    use_auth,
};
use crate::components::{DepartmentOption, UserEditData, UserForm, UserFormData};
use leptos::*;
use serde::Deserialize;
use uuid::Uuid;

/// User data structure
#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    pub department_id: Option<Uuid>,
}

/// Department data structure
#[derive(Debug, Clone, Deserialize)]
pub struct Department {
    pub id: Uuid,
    pub name: String,
}

/// Users content component (without header/footer)
#[component]
pub fn UsersContent() -> impl IntoView {
    let auth = use_auth();
    let is_admin = move || auth.user.get().map(|u| u.role == "admin").unwrap_or(false);

    // Data signals
    let (users, set_users) = create_signal(Vec::new());
    let (departments, set_departments) = create_signal(Vec::new());
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);

    let (show_form, set_show_form) = create_signal(false);
    let (editing_user, set_editing_user) = create_signal(Option::<User>::None);
    let (form_submitting, set_form_submitting) = create_signal(false);
    let (deleting_id, set_deleting_id) = create_signal(Option::<String>::None);
    let (resetting_id, set_resetting_id) = create_signal(Option::<String>::None);
    let (new_password, set_new_password) = create_signal(String::new());

    // Load data on mount
    create_effect(move |_| {
        set_loading.set(true);
        spawn_local(async move {
            // Load users
            match fetch_users().await {
                Ok(data) => set_users.set(data),
                Err(e) => set_error.set(Some(e)),
            }

            // Load departments
            match fetch_departments().await {
                Ok(data) => set_departments.set(data),
                Err(e) => set_error.set(Some(e)),
            }

            set_loading.set(false);
        });
    });

    // Handle form submission
    let handle_submit = move |form_data: UserFormData| {
        let editing_id = editing_user.get().map(|u| u.id);
        spawn_local(async move {
            set_form_submitting.set(true);
            set_error.set(None);

            let result = if let Some(user_id) = editing_id {
                update_user_form(user_id.to_string(), form_data).await
            } else {
                create_user(form_data).await
            };

            match result {
                Ok(_) => {
                    // Reload users
                    match fetch_users().await {
                        Ok(data) => {
                            set_users.set(data);
                            set_show_form.set(false);
                            set_editing_user.set(None);
                        }
                        Err(e) => set_error.set(Some(e)),
                    }
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_form_submitting.set(false);
        });
    };

    let handle_cancel = move |_| {
        set_show_form.set(false);
        set_editing_user.set(None);
    };

    // Convert departments to options for form
    let department_options = create_memo(move |_| {
        departments
            .get()
            .into_iter()
            .map(|d| DepartmentOption {
                id: d.id,
                name: d.name,
            })
            .collect::<Vec<_>>()
    });

    let editing_form_data = Signal::derive(move || {
        editing_user.get().map(|u| UserEditData {
            id: u.id.to_string(),
            email: u.email.clone(),
            first_name: u.first_name.clone(),
            last_name: u.last_name.clone(),
            role: u.role.clone(),
            department_id: u.department_id.map(|id| id.to_string()).unwrap_or_default(),
        })
    });

    // Helper function to get department name
    let get_department_name = move |dept_id: Option<Uuid>| -> String {
        if let Some(id) = dept_id {
            departments
                .get()
                .iter()
                .find(|d| d.id == id)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            "Unassigned".to_string()
        }
    };

    // Helper function to get role badge class
    let get_role_badge_class = |role: &str| -> &str {
        match role {
            "admin" => "badge-negative",
            "project_manager" => "badge-primary",
            _ => "badge-positive",
        }
    };

    view! {
        <div class="space-y-4">
            <div class="page-header">
                <div>
                    <h1 class="text-xl font-semibold text-huly-caption">
                        "User Management"
                    </h1>
                    <p class="text-huly-secondary mt-1">
                        "Manage system users and their roles"
                    </p>
                </div>

                <div class="flex items-center gap-2">
                    {move || {
                        if is_admin() {
                            view! {
                                <button
                                    class="btn-primary btn-press"
                                    on:click=move |_| set_show_form.set(true)
                                >
                                    "Add User"
                                </button>
                            }.into_view()
                        } else {
                            view! { <div></div> }.into_view()
                        }
                    }}
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
                    let is_edit = editing_user.get().is_some();
                    let title = if is_edit { "Edit User" } else { "Create User" };
                    view! {
                        <div class="card relative">
                            <h2 class="text-xl font-semibold text-huly-caption mb-4">
                                {title}
                            </h2>
                            {move || {
                                let editing_data = editing_form_data.get();
                                view! {
                                    <UserForm
                                        departments=department_options.get()
                                        editing_user=editing_data
                                        on_submit=Callback::new(handle_submit)
                                        on_cancel=Callback::new(handle_cancel)
                                        is_submitting=form_submitting.get()
                                    />
                                }
                            }}
                            {move || {
                                if form_submitting.get() {
                                    view! {
                                        <div class="absolute inset-0 flex items-center justify-center bg-huly-back/70 rounded-lg">
                                            <div class="text-center">
                                                <div class="spinner mx-auto mb-2"></div>
                                                <p class="text-sm text-huly-secondary">"Saving..."</p>
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

            <div class="panel">
                <div class="toolbar">
                    <h2 class="text-sm font-semibold text-huly-caption">"User Management"</h2>
                </div>
                <div class="overflow-x-auto">
                    <table class="min-w-full divide-y divide-huly-divider">
                        <thead class="bg-huly-surface-2">
                            <tr>
                                <th class="th-cell-compact">"Name"</th>
                                <th class="th-cell-compact">"Email"</th>
                                <th class="th-cell-compact">"Role"</th>
                                <th class="th-cell-compact">"Department"</th>
                                <th class="th-cell-compact">"Actions"</th>
                            </tr>
                        </thead>
                        <tbody class="bg-huly-surface divide-y divide-huly-divider">
                            {move || {
                                if loading.get() {
                                    view! {
                                        <tr>
                                            <td colspan="5" class="td-cell-compact">
                                                <div class="space-y-2 py-2">
                                                    <div class="skeleton-row"><div class="skeleton-text w-28"></div><div class="skeleton-text w-32"></div><div class="skeleton-text w-16"></div><div class="skeleton-text w-20"></div><div class="skeleton-text w-12"></div></div>
                                                    <div class="skeleton-row"><div class="skeleton-text w-24"></div><div class="skeleton-text w-28"></div><div class="skeleton-text w-20"></div><div class="skeleton-text w-16"></div><div class="skeleton-text w-16"></div></div>
                                                    <div class="skeleton-row"><div class="skeleton-text w-32"></div><div class="skeleton-text w-24"></div><div class="skeleton-text w-12"></div><div class="skeleton-text w-24"></div><div class="skeleton-text w-12"></div></div>
                                                </div>
                                            </td>
                                        </tr>
                                    }.into_view()
                                } else if users.get().is_empty() {
                                    view! {
                                        <tr>
                                            <td colspan="5" class="td-cell-compact">
                                                <div class="empty-state py-8">
                                                    <svg class="w-10 h-10 text-huly-ghost mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1">
                                                        <path stroke-linecap="round" stroke-linejoin="round" d="M19 7.5v3m0 0v3m0-3h3m-3 0h-3m-2.25-4.125a3.375 3.375 0 11-6.75 0 3.375 3.375 0 016.75 0zM4 19.235v-.11a6.375 6.375 0 0112.75 0v.109A12.318 12.318 0 0110.374 21c-2.331 0-4.512-.645-6.374-1.766z" />
                                                    </svg>
                                                    <p class="text-huly-secondary text-sm">"No users found."</p>
                                                    <p class="text-huly-muted text-xs mt-1">"Click 'Add User' to create one."</p>
                                                </div>
                                            </td>
                                        </tr>
                                    }.into_view()
                                } else {
                                    users.get().into_iter().map(|user| {
                                        let user_id = user.id.to_string();
                                        let user_id_for_reset = user.id.to_string();
                                        let user_for_edit = user.clone();
                                        let dept_name = get_department_name(user.department_id);
                                        let role_class = get_role_badge_class(&user.role);
                                        let role_display = user.role.replace("_", " ");

                                        view! {
                                            <tr class="table-row-hover">
                                                <td class="td-cell-compact whitespace-nowrap">
                                                    <div class="flex items-center">
                                                        <div class="flex-shrink-0 h-10 w-10 rounded-full bg-blue-500 flex items-center justify-center text-white font-semibold">
                                                            {format!("{}{}",
                                                                user.first_name.chars().next().unwrap_or('U'),
                                                                user.last_name.chars().next().unwrap_or('N')
                                                            )}
                                                        </div>
                                                        <div class="ml-4">
                                                            <div class="text-sm font-medium text-huly-caption">
                                                                {format!("{} {}", user.first_name, user.last_name)}
                                                            </div>
                                                        </div>
                                                    </div>
                                                </td>
                                                <td class="td-cell-compact whitespace-nowrap text-huly-muted">
                                                    {user.email.clone()}
                                                </td>
                                                <td class="td-cell-compact whitespace-nowrap">
                                                    <span class={format!("px-2 inline-flex text-xs leading-5 font-semibold rounded-full {}", role_class)}>
                                                        {role_display}
                                                    </span>
                                                </td>
                                                <td class="td-cell-compact whitespace-nowrap text-huly-muted">
                                                    {dept_name}
                                                </td>
                                                <td class="td-cell-compact whitespace-nowrap text-huly-muted">
                                                    <div class="flex flex-col space-y-2">
                                                        {if is_admin() {
                                                            view! {
                                                                <>
                                                                    <div class="flex items-center space-x-2">
                                                                        <button
                                                                            class="link"
                                                                            on:click={
                                                                                let user = user_for_edit.clone();
                                                                                move |_| {
                                                                                    set_editing_user.set(Some(user.clone()));
                                                                                    set_show_form.set(true);
                                                                                }
                                                                            }
                                                                        >
                                                                            "Edit"
                                                                        </button>
                                                                        <button
                                                                            class="text-warning-default hover:text-warning-default/80"
                                                                            on:click={
                                                                                let uid = user_id.clone();
                                                                                move |_| {
                                                                                    set_new_password.set(String::new());
                                                                                    set_resetting_id.set(Some(uid.clone()));
                                                                                }
                                                                            }
                                                                        >
                                                                            "Reset Pwd"
                                                                        </button>
                                                                        {move || {
                                                                            let is_deleting = deleting_id.get() == Some(user_id.clone());
                                                                            view! {
                                                                                <button
                                                                                    class="link-danger disabled:opacity-50 disabled:cursor-not-allowed"
                                                                                    disabled=is_deleting
                                                                                    on:click={
                                                                                        let id = user_id.clone();
                                                                                        move |_| {
                                                                                            let id_clone = id.clone();
                                                                                            set_deleting_id.set(Some(id_clone.clone()));
                                                                                            spawn_local(async move {
                                                                                                set_error.set(None);
                                                                                                match delete_user(id_clone).await {
                                                                                                    Ok(_) => {
                                                                                                        match fetch_users().await {
                                                                                                            Ok(data) => set_users.set(data),
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
                                                                    // Inline password reset form
                                                                    {move || {
                                                                        let uid = user_id_for_reset.clone();
                                                                        if resetting_id.get() == Some(uid.clone()) {
                                                                            view! {
                                                                                <div class="flex items-center space-x-2 mt-1">
                                                                                    <input
                                                                                        type="password"
                                                                                        class="w-32 px-2 py-1 text-xs rounded border input"
                                                                                        placeholder="New password"
                                                                                        prop:value=new_password
                                                                                        on:input=move |ev| set_new_password.set(event_target_value(&ev))
                                                                                    />
                                                                                    <button
                                                                                        class="text-xs px-2 py-1 bg-amber-500 text-white rounded hover:bg-amber-600 disabled:opacity-50"
                                                                                        disabled=move || new_password.get().len() < 6
                                                                                        on:click={
                                                                                            let uid = uid.clone();
                                                                                            move |_| {
                                                                                                let uid = uid.clone();
                                                                                                let pw = new_password.get();
                                                                                                spawn_local(async move {
                                                                                                    set_error.set(None);
                                                                                                    match reset_user_password(uid, pw).await {
                                                                                                        Ok(_) => set_resetting_id.set(None),
                                                                                                        Err(e) => set_error.set(Some(e)),
                                                                                                    }
                                                                                                });
                                                                                            }
                                                                                        }
                                                                                    >
                                                                                        "Save"
                                                                                    </button>
                                                                                    <button
                                                                                        class="text-xs px-2 py-1 text-huly-muted hover:text-huly-content"
                                                                                        on:click=move |_| set_resetting_id.set(None)
                                                                                    >
                                                                                        "Cancel"
                                                                                    </button>
                                                                                </div>
                                                                            }.into_view()
                                                                        } else {
                                                                            view! { <span></span> }.into_view()
                                                                        }
                                                                    }}
                                                                </>
                                                            }.into_view()
                                                        } else {
                                                            view! {
                                                                <span class="text-huly-ghost italic">"No access"</span>
                                                            }.into_view()
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

/// Fetch all users from API
async fn fetch_users() -> Result<Vec<User>, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/users")
        .await
        .map_err(|e| format!("Failed to fetch users: {}", e))?;

    if response.status().is_success() {
        response
            .json::<Vec<User>>()
            .await
            .map_err(|e| format!("Failed to parse users: {}", e))
    } else if response.status() == reqwest::StatusCode::FORBIDDEN {
        Err("Insufficient permissions".to_string())
    } else {
        Err(format!("Failed to fetch users: {}", response.status()))
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

/// Create a new user
async fn create_user(form_data: UserFormData) -> Result<(), String> {
    let department_id = if form_data.department_id.is_empty() {
        None
    } else {
        Some(
            form_data
                .department_id
                .parse::<Uuid>()
                .map_err(|_| "Invalid department ID")?,
        )
    };

    let response = authenticated_post_json(
        "http://localhost:3000/api/v1/users",
        &serde_json::json!({
            "email": form_data.email,
            "password": form_data.password,
            "first_name": form_data.first_name,
            "last_name": form_data.last_name,
            "role": form_data.role,
            "department_id": department_id,
        }),
    )
    .await
    .map_err(|e| format!("Failed to create user: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        if status == reqwest::StatusCode::FORBIDDEN {
            Err("Insufficient permissions".to_string())
        } else {
            Err(format!("Failed to create user: {}", error_text))
        }
    }
}

/// Update an existing user
async fn update_user_form(user_id: String, form_data: UserFormData) -> Result<(), String> {
    let id = user_id.parse::<Uuid>().map_err(|_| "Invalid user ID")?;

    let department_id = if form_data.department_id.is_empty() {
        None
    } else {
        Some(
            form_data
                .department_id
                .parse::<Uuid>()
                .map_err(|_| "Invalid department ID")?,
        )
    };

    let response = authenticated_put_json(
        &format!("http://localhost:3000/api/v1/users/{}", id),
        &serde_json::json!({
            "email": form_data.email,
            "first_name": form_data.first_name,
            "last_name": form_data.last_name,
            "role": form_data.role,
            "department_id": department_id,
        }),
    )
    .await
    .map_err(|e| format!("Failed to update user: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        if status == reqwest::StatusCode::FORBIDDEN {
            Err("Insufficient permissions".to_string())
        } else {
            Err(format!("Failed to update user: {}", error_text))
        }
    }
}

/// Delete a user
async fn delete_user(user_id: String) -> Result<(), String> {
    let id = user_id.parse::<Uuid>().map_err(|_| "Invalid user ID")?;

    let response = authenticated_delete(&format!("http://localhost:3000/api/v1/users/{}", id))
        .await
        .map_err(|e| format!("Failed to delete user: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        if status == reqwest::StatusCode::FORBIDDEN {
            Err("Insufficient permissions".to_string())
        } else {
            Err(format!("Failed to delete user: {}", error_text))
        }
    }
}

/// Reset a user's password (admin only)
async fn reset_user_password(user_id: String, new_password: String) -> Result<(), String> {
    let id = user_id.parse::<Uuid>().map_err(|_| "Invalid user ID")?;

    let response = authenticated_put_json(
        &format!("/api/v1/users/{}", id),
        &serde_json::json!({
            "password": new_password,
        }),
    )
    .await
    .map_err(|e| format!("Failed to reset password: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to reset password: {}", error_text))
    }
}
