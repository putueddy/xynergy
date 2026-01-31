use chrono::Datelike;
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
    pub include_weekend: bool,
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

/// Allocation data for editing
#[derive(Debug, Clone)]
pub struct AllocationEditData {
    pub id: String,
    pub resource_id: String,
    pub project_id: String,
    pub start_date: String,
    pub end_date: String,
    pub allocation_percentage: f64,
    pub include_weekend: bool,
}

/// Allocation form component
#[component]
pub fn AllocationForm(
    resources: Signal<Vec<ResourceOption>>,
    projects: Signal<Vec<ProjectOption>>,
    editing_allocation: Signal<Option<AllocationEditData>>,
    on_submit: Callback<AllocationFormData>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    // Start with empty values; sync from editing_allocation reactively
    let (resource_id, set_resource_id) = create_signal(String::new());
    let (project_id, set_project_id) = create_signal(String::new());
    let (start_date, set_start_date) = create_signal(String::new());
    let (end_date, set_end_date) = create_signal(String::new());
    let (allocation_percentage, set_allocation_percentage) = create_signal("100".to_string());
    let (include_weekend, set_include_weekend) = create_signal(false);
    let (total_days, set_total_days) = create_signal(0);
    let (hours_per_day, set_hours_per_day) = create_signal(0.0);

    // Keep form state in sync when edit selection changes
    create_effect(move |_| {
        if let Some(edit_data) = editing_allocation.get() {
            set_resource_id.set(edit_data.resource_id);
            set_project_id.set(edit_data.project_id);
            set_start_date.set(edit_data.start_date);
            set_end_date.set(edit_data.end_date);
            set_allocation_percentage.set(format!("{:.0}", edit_data.allocation_percentage));
            set_include_weekend.set(edit_data.include_weekend);
        } else {
            set_resource_id.set(String::new());
            set_project_id.set(String::new());
            set_start_date.set(String::new());
            set_end_date.set(String::new());
            set_allocation_percentage.set("100".to_string());
            set_include_weekend.set(false);
        }
    });

    // Calculate total days and hours per day when dates or include_weekend changes
    create_effect(move |_| {
        let start = start_date.get();
        let end = end_date.get();
        let include_wknd = include_weekend.get();

        if !start.is_empty() && !end.is_empty() {
            if let (Ok(start_date), Ok(end_date)) = (
                chrono::NaiveDate::parse_from_str(&start, "%Y-%m-%d"),
                chrono::NaiveDate::parse_from_str(&end, "%Y-%m-%d"),
            ) {
                let days = if include_wknd {
                    // Include all days
                    (end_date - start_date).num_days() + 1
                } else {
                    // Count only working days (exclude weekends)
                    let mut count = 0;
                    let mut current = start_date;
                    while current <= end_date {
                        let weekday = current.weekday();
                        if weekday != chrono::Weekday::Sat && weekday != chrono::Weekday::Sun {
                            count += 1;
                        }
                        current = current + chrono::Duration::days(1);
                    }
                    count
                };
                set_total_days.set(days.max(0) as i32);

                // Calculate hours per day (8 hours * allocation_percentage / 100)
                if let Ok(percentage) = allocation_percentage.get().parse::<f64>() {
                    let hours = 8.0 * (percentage / 100.0);
                    set_hours_per_day.set(hours);
                }
            }
        }
    });

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        on_submit.call(AllocationFormData {
            resource_id: resource_id.get(),
            project_id: project_id.get(),
            start_date: start_date.get(),
            end_date: end_date.get(),
            allocation_percentage: allocation_percentage.get(),
            include_weekend: include_weekend.get(),
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
                        <option value="" disabled>"Select resource..."</option>
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
                        <option value="" disabled>"Select project..."</option>
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

            <div class="grid grid-cols-3 gap-4">
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        "Allocation Percentage"
                    </label>
                    <div class="flex items-center space-x-2 mt-1">
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

                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        "Hours/Day"
                    </label>
                    <div class="mt-1 block w-full rounded-md border border-gray-300 bg-gray-50 px-3 py-2 text-sm text-gray-700 dark:bg-gray-700 dark:border-gray-600 dark:text-white">
                        {move || format!("{:.1} hours", hours_per_day.get())}
                    </div>
                </div>

                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        "Total Days"
                    </label>
                    <div class="flex items-center space-x-2 mt-1">
                        <div class="flex-1 rounded-md border border-gray-300 bg-gray-50 px-3 py-2 text-sm text-gray-700 dark:bg-gray-700 dark:border-gray-600 dark:text-white">
                            {move || total_days.get().to_string()}
                        </div>
                        <label class="flex items-center space-x-2 text-sm">
                            <input
                                type="checkbox"
                                prop:checked=include_weekend
                                on:change=move |ev| set_include_weekend.set(event_target_checked(&ev))
                            />
                            <span class="text-gray-700 dark:text-gray-300">"Include Weekend"</span>
                        </label>
                    </div>
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
                    {move || if editing_allocation.get().is_some() { "Update Allocation" } else { "Create Allocation" }}
                </button>
            </div>
        </form>
    }
}
