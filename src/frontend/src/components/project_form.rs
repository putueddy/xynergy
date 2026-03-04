use leptos::*;

/// Project form data
#[derive(Debug, Clone, Default)]
pub struct ProjectFormData {
    pub name: String,
    pub client: String,
    pub description: String,
    pub start_date: String,
    pub end_date: String,
    pub status: String,
    pub total_budget_idr: String,
    pub budget_hr_idr: String,
    pub budget_software_idr: String,
    pub budget_hardware_idr: String,
    pub budget_overhead_idr: String,
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
    let (client, set_client) = create_signal(
        initial_data
            .as_ref()
            .map(|d| d.client.clone())
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
            .unwrap_or_else(|| "Active".to_string()),
    );
    let (total_budget_idr, set_total_budget_idr) = create_signal(
        initial_data
            .as_ref()
            .map(|d| d.total_budget_idr.clone())
            .unwrap_or_default(),
    );
    let (budget_hr_idr, set_budget_hr_idr) = create_signal(
        initial_data
            .as_ref()
            .map(|d| d.budget_hr_idr.clone())
            .unwrap_or_default(),
    );
    let (budget_software_idr, set_budget_software_idr) = create_signal(
        initial_data
            .as_ref()
            .map(|d| d.budget_software_idr.clone())
            .unwrap_or_default(),
    );
    let (budget_hardware_idr, set_budget_hardware_idr) = create_signal(
        initial_data
            .as_ref()
            .map(|d| d.budget_hardware_idr.clone())
            .unwrap_or_default(),
    );
    let (budget_overhead_idr, set_budget_overhead_idr) = create_signal(
        initial_data
            .as_ref()
            .map(|d| d.budget_overhead_idr.clone())
            .unwrap_or_default(),
    );

    let budget_percentages = move || {
        let total: i64 = total_budget_idr.get().parse().unwrap_or(0);
        let hr: i64 = budget_hr_idr.get().parse().unwrap_or(0);
        let sw: i64 = budget_software_idr.get().parse().unwrap_or(0);
        let hw: i64 = budget_hardware_idr.get().parse().unwrap_or(0);
        let oh: i64 = budget_overhead_idr.get().parse().unwrap_or(0);
        let sum = hr + sw + hw + oh;

        if total > 0 {
            (
                hr as f64 / total as f64 * 100.0,
                sw as f64 / total as f64 * 100.0,
                hw as f64 / total as f64 * 100.0,
                oh as f64 / total as f64 * 100.0,
                sum == total,
            )
        } else {
            (0.0, 0.0, 0.0, 0.0, false)
        }
    };

    let budget_totals = move || {
        let total: i64 = total_budget_idr.get().parse().unwrap_or(0);
        let hr: i64 = budget_hr_idr.get().parse().unwrap_or(0);
        let sw: i64 = budget_software_idr.get().parse().unwrap_or(0);
        let hw: i64 = budget_hardware_idr.get().parse().unwrap_or(0);
        let oh: i64 = budget_overhead_idr.get().parse().unwrap_or(0);
        let sum = hr + sw + hw + oh;
        (total, sum)
    };

    let (validation_error, set_validation_error) = create_signal(Option::<String>::None);

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();

        let parse_budget = |raw: &str, field_name: &str| -> std::result::Result<i64, String> {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Ok(0);
            }

            let value = trimmed
                .parse::<i64>()
                .map_err(|_| format!("{} must be a whole number", field_name))?;
            if value < 0 {
                return Err(format!("{} cannot be negative", field_name));
            }

