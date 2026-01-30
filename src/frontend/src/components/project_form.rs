use chrono::NaiveDate;
use leptos::*;

/// Project form data
#[derive(Debug, Clone, Default)]
pub struct ProjectFormData {
    pub name: String,
    pub description: String,
    pub start_date: String,
    pub end_date: String,
    pub status: String,
}

/// Project form component
#[component]
pub fn ProjectForm(
    #[prop(optional)] initial_data: Option<ProjectFormData>,
    on_submit: Callback<ProjectFormData>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let (name, set_name) = create_signal(
        initial_data
            .as_ref()
            .map(|d| d.name.clone())
            .unwrap_or_default(),
    );
    let (description, set_description) = create_signal(
        initial_data
            .as_ref()
            .map(|d| d.description.clone())
            .unwrap_or_default(),
    );
    let (start_date, set_start_date) = create_signal(
        initial_data
            .as_ref()
            .map(|d| d.start_date.clone())
            .unwrap_or_default(),
    );
    let (end_date, set_end_date) = create_signal(
        initial_data
            .as_ref()
            .map(|d| d.end_date.clone())
            .unwrap_or_default(),
    );
    let (status, set_status) = create_signal(
        initial_data
            .as_ref()
            .map(|d| d.status.clone())
            .unwrap_or_else(|| "planning".to_string()),
    );

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        on_submit.call(ProjectFormData {
            name: name.get(),
            description: description.get(),
            start_date: start_date.get(),
            end_date: end_date.get(),
            status: status.get(),
        });
    };

    view! {
        <form class="space-y-4" on:submit=handle_submit>
            <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                    "Project Name"
                </label>
                <input
                    type="text"
                    required
                    class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    placeholder="Project name"
                    prop:value=name
                    on:input=move |ev| set_name.set(event_target_value(&ev))
                />
            </div>

            <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                    "Description"
                </label>
                <textarea
                    class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    placeholder="Project description"
                    rows="3"
                    prop:value=description
                    on:input=move |ev| set_description.set(event_target_value(&ev))
                />
            </div>

            <div class="grid grid-cols-2 gap-4">
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        "Start Date"
                    </label>
                    <input
                        type="date"
                        required
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                        prop:value=start_date
                        on:input=move |ev| set_start_date.set(event_target_value(&ev))
                    />
                </div>

                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        "End Date"
                    </label>
                    <input
                        type="date"
                        required
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                        prop:value=end_date
                        on:input=move |ev| set_end_date.set(event_target_value(&ev))
                    />
                </div>
            </div>

            <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                    "Status"
                </label>
                <select
                    required
                    class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    prop:value=status
                    on:change=move |ev| set_status.set(event_target_value(&ev))
                >
                    <option value="planning">"Planning"</option>
                    <option value="active">"Active"</option>
                    <option value="completed">"Completed"</option>
                    <option value="cancelled">"Cancelled"</option>
                </select>
            </div>

            <div class="flex justify-end space-x-3 pt-4">
                <button
                    type="button"
                    class="btn-secondary"
                    on:click=move |_| on_cancel.call(())
                >
                    "Cancel"
                </button>
                <button
                    type="submit"
                    class="btn-primary"
                >
                    "Save"
                </button>
            </div>
        </form>
    }
}
