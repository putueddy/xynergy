use leptos::prelude::*;

/// Resource form data
#[derive(Debug, Clone, Default)]
pub struct ResourceFormData {
    pub name: String,
    pub resource_type: String,
    pub capacity: Option<f64>,
    pub department_id: Option<uuid::Uuid>,
    pub employment_start_date: Option<String>,
}

/// Resource form component
#[component]
pub fn ResourceForm(
    initial_data: Signal<Option<ResourceFormData>>,
    departments: Signal<Vec<(String, String)>>,
    on_submit: Callback<ResourceFormData>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (resource_type, set_resource_type) = signal(String::new());
    let (capacity, set_capacity) = signal(String::new());
    let (department_id, set_department_id) = signal(String::new());
    let (employment_start_date, set_employment_start_date) = signal(String::new());

    Effect::new(move |_| {
        if let Some(data) = initial_data.get() {
            set_name.set(data.name);
            set_resource_type.set(data.resource_type);
            set_capacity.set(data.capacity.map(|c| c.to_string()).unwrap_or_default());
            set_department_id.set(
                data.department_id
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
            );
            set_employment_start_date.set(data.employment_start_date.unwrap_or_default());
        } else {
            set_name.set(String::new());
            set_resource_type.set(String::new());
            set_capacity.set(String::new());
            set_department_id.set(String::new());
            set_employment_start_date.set(String::new());
        }
    });

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let capacity_val = capacity.get().parse::<f64>().ok();
        let selected_department = department_id.get();
        let selected_start_date = employment_start_date.get();
        on_submit.run(ResourceFormData {
            name: name.get(),
            resource_type: resource_type.get(),
            capacity: capacity_val,
            department_id: if selected_department.is_empty() {
                None
            } else {
                selected_department.parse::<uuid::Uuid>().ok()
            },
            employment_start_date: if selected_start_date.is_empty() {
                None
            } else {
                Some(selected_start_date)
            },
        });
    };

    view! {
        <form class="space-y-3" on:submit=handle_submit>
            <div>
                <label class="label">
                    "Name"
                </label>
                <input
                    type="text"
                    required
                    class="input"
                    placeholder="Resource name"
                    prop:value=name
                    on:input=move |ev| set_name.set(event_target_value(&ev))
                />
            </div>

            <div>
                <label class="label">
                    "Type"
                </label>
                <select
                    required
                    class="input"
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
                <label class="label">
                    "Department"
                </label>
                <select
                    class="input"
                    prop:value=department_id
                    on:change=move |ev| set_department_id.set(event_target_value(&ev))
                >
                    <option value="">"Select department..."</option>
                    {move || departments.get().into_iter().map(|(id, name)| {
                        view! { <option value=id>{name}</option> }
                    }).collect_view()}
                </select>
            </div>

            <div>
                <label class="label">
                    "Employment Start Date"
                </label>
                <input
                    type="date"
                    class="input"
                    prop:value=employment_start_date
                    on:input=move |ev| set_employment_start_date.set(event_target_value(&ev))
                />
            </div>

            <div>
                <label class="label">
                    "Capacity (hours/day)"
                </label>
                <input
                    type="number"
                    min="0"
                    max="24"
                    step="0.5"
                    class="input"
                    placeholder="8"
                    prop:value=capacity
                    on:input=move |ev| set_capacity.set(event_target_value(&ev))
                />
            </div>

            <div class="flex justify-end gap-2 pt-3">
                <button
                    type="button"
                    class="btn-secondary btn-press"
                    on:click=move |_| on_cancel.run(())
                >
                    "Cancel"
                </button>
                <button
                    type="submit"
                    class="btn-primary btn-press"
                >
                    "Save"
                </button>
            </div>
        </form>
    }
}
