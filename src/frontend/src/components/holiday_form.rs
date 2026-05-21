use leptos::prelude::*;
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
    let (name, set_name) = signal(
        editing_holiday
            .as_ref()
            .map(|h| h.name.clone())
            .unwrap_or_default(),
    );
    let (date, set_date) = signal(
        editing_holiday
            .as_ref()
            .map(|h| h.date.clone())
            .unwrap_or_default(),
    );
    let (description, set_description) = signal(
        editing_holiday
            .as_ref()
            .map(|h| h.description.clone())
            .unwrap_or_default(),
    );

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        on_submit.run(HolidayFormData {
            name: name.get(),
            date: date.get(),
            description: description.get(),
        });
    };

    let is_edit = editing_holiday.is_some();

    view! {
        <form class="space-y-3" on:submit=handle_submit>
            <div>
                <label class="label">
                    "Holiday Name"
                </label>
                <input
                    type="text"
                    required
                    class="input"
                    prop:value=name
                    on:input=move |ev| set_name.set(event_target_value(&ev))
                    placeholder="e.g., New Year's Day"
                />
            </div>

            <div>
                <label class="label">
                    "Date"
                </label>
                <input
                    type="date"
                    required
                    class="input"
                    prop:value=date
                    on:input=move |ev| set_date.set(event_target_value(&ev))
                />
            </div>

            <div>
                <label class="label">
                    "Description"
                </label>
                <textarea
                    class="input"
                    prop:value=description
                    on:input=move |ev| set_description.set(event_target_value(&ev))
                    placeholder="Optional description"
                    rows="3"
                />
            </div>

            <div class="flex justify-end gap-2 pt-3">
                <button
                    type="button"
                    class="btn-secondary btn-press"
                    disabled=is_submitting
                    on:click=move |_| on_cancel.run(())
                >
                    "Cancel"
                </button>
                <button
                    type="submit"
                    class="btn-primary btn-press"
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