            Ok(value)
        };

        let total = match parse_budget(&total_budget_idr.get(), "Total budget") {
            Ok(value) => value,
            Err(err) => {
                set_validation_error.set(Some(err));
                return;
            }
        };
        let hr = match parse_budget(&budget_hr_idr.get(), "HR budget") {
            Ok(value) => value,
            Err(err) => {
                set_validation_error.set(Some(err));
                return;
            }
        };
        let sw = match parse_budget(&budget_software_idr.get(), "Software budget") {
            Ok(value) => value,
            Err(err) => {
                set_validation_error.set(Some(err));
                return;
            }
        };
        let hw = match parse_budget(&budget_hardware_idr.get(), "Hardware budget") {
            Ok(value) => value,
            Err(err) => {
                set_validation_error.set(Some(err));
                return;
            }
        };
        let oh = match parse_budget(&budget_overhead_idr.get(), "Overhead budget") {
            Ok(value) => value,
            Err(err) => {
                set_validation_error.set(Some(err));
                return;
            }
        };

        if total <= 0 {
            set_validation_error.set(Some("Total budget must be greater than 0".to_string()));
            return;
        }
        let sum = hr + sw + hw + oh;
        if sum != total {
            set_validation_error.set(Some(format!(
                "Budget categories sum ({}) must equal total budget ({})",
                sum, total
            )));
            return;
        }

        set_validation_error.set(None);
        on_submit.call(ProjectFormData {
            name: name.get(),
            client: client.get(),
            description: description.get(),
            start_date: start_date.get(),
            end_date: end_date.get(),
            status: status.get(),
            total_budget_idr: total.to_string(),
            budget_hr_idr: hr.to_string(),
            budget_software_idr: sw.to_string(),
            budget_hardware_idr: hw.to_string(),
            budget_overhead_idr: oh.to_string(),
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

            <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                    "Client"
                </label>
                <input
                    type="text"
                    class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    placeholder="Client name"
                    prop:value=client
                    on:input=move |ev| set_client.set(event_target_value(&ev))
                />
            </div>

            <div class="rounded-lg border border-gray-200 dark:border-gray-700 p-4 space-y-4">
                <h3 class="text-sm font-semibold text-gray-900 dark:text-white">
                    "Budget Categories (IDR)"
                </h3>

                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        "Total Budget"
                    </label>
                    <input
                        type="number"
                        required
                        min="1"
                        step="1"
                        class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                        placeholder="Total budget in IDR"
                        prop:value=total_budget_idr
                        on:input=move |ev| set_total_budget_idr.set(event_target_value(&ev))
                    />
                </div>

                <div class="grid grid-cols-2 gap-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                            "HR Budget"
                        </label>
                        <input
                            type="number"
                            min="0"
                            step="1"
                            class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                            placeholder="0"
                            prop:value=budget_hr_idr
                            on:input=move |ev| set_budget_hr_idr.set(event_target_value(&ev))
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                            "Software Budget"
                        </label>
                        <input
                            type="number"
                            min="0"
                            step="1"
                            class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                            placeholder="0"
                            prop:value=budget_software_idr
                            on:input=move |ev| set_budget_software_idr.set(event_target_value(&ev))
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                            "Hardware Budget"
                        </label>
                        <input
                            type="number"
                            min="0"
                            step="1"
                            class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                            placeholder="0"
                            prop:value=budget_hardware_idr
                            on:input=move |ev| set_budget_hardware_idr.set(event_target_value(&ev))
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                            "Overhead Budget"
                        </label>
                        <input
                            type="number"
                            min="0"
                            step="1"
                            class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                            placeholder="0"
                            prop:value=budget_overhead_idr
                            on:input=move |ev| set_budget_overhead_idr.set(event_target_value(&ev))
                        />
                    </div>
                </div>

                <div class="rounded-md bg-gray-50 dark:bg-gray-800/60 p-3 text-sm text-gray-700 dark:text-gray-300 space-y-1">
                    {move || {
                        let (hr_pct, sw_pct, hw_pct, oh_pct, is_valid) = budget_percentages();
                        view! {
                            <>
                                <p>{format!("HR: {:.1}%", hr_pct)}</p>
                                <p>{format!("Software: {:.1}%", sw_pct)}</p>
                                <p>{format!("Hardware: {:.1}%", hw_pct)}</p>
                                <p>{format!("Overhead: {:.1}%", oh_pct)}</p>
                                <p class=if is_valid { "text-green-600 dark:text-green-400" } else { "text-gray-500 dark:text-gray-400" }>
                                    {if is_valid { "Category sum matches total budget" } else { "Category sum does not match total budget" }}
                                </p>
                            </>
                        }
                    }}
                </div>

                {move || {
                    let (total, sum) = budget_totals();
                    if total > 0 && sum != total {
                        view! {
                            <p class="text-sm text-amber-600 dark:text-amber-400">
                                {format!("⚠ Category sum ({}) does not equal total budget ({})", sum, total)}
                            </p>
                        }
                            .into_view()
                    } else {
                        view! { <></> }.into_view()
                    }
                }}

                {move || validation_error.get().map(|err| {
                    view! {
                        <p class="text-sm text-red-600 dark:text-red-400">{err}</p>
                    }
                })}
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
                    <option value="Active">"Active"</option>
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
