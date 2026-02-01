use leptos::*;
use serde::Deserialize;
use uuid::Uuid;

/// Department form data structure
#[derive(Debug, Clone, Default)]
pub struct DepartmentFormData {
    pub name: String,
    pub head_id: String,
}

/// Department edit data structure
#[derive(Debug, Clone)]
pub struct DepartmentEditData {
    pub id: String,
    pub name: String,
    pub head_id: String,
}

/// Head candidate option for dropdown
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HeadCandidate {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

/// Department form component
#[component]
pub fn DepartmentForm(
    #[prop(optional)] head_candidates: Vec<HeadCandidate>,
    editing_department: Option<DepartmentEditData>,
    #[prop(into)] on_submit: Callback<DepartmentFormData>,
    #[prop(into)] on_cancel: Callback<()>,
    #[prop(default = false)] is_submitting: bool,
) -> impl IntoView {
    let is_edit = editing_department.is_some();

    // Form fields
    let (name, set_name) = create_signal(
        editing_department
            .as_ref()
            .map(|d| d.name.clone())
            .unwrap_or_default(),
    );
    let (head_id, set_head_id) = create_signal(
        editing_department
            .as_ref()
            .map(|d| d.head_id.clone())
            .unwrap_or_default(),
    );

    // Update form fields when editing_department changes
    create_effect(move |_| {
        if let Some(dept) = &editing_department {
            set_name.set(dept.name.clone());
            set_head_id.set(dept.head_id.clone());
        } else {
            set_name.set(String::new());
            set_head_id.set(String::new());
        }
    });

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();

        let form_data = DepartmentFormData {
            name: name.get(),
            head_id: head_id.get(),
        };

        on_submit.call(form_data);
    };

    view! {
        <form on:submit=handle_submit class="space-y-4">
            <div class="grid grid-cols-1 gap-4">
                // Department Name
                <div>
                    <label for="name" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        "Department Name *"
                    </label>
                    <input
                        type="text"
                        id="name"
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white sm:text-sm"
                        placeholder="Engineering"
                        prop:value=name
                        on:input=move |ev| set_name.set(event_target_value(&ev))
                        required
                    />
                </div>

                // Department Head
                <div>
                    <label for="head" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        "Department Head"
                    </label>
                    <select
                        id="head"
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white sm:text-sm"
                        prop:value=head_id
                        on:change=move |ev| set_head_id.set(event_target_value(&ev))
                    >
                        <option value="">"-- Select Department Head --"</option>
                        {head_candidates.iter().map(|candidate| {
                            view! {
                                <option value={candidate.id.to_string()}>
                                    {format!("{} ({})", candidate.name, candidate.email)}
                                </option>
                            }
                        }).collect_view()}
                    </select>
                    <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                        "Only admin and project manager roles can be department heads"
                    </p>
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
                        if is_edit { "Save Changes" } else { "Create Department" }
                    }}
                </button>
            </div>
        </form>
    }
}
