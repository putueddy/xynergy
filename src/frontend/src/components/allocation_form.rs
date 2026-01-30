use leptos::*;
use uuid::Uuid;

/// Allocation form data
#[derive(Debug, Clone, Default)]
pub struct AllocationFormData {
    pub resource_id: String,
    pub project_id: String,
    pub start_date: String,
    pub end_date: String,
    pub allocation_percentage: String,
}

/// Resource option for dropdown
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceOption {
    pub id: Uuid,
    pub name: String,
}

/// Project option for dropdown
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectOption {
    pub id: Uuid,
    pub name: String,
}

/// Allocation form component
#[component]
pub fn AllocationForm(
    resources: Signal<Vec<ResourceOption>>,
    projects: Signal<Vec<ProjectOption>>,
    on_submit: Callback<AllocationFormData>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let (resource_id, set_resource_id) = create_signal(String::new());
    let (project_id, set_project_id) = create_signal(String::new());
    let (start_date, set_start_date) = create_signal(String::new());
    let (end_date, set_end_date) = create_signal(String::new());
    let (allocation_percentage, set_allocation_percentage) = create_signal("100".to_string());

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        on_submit.call(AllocationFormData {
            resource_id: resource_id.get(),
            project_id: project_id.get(),
            start_date: start_date.get(),
            end_date: end_date.get(),
            allocation_percentage: allocation_percentage.get(),
        });
    };

    view! {
        <form class="space-y-4" on:submit=handle_submit>
            <div class="grid grid-cols-2 gap-4">
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        "Resource"
                    </label>
                    <select
                        required
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                        prop:value=resource_id
                        on:change=move |ev| set_resource_id.set(event_target_value(&ev))
                    >
                        <option value="" disabled selected>"Select resource..."</option>
                        {move || resources.get().into_iter().map(|r| {
                            view! {
                                <option value={r.id.to_string()}>{r.name.clone()}</option>
                            }
                        }).collect_view()}
                    </select>
                </div>

                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        "Project"
                    </label>
                    <select
                        required
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                        prop:value=project_id
                        on:change=move |ev| set_project_id.set(event_target_value(&ev))
                    >
                        <option value="" disabled selected>"Select project..."</option>
                        {move || projects.get().into_iter().map(|p| {
                            view! {
                                <option value={p.id.to_string()}>{p.name.clone()}</option>
                            }
                        }).collect_view()}
                    </select>
                </div>
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
                    "Allocation Percentage"
                </label>
                <div class="flex items-center space-x-2">
                    <input
                        type="range"
                        min="1"
                        max="100"
                        required
                        class="flex-1"
                        prop:value=allocation_percentage
                        on:input=move |ev| set_allocation_percentage.set(event_target_value(&ev))
                    />
                    <span class="text-sm font-medium text-gray-700 dark:text-gray-300 w-12">
                        {move || format!("{}%", allocation_percentage.get())}
                    </span>
                </div>
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
                    "Create Allocation"
                </button>
            </div>
        </form>
    }
}
