use leptos::*;
use uuid::Uuid;

/// User form data structure
#[derive(Debug, Clone, Default)]
pub struct UserFormData {
    pub email: String,
    pub password: String,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    pub department_id: String,
}

/// User edit data structure
#[derive(Debug, Clone)]
pub struct UserEditData {
    pub id: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    pub department_id: String,
}

/// Department option for dropdown
#[derive(Debug, Clone, PartialEq)]
pub struct DepartmentOption {
    pub id: Uuid,
    pub name: String,
}

/// User form component
#[component]
pub fn UserForm(
    #[prop(optional)] departments: Vec<DepartmentOption>,
    editing_user: Option<UserEditData>,
    #[prop(into)] on_submit: Callback<UserFormData>,
    #[prop(into)] on_cancel: Callback<()>,
    #[prop(default = false)] is_submitting: bool,
) -> impl IntoView {
    let is_edit = editing_user.is_some();

    // Form fields - initialize with empty values
    let (email, set_email) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (first_name, set_first_name) = create_signal(String::new());
    let (last_name, set_last_name) = create_signal(String::new());
    let (role, set_role) = create_signal("team_member".to_string());
    let (department_id, set_department_id) = create_signal(String::new());

    // Update form fields when editing_user changes
    create_effect(move |_| {
        if let Some(user) = &editing_user {
            set_email.set(user.email.clone());
            set_first_name.set(user.first_name.clone());
            set_last_name.set(user.last_name.clone());
            set_role.set(user.role.clone());
            set_department_id.set(user.department_id.clone());
            set_password.set(String::new()); // Clear password field on edit
        } else {
            // Reset form for new user
            set_email.set(String::new());
            set_first_name.set(String::new());
            set_last_name.set(String::new());
            set_role.set("team_member".to_string());
            set_department_id.set(String::new());
            set_password.set(String::new());
        }
    });

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();

        let form_data = UserFormData {
            email: email.get(),
            password: password.get(),
            first_name: first_name.get(),
            last_name: last_name.get(),
            role: role.get(),
            department_id: department_id.get(),
        };

        on_submit.call(form_data);
    };

    view! {
        <form on:submit=handle_submit class="space-y-4">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                // Email
                <div>
                    <label for="email" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        "Email *"
                    </label>
                    <input
                        type="email"
                        id="email"
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white sm:text-sm"
                        placeholder="user@example.com"
                        prop:value=email
                        on:input=move |ev| set_email.set(event_target_value(&ev))
                        required
                        disabled=is_edit
                    />
                </div>

                // Password (only for new users)
                {if !is_edit {
                    view! {
                        <div>
                            <label for="password" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                                "Password *"
                            </label>
                            <input
                                type="password"
                                id="password"
                                class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white sm:text-sm"
                                placeholder="••••••••"
                                prop:value=password
                                on:input=move |ev| set_password.set(event_target_value(&ev))
                                required=!is_edit
                            />
                        </div>
                    }.into_view()
                } else {
                    view! { <div></div> }.into_view()
                }}

                // First Name
                <div>
                    <label for="first_name" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        "First Name *"
                    </label>
                    <input
                        type="text"
                        id="first_name"
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white sm:text-sm"
                        placeholder="John"
                        prop:value=first_name
                        on:input=move |ev| set_first_name.set(event_target_value(&ev))
                        required
                    />
                </div>

                // Last Name
                <div>
                    <label for="last_name" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        "Last Name *"
                    </label>
                    <input
                        type="text"
                        id="last_name"
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white sm:text-sm"
                        placeholder="Doe"
                        prop:value=last_name
                        on:input=move |ev| set_last_name.set(event_target_value(&ev))
                        required
                    />
                </div>

                // Role
                <div>
                    <label for="role" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        "Role *"
                    </label>
                    <select
                        id="role"
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white sm:text-sm"
                        prop:value=role
                        on:change=move |ev| set_role.set(event_target_value(&ev))
                        required
                    >
                        <option value="admin">"Admin"</option>
                        <option value="hr">"HR"</option>
                        <option value="department_head">"Department Head"</option>
                        <option value="project_manager">"Project Manager"</option>
                        <option value="finance">"Finance"</option>
                        <option value="team_member">"Team Member"</option>
                    </select>
                </div>

                // Department
                <div>
                    <label for="department" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        "Department"
                    </label>
                    <select
                        id="department"
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white sm:text-sm"
                        prop:value=department_id
                        on:change=move |ev| set_department_id.set(event_target_value(&ev))
                    >
                        <option value="">"-- Select Department --"</option>
                        {departments.iter().map(|dept| {
                            view! {
                                <option value={dept.id.to_string()}>{dept.name.clone()}</option>
                            }
                        }).collect_view()}
                    </select>
                </div>
            </div>

            // Form buttons
            <div class="flex justify-end space-x-3 pt-4">
                <button
                    type="button"
                    class="btn-secondary"
                    on:click=move |_| on_cancel.call(())
                    disabled=is_submitting
                >
                    "Cancel"
                </button>
                <button
                    type="submit"
                    class="btn-primary"
                    disabled=is_submitting
                >
                    {if is_submitting {
                        if is_edit { "Saving..." } else { "Creating..." }
                    } else {
                        if is_edit { "Save Changes" } else { "Create User" }
                    }}
                </button>
            </div>
        </form>
    }
}
