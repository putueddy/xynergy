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
    // Data signals
    let (users, set_users) = create_signal(Vec::new());
    let (departments, set_departments) = create_signal(Vec::new());
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);

    let (show_form, set_show_form) = create_signal(false);
    let (editing_user, set_editing_user) = create_signal(Option::<User>::None);
    let (form_submitting, set_form_submitting) = create_signal(false);
    let (deleting_id, set_deleting_id) = create_signal(Option::<String>::None);

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
            "admin" => "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200",
            "project_manager" => "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200",
            _ => "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200",
        }
    };

    view! {
        <div class="space-y-6">
            <div class="flex items-center justify-between">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                        "User Management"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400 mt-1">
                        "Manage system users and their roles"
                    </p>
                </div>

                <div class="flex items-center space-x-3">
                    <button
                        class="btn-primary"
                        on:click=move |_| set_show_form.set(true)
                    >
                        "Add User"
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
                    let is_edit = editing_user.get().is_some();
                    let title = if is_edit { "Edit User" } else { "Create User" };
                    view! {
                        <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 relative">
                            <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">
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
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Name"</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Email"</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Role"</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Department"</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Actions"</th>
                            </tr>
                        </thead>
                        <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                            {move || {
                                if loading.get() {
                                    view! {
                                        <tr>
                                            <td colspan="5" class="px-6 py-4 text-center text-gray-500 dark:text-gray-400">
                                                <div class="flex items-center justify-center">
                                                    <div class="spinner mr-2"></div>
                                                    "Loading users..."
                                                </div>
                                            </td>
                                        </tr>
                                    }.into_view()
                                } else if users.get().is_empty() {
                                    view! {
                                        <tr>
                                            <td colspan="5" class="px-6 py-4 text-center text-gray-500 dark:text-gray-400">
                                                "No users found."
                                            </td>
                                        </tr>
                                    }.into_view()
                                } else {
                                    users.get().into_iter().map(|user| {
                                        let user_id = user.id.to_string();
                                        let user_for_edit = user.clone();
                                        let dept_name = get_department_name(user.department_id);
                                        let role_class = get_role_badge_class(&user.role);
                                        let role_display = user.role.replace("_", " ");

                                        view! {
                                            <tr class="hover:bg-gray-50 dark:hover:bg-gray-700">
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <div class="flex items-center">
                                                        <div class="flex-shrink-0 h-10 w-10 rounded-full bg-blue-500 flex items-center justify-center text-white font-semibold">
                                                            {format!("{}{}",
                                                                user.first_name.chars().next().unwrap_or('U'),
                                                                user.last_name.chars().next().unwrap_or('N')
                                                            )}
                                                        </div>
                                                        <div class="ml-4">
                                                            <div class="text-sm font-medium text-gray-900 dark:text-white">
                                                                {format!("{} {}", user.first_name, user.last_name)}
                                                            </div>
                                                        </div>
                                                    </div>
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                    {user.email.clone()}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <span class={format!("px-2 inline-flex text-xs leading-5 font-semibold rounded-full {}", role_class)}>
                                                        {role_display}
                                                    </span>
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                    {dept_name}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                    <div class="flex items-center space-x-2">
                                                        <button
                                                            class="text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
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
                                                        {move || {
                                                            let is_deleting = deleting_id.get() == Some(user_id.clone());
                                                            view! {
                                                                <button
                                                                    class="text-red-600 hover:text-red-800 dark:text-red-400 dark:hover:text-red-300 disabled:opacity-50 disabled:cursor-not-allowed"
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
                                                                                        // Reload users
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
    let response = reqwest::get("http://localhost:3000/api/v1/users")
        .await
        .map_err(|e| format!("Failed to fetch users: {}", e))?;

    if response.status().is_success() {
        response
            .json::<Vec<User>>()
            .await
            .map_err(|e| format!("Failed to parse users: {}", e))
    } else {
        Err(format!("Failed to fetch users: {}", response.status()))
    }
}

/// Fetch all departments from API
async fn fetch_departments() -> Result<Vec<Department>, String> {
    let response = reqwest::get("http://localhost:3000/api/v1/departments")
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

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:3000/api/v1/users")
        .json(&serde_json::json!({
            "email": form_data.email,
            "password": form_data.password,
            "first_name": form_data.first_name,
            "last_name": form_data.last_name,
            "role": form_data.role,
            "department_id": department_id,
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to create user: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to create user: {}", error_text))
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

    let client = reqwest::Client::new();
    let response = client
        .put(&format!("http://localhost:3000/api/v1/users/{}", id))
        .json(&serde_json::json!({
            "email": form_data.email,
            "first_name": form_data.first_name,
            "last_name": form_data.last_name,
            "role": form_data.role,
            "department_id": department_id,
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to update user: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to update user: {}", error_text))
    }
}

/// Delete a user
async fn delete_user(user_id: String) -> Result<(), String> {
    let id = user_id.parse::<Uuid>().map_err(|_| "Invalid user ID")?;

    let client = reqwest::Client::new();
    let response = client
        .delete(&format!("http://localhost:3000/api/v1/users/{}", id))
        .send()
        .await
        .map_err(|e| format!("Failed to delete user: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to delete user: {}", error_text))
    }
}
