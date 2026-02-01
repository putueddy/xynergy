use leptos::*;
use serde::{Deserialize, Serialize};

/// Holiday form data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HolidayFormData {
    pub name: String,
    pub date: String,
    pub description: String,
}

/// Holiday form component
#[component]
pub fn HolidayForm(
    #[prop(default = None)] editing_holiday: Option<HolidayFormData>,
    #[prop(default = false)] is_submitting: bool,
    on_submit: Callback<HolidayFormData>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let (name, set_name) = create_signal(
        editing_holiday
            .as_ref()
            .map(|h| h.name.clone())
            .unwrap_or_default(),
    );
    let (date, set_date) = create_signal(
        editing_holiday
            .as_ref()
            .map(|h| h.date.clone())
            .unwrap_or_default(),
    );
    let (description, set_description) = create_signal(
        editing_holiday
            .as_ref()
            .map(|h| h.description.clone())
            .unwrap_or_default(),
    );

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        on_submit.call(HolidayFormData {
            name: name.get(),
            date: date.get(),
            description: description.get(),
        });
    };

    let is_edit = editing_holiday.is_some();

    view! {
        <form class="space-y-4" on:submit=handle_submit>
            <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                    "Holiday Name"
                </label>
                <input
                    type="text"
                    required
                    class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    prop:value=name
                    on:input=move |ev| set_name.set(event_target_value(&ev))
                    placeholder="e.g., New Year's Day"
                />
            </div>

            <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                    "Date"
                </label>
                <input
                    type="date"
                    required
                    class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    prop:value=date
                    on:input=move |ev| set_date.set(event_target_value(&ev))
                />
            </div>

            <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                    "Description"
                </label>
                <textarea
                    class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    prop:value=description
                    on:input=move |ev| set_description.set(event_target_value(&ev))
                    placeholder="Optional description"
                    rows="3"
                />
            </div>

            <div class="flex justify-end space-x-3 pt-4">
                <button
                    type="button"
                    class="btn-secondary"
                    disabled=is_submitting
                    on:click=move |_| on_cancel.call(())
                >
                    "Cancel"
                </button>
                <button
                    type="submit"
                    class="btn-primary"
                    disabled=is_submitting
                >
                    {move || {
                        if is_submitting {
                            if is_edit { "Updating..." } else { "Creating..." }
                        } else {
                            if is_edit { "Update Holiday" } else { "Create Holiday" }
                        }
                    }}
                </button>
            </div>
        </form>
    }
}
