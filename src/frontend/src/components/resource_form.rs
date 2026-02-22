use leptos::*;

/// Resource form data
#[derive(Debug, Clone, Default)]
pub struct ResourceFormData {
    pub name: String,
    pub resource_type: String,
    pub capacity: Option<f64>,
    pub department_id: Option<uuid::Uuid>,
}

/// Resource form component
#[component]
pub fn ResourceForm(
    initial_data: Signal<Option<ResourceFormData>>,
    on_submit: Callback<ResourceFormData>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let (name, set_name) = create_signal(String::new());
    let (resource_type, set_resource_type) = create_signal(String::new());
    let (capacity, set_capacity) = create_signal(String::new());

    create_effect(move |_| {
        if let Some(data) = initial_data.get() {
            set_name.set(data.name);
            set_resource_type.set(data.resource_type);
            set_capacity.set(data.capacity.map(|c| c.to_string()).unwrap_or_default());
        } else {
            set_name.set(String::new());
            set_resource_type.set(String::new());
            set_capacity.set(String::new());
        }
    });

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let capacity_val = capacity.get().parse::<f64>().ok();
        on_submit.call(ResourceFormData {
            name: name.get(),
            resource_type: resource_type.get(),
            capacity: capacity_val,
            department_id: None, // TODO: Add department selection
        });
    };

    view! {
        <form class="space-y-4" on:submit=handle_submit>
            <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                    "Name"
                </label>
                <input
                    type="text"
                    required
                    class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    placeholder="Resource name"
                    prop:value=name
                    on:input=move |ev| set_name.set(event_target_value(&ev))
                />
            </div>

            <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                    "Type"
                </label>
                <select
                    required
                    class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    prop:value=resource_type
                    on:change=move |ev| set_resource_type.set(event_target_value(&ev))
                >
                    <option value="" disabled>"Select type..."</option>
                    <option value="employee">"Employee"</option>
                    <option value="contractor">"Contractor"</option>
                    <option value="equipment">"Equipment"</option>
                    <option value="room">"Room"</option>
                </select>
            </div>

            <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                    "Capacity (hours/day)"
                </label>
                <input
                    type="number"
                    min="0"
                    max="24"
                    step="0.5"
                    class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    placeholder="8"
                    prop:value=capacity
                    on:input=move |ev| set_capacity.set(event_target_value(&ev))
                />
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
