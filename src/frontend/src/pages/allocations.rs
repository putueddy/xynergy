use crate::auth::{authenticated_delete, authenticated_get, authenticated_post_json, authenticated_put_json, use_auth};
use crate::components::{
    AllocationEditData, AllocationForm, AllocationFormData, Footer, Header, ProjectOption,
    ResourceOption, TimelineChart,
};
use crate::timeline::{TimelineGroup, TimelineItem};
use chrono::Datelike;
use leptos::*;
use leptos_router::*;
use serde::Deserialize;
use uuid::Uuid;

/// Allocation data structure
#[derive(Debug, Clone, Deserialize)]
pub struct Allocation {
    pub id: Uuid,
    pub project_id: Uuid,
    pub resource_id: Uuid,
    pub start_date: String,
    pub end_date: String,
    pub allocation_percentage: f64,
    pub project_name: String,
    pub resource_name: String,
    #[serde(default)]
    pub include_weekend: bool,
}

/// Resource data for dropdown
#[derive(Debug, Clone, Deserialize)]
pub struct Resource {
    pub id: Uuid,
    pub name: String,
}

/// Holiday data structure
#[derive(Debug, Clone, Deserialize)]
pub struct Holiday {
    pub id: Uuid,
    pub name: String,
    pub date: String,
    pub description: Option<String>,
}

/// Project data for dropdown
#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
}

/// Allocations page component
#[component]
pub fn Allocations() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();

    // Redirect if not logged in
    {
        let navigate = navigate.clone();
        create_effect(move |_| {
            if !auth.is_authenticated.get() {
                navigate("/login", Default::default());
            }
        });
    }

    // Data signals
    let (allocations, set_allocations) = create_signal(Vec::new());
    let (resources, set_resources) = create_signal(Vec::new());
    let (projects, set_projects) = create_signal(Vec::new());
    let (holidays, set_holidays) = create_signal(Vec::new());
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);

    let (show_form, set_show_form) = create_signal(false);
    let (editing_allocation, set_editing_allocation) = create_signal(Option::<Allocation>::None);
    let (form_submitting, set_form_submitting) = create_signal(false);
    let (deleting_id, set_deleting_id) = create_signal(Option::<String>::None);

    // Load data on mount
    create_effect(move |_| {
        set_loading.set(true);
        spawn_local(async move {
            // Load allocations
            match fetch_allocations().await {
                Ok(data) => set_allocations.set(data),
                Err(e) => set_error.set(Some(e)),
            }

            // Load resources
            match fetch_resources().await {
                Ok(data) => set_resources.set(data),
                Err(e) => set_error.set(Some(e)),
            }

            // Load projects
            match fetch_projects().await {
                Ok(data) => set_projects.set(data),
                Err(e) => set_error.set(Some(e)),
            }

            // Load holidays
            match fetch_holidays().await {
                Ok(data) => set_holidays.set(data),
                Err(e) => set_error.set(Some(e)),
            }

            set_loading.set(false);
        });
    });

    // Convert allocations to timeline groups and items
    let (timeline_groups, set_timeline_groups) = create_signal(Vec::<TimelineGroup>::new());
    let (timeline_items, set_timeline_items) = create_signal(Vec::<TimelineItem>::new());

    create_effect(move |_| {
        let all_allocations = allocations.get();

        // Create groups from unique resources
        let mut resource_map = std::collections::HashMap::new();
        for allocation in &all_allocations {
            resource_map
                .entry(allocation.resource_id.clone())
                .or_insert_with(|| (allocation.resource_name.clone(), 0.0));
        }

        // Calculate total allocation percentage per resource
        for allocation in &all_allocations {
            if let Some((_, total)) = resource_map.get_mut(&allocation.resource_id) {
                *total += allocation.allocation_percentage;
            }
        }

        // Create timeline groups - no background color on rows
        // Sort by resource name for stable ordering
        let mut groups: Vec<TimelineGroup> = resource_map
            .into_iter()
            .map(
                |(resource_id, (resource_name, _total_percentage))| TimelineGroup {
                    id: resource_id.to_string(),
                    content: resource_name,
                    class_name: None, // No background class on group rows
                    style: None,
                },
            )
            .collect();
        groups.sort_by(|a, b| a.content.cmp(&b.content));

        // Create timeline items from allocations
        // Assign consistent colors to projects
        let mut project_colors: std::collections::HashMap<
            String,
            (String, String, String, String),
        > = std::collections::HashMap::new();
        let color_palette: [(String, String, String, String); 25] = [
            (
                "#3b82f6".to_string(),
                "bg-blue-500".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 1. Blue
            (
                "#ef4444".to_string(),
                "bg-red-500".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 2. Red
            (
                "#22c55e".to_string(),
                "bg-green-500".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 3. Green
            (
                "#eab308".to_string(),
                "bg-yellow-500".to_string(),
                "text-slate-900".to_string(),
                "#0f172a".to_string(),
            ), // 4. Yellow
            (
                "#a855f7".to_string(),
                "bg-purple-500".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 5. Purple
            (
                "#f97316".to_string(),
                "bg-orange-500".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 6. Orange
            (
                "#06b6d4".to_string(),
                "bg-cyan-500".to_string(),
                "text-slate-900".to_string(),
                "#0f172a".to_string(),
            ), // 7. Cyan
            (
                "#ec4899".to_string(),
                "bg-pink-500".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 8. Pink
            (
                "#6366f1".to_string(),
                "bg-indigo-500".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 9. Indigo
            (
                "#14b8a6".to_string(),
                "bg-teal-500".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 10. Teal
            (
                "#84cc16".to_string(),
                "bg-lime-500".to_string(),
                "text-slate-900".to_string(),
                "#0f172a".to_string(),
            ), // 11. Lime
            (
                "#f43f5e".to_string(),
                "bg-rose-500".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 12. Rose
            (
                "#8b5cf6".to_string(),
                "bg-violet-500".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 13. Violet
            (
                "#0ea5e9".to_string(),
                "bg-sky-500".to_string(),
                "text-slate-900".to_string(),
                "#0f172a".to_string(),
            ), // 14. Sky
            (
                "#10b981".to_string(),
                "bg-emerald-500".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 15. Emerald
            (
                "#f59e0b".to_string(),
                "bg-amber-500".to_string(),
                "text-slate-900".to_string(),
                "#0f172a".to_string(),
            ), // 16. Amber
            (
                "#d946ef".to_string(),
                "bg-fuchsia-500".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 17. Fuchsia
            (
                "#64748b".to_string(),
                "bg-slate-500".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 18. Slate
            (
                "#71717a".to_string(),
                "bg-zinc-500".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 19. Zinc
            (
                "#dc2626".to_string(),
                "bg-red-600".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 20. Dark Red
            (
                "#2563eb".to_string(),
                "bg-blue-600".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 21. Dark Blue
            (
                "#16a34a".to_string(),
                "bg-green-600".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 22. Dark Green
            (
                "#9333ea".to_string(),
                "bg-purple-600".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 23. Dark Purple
            (
                "#c2410c".to_string(),
                "bg-orange-600".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 24. Dark Orange
            (
                "#0891b2".to_string(),
                "bg-cyan-600".to_string(),
                "text-white".to_string(),
                "#ffffff".to_string(),
            ), // 25. Dark Cyan
        ];
        let mut color_index = 0;
        let mut items: Vec<TimelineItem> = Vec::new();

        for a in all_allocations {
            // Get or assign color for this project
            let (color, bg_class, text_class, text_color) = project_colors
                .entry(a.project_name.clone())
                .or_insert_with(|| {
                    let idx = color_index % color_palette.len();
                    color_index += 1;
                    color_palette[idx].clone()
                })
                .clone();
            let color_class = format!("{} {}", bg_class, text_class);
            let weekend_badge = if a.include_weekend {
                " <span class='ml-2 inline-flex items-center rounded bg-white/80 px-1.5 py-0.5 text-[10px] font-semibold text-slate-900'>Weekend</span>"
            } else {
                ""
            };

            if a.include_weekend {
                // Continuous allocation from start to end
                // Add one day to end date to make it inclusive
                let end_date_inclusive = if let Ok(end_date) =
                    chrono::NaiveDate::parse_from_str(&a.end_date, "%Y-%m-%d")
                {
                    (end_date + chrono::Duration::days(1))
                        .format("%Y-%m-%d")
                        .to_string()
                } else {
                    a.end_date.clone()
                };

                items.push(TimelineItem {
                    id: a.id.to_string(),
                    group: Some(a.resource_id.to_string()),
                    content: format!(
                        "<div class='allocation-item {}'>{} ({:.0}%){} </div>",
                        color_class, a.project_name, a.allocation_percentage, weekend_badge
                    ),
                    start: a.start_date.clone(),
                    end: Some(end_date_inclusive),
                    class_name: Some(color_class.clone()),
                    style: Some(format!(
                        "background-color: {}; border-color: {}; color: {}",
                        color, color, text_color
                    )),
                    editable: Some(true),
                    item_type: None,
                });
            } else {
                // Split allocation into working days only (excluding weekends and holidays)
                if let (Ok(start_date), Ok(end_date)) = (
                    chrono::NaiveDate::parse_from_str(&a.start_date, "%Y-%m-%d"),
                    chrono::NaiveDate::parse_from_str(&a.end_date, "%Y-%m-%d"),
                ) {
                    // Get holiday dates as a set for O(1) lookup
                    let holiday_dates: std::collections::HashSet<String> =
                        holidays.get().iter().map(|h| h.date.clone()).collect();

                    let mut current_start: Option<chrono::NaiveDate> = None;
                    let mut current_end: Option<chrono::NaiveDate> = None;

                    let mut current = start_date;
                    while current <= end_date {
                        let weekday = current.weekday();
                        let is_weekend =
                            weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun;
                        let current_date_str = current.format("%Y-%m-%d").to_string();
                        let is_holiday = holiday_dates.contains(&current_date_str);
                        let is_working_day = !is_weekend && !is_holiday;

                        if is_working_day {
                            if current_start.is_none() {
                                current_start = Some(current);
                            }
                            current_end = Some(current);
                        } else {
                            // End of a working period, create item
                            if let (Some(start), Some(end)) = (current_start, current_end) {
                                // Add one day to make end date inclusive
                                let end_inclusive = (end + chrono::Duration::days(1))
                                    .format("%Y-%m-%d")
                                    .to_string();
                                items.push(TimelineItem {
                                    id: format!("{}-{}", a.id, items.len()),
                                    group: Some(a.resource_id.to_string()),
                                    content: format!(
                                        "<div class='allocation-item {}'>{} ({:.0}%){} </div>",
                                        color_class,
                                        a.project_name,
                                        a.allocation_percentage,
                                        weekend_badge
                                    ),
                                    start: start.format("%Y-%m-%d").to_string(),
                                    end: Some(end_inclusive),
                                    class_name: Some(color_class.clone()),
                                    style: Some(format!(
                                        "background-color: {}; border-color: {}; color: {}",
                                        color, color, text_color
                                    )),
                                    editable: Some(true),
                                    item_type: None,
                                });
                            }
                            current_start = None;
                            current_end = None;
                        }

                        current = current + chrono::Duration::days(1);
                    }

                    // Create final item if there's an ongoing working period
                    if let (Some(start), Some(end)) = (current_start, current_end) {
                        // Add one day to make end date inclusive
                        let end_inclusive = (end + chrono::Duration::days(1))
                            .format("%Y-%m-%d")
                            .to_string();
                        items.push(TimelineItem {
                            id: format!("{}-{}", a.id, items.len()),
                            group: Some(a.resource_id.to_string()),
                            content: format!(
                                "<div class='allocation-item {}'>{} ({:.0}%){} </div>",
                                color_class, a.project_name, a.allocation_percentage, weekend_badge
                            ),
                            start: start.format("%Y-%m-%d").to_string(),
                            end: Some(end_inclusive),
                            class_name: Some(color_class.clone()),
                            style: Some(format!(
                                "background-color: {}; border-color: {}; color: {}",
                                color, color, text_color
                            )),
                            editable: Some(true),
                            item_type: None,
                        });
                    }
                }
            }
        }

        // Add holiday background items
        for holiday in holidays.get() {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&holiday.date, "%Y-%m-%d") {
                let end_inclusive = (date + chrono::Duration::days(1))
                    .format("%Y-%m-%d")
                    .to_string();
                items.push(TimelineItem {
                    id: format!("holiday-{}", holiday.id),
                    group: None,
                    content: String::new(),
                    start: holiday.date.clone(),
                    end: Some(end_inclusive),
                    class_name: Some("holiday-bg".to_string()),
                    style: None,
                    editable: Some(false),
                    item_type: Some("background".to_string()),
                });
            }
        }

        set_timeline_groups.set(groups);
        set_timeline_items.set(items);
    });

    // Handle form submission
    let handle_submit = move |form_data: AllocationFormData| {
        let editing_id = editing_allocation.get().map(|a| a.id);
        spawn_local(async move {
            set_form_submitting.set(true);
            set_error.set(None);

            let result = if let Some(allocation_id) = editing_id {
                update_allocation_form(allocation_id.to_string(), form_data).await
            } else {
                create_allocation(form_data).await
            };

            match result {
                Ok(_) => {
                    // Reload allocations
                    match fetch_allocations().await {
                        Ok(data) => {
                            set_allocations.set(data);
                            set_show_form.set(false);
                            set_editing_allocation.set(None);
                        }
                        Err(e) => set_error.set(Some(e)),
                    }
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_form_submitting.set(false);
        });
    };

    let handle_cancel = move |_| {
        set_show_form.set(false);
        set_editing_allocation.set(None);
    };

    // Convert to option types for form
    let resource_options = create_memo(move |_| {
        resources
            .get()
            .into_iter()
            .map(|r| ResourceOption {
                id: r.id,
                name: r.name,
            })
            .collect::<Vec<_>>()
    });

    let project_options = create_memo(move |_| {
        projects
            .get()
            .into_iter()
            .map(|p| ProjectOption {
                id: p.id,
                name: p.name,
            })
            .collect::<Vec<_>>()
    });

    let holiday_axis_css = create_memo(move |_| {
        let month_names = [
            "january",
            "february",
            "march",
            "april",
            "may",
            "june",
            "july",
            "august",
            "september",
            "october",
            "november",
            "december",
        ];
        let mut rules = String::new();
        for holiday in holidays.get() {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&holiday.date, "%Y-%m-%d") {
                let month_idx = (date.month0() as usize).min(11);
                let month_class = month_names[month_idx];
                let day_class = format!("vis-day{}", date.day());
                rules.push_str(&format!(
                    ".vis-time-axis .vis-text.vis-minor.{}.vis-{} {{ background-color: rgba(156, 163, 175, 0.35) !important; }}\n",
                    day_class, month_class
                ));
            }
        }
        rules
    });

    let editing_form_data = Signal::derive(move || {
        editing_allocation.get().map(|a| AllocationEditData {
            id: a.id.to_string(),
            resource_id: a.resource_id.to_string(),
            project_id: a.project_id.to_string(),
            start_date: a.start_date.clone(),
            end_date: a.end_date.clone(),
            allocation_percentage: a.allocation_percentage,
            include_weekend: a.include_weekend,
        })
    });

    view! {
        <div class="min-h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
            <Header/>

            <main class="flex-grow max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12 w-full">
                <div class="space-y-6">
                    <div class="flex items-center justify-between">
                        <div>
                            <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                                "Resource Allocations"
                            </h1>
                            <p class="text-gray-600 dark:text-gray-400 mt-1">
                                "Visual timeline of resource assignments to projects"
                            </p>
                        </div>

                        <div class="flex items-center space-x-3">
                            <button
                                class="btn-primary"
                                on:click=move |_| set_show_form.set(true)
                            >
                                "Add Allocation"
                            </button>
                        </div>
                    </div>

                    {move || error.get().map(|err| {
                        view! {
                            <div class="rounded-md bg-red-50 p-4 dark:bg-red-900/20">
                                <div class="flex">
                                    <div class="ml-3">
                                        <h3 class="text-sm font-medium text-red-800 dark:text-red-200">
                                            {err}
                                        </h3>
                                    </div>
                                </div>
                            </div>
                        }
                    })}

                    {move || {
                        if show_form.get() {
                            let is_edit = editing_allocation.get().is_some();
                            let title = if is_edit { "Edit Allocation" } else { "Create Allocation" };
                            view! {
                                <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 relative">
                                    <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">
                                        {title}
                                    </h2>
                                    <AllocationForm
                                        resources=resource_options.into()
                                        projects=project_options.into()
                                        editing_allocation=editing_form_data
                                        on_submit=Callback::new(handle_submit)
                                        on_cancel=Callback::new(handle_cancel)
                                        is_submitting=form_submitting.get()
                                    />
                                    {move || {
                                        if form_submitting.get() {
                                            view! {
                                                <div class="absolute inset-0 flex items-center justify-center bg-white/70 dark:bg-gray-800/70 rounded-lg">
                                                    <div class="text-center">
                                                        <div class="spinner mx-auto mb-2"></div>
                                                        <p class="text-sm text-gray-600 dark:text-gray-400">"Saving..."</p>
                                                    </div>
                                                </div>
                                            }.into_view()
                                        } else {
                                            view! { <div></div> }.into_view()
                                        }
                                    }}
                                </div>
                            }.into_view()
                        } else {
                            view! { <div></div> }.into_view()
                        }
                    }}

                    <div class="space-y-6">
                        <div class="bg-white dark:bg-gray-800 shadow rounded-lg overflow-hidden">
                            <div class="h-[500px] relative">
                                <style>{move || holiday_axis_css.get()}</style>
                                <TimelineChart
                                    groups=timeline_groups.into()
                                    items=timeline_items.into()
                                    days_before=15
                                    days_after=15
                                    holidays={holidays.get().iter().map(|h| h.date.clone()).collect::<Vec<_>>()}
                                    on_item_move=Callback::new(move |(item_id, new_start, new_end): (String, String, String)| {
                                        web_sys::console::log_1(&format!("Drag callback triggered: id={}, start={}, end={}", item_id, new_start, new_end).into());
                                        let base_id = item_id.split('-').take(5).collect::<Vec<_>>().join("-");
                                        let previous_allocations = allocations.get();

                                        // Optimistically update allocations to avoid flicker
                                        set_allocations.update(|allocs| {
                                            for allocation in allocs.iter_mut() {
                                                if allocation.id.to_string() == base_id {
                                                    allocation.start_date = new_start.clone();
                                                    allocation.end_date = new_end.clone();
                                                }
                                            }
                                        });

                                        spawn_local(async move {
                                            set_error.set(None);

                                            web_sys::console::log_1(&"Calling update_allocation...".into());
                                            match update_allocation(item_id, new_start, new_end).await {
                                                Ok(_) => {
                                                    web_sys::console::log_1(&"Update successful".into());
                                                }
                                                Err(e) => {
                                                    web_sys::console::log_1(&format!("Update failed: {}", e).into());
                                                    set_error.set(Some(e));
                                                    // Revert optimistic update
                                                    set_allocations.set(previous_allocations);
                                                }
                                            }
                                        });
                                    })
                                />

                                {move || {
                                    if loading.get() {
                                        view! {
                                            <div class="absolute inset-0 flex items-center justify-center bg-white/70 dark:bg-gray-800/70">
                                                <div class="text-center">
                                                    <div class="spinner mx-auto mb-4"></div>
                                                    <p class="text-gray-600 dark:text-gray-400">"Loading allocations..."</p>
                                                </div>
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! { <div></div> }.into_view()
                                    }
                                }}
                            </div>
                        </div>

                        {move || {
                            if allocations.get().is_empty() && !loading.get() {
                                view! {
                                    <div class="text-center py-6 bg-white dark:bg-gray-800 rounded-lg shadow">
                                        <p class="text-gray-600 dark:text-gray-400">"No allocations found."</p>
                                        <p class="text-sm text-gray-500 dark:text-gray-500 mt-2">"Click 'Add Allocation' to create one."</p>
                                    </div>
                                }.into_view()
                            } else {
                                view! { <div></div> }.into_view()
                            }
                        }}

                        {move || {
                            if allocations.get().is_empty() {
                                view! { <div></div> }.into_view()
                            } else {
                                view! {
                                    <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6">
                                        <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                                            "Allocation Details"
                                        </h2>
                                        <div class="overflow-x-auto">
                                            <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                                <thead class="bg-gray-50 dark:bg-gray-700">
                                                    <tr>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Resource"</th>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Project"</th>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Start Date"</th>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"End Date"</th>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Allocation"</th>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Actions"</th>
                                                    </tr>
                                                </thead>
                                                <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                                                    {move || allocations.get().into_iter().map(|allocation| {
                                                        let allocation_id = allocation.id.to_string();
                                                        let alloc_for_edit = allocation.clone();
                                                        view! {
                                                            <tr class="hover:bg-gray-50 dark:hover:bg-gray-700">
                                                                <td class="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900 dark:text-white">
                                                                    {allocation.resource_name.clone()}
                                                                </td>
                                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                                    {allocation.project_name.clone()}
                                                                </td>
                                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                                    {allocation.start_date.clone()}
                                                                </td>
                                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                                    {allocation.end_date.clone()}</td>
                                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                                    {format!("{:.0}%", allocation.allocation_percentage)}
                                                                </td>
                                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                                    <div class="flex items-center space-x-2">
                                                                        <button
                                                                            class="text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
                                                                            on:click={
                                                                                let alloc = alloc_for_edit.clone();
                                                                                move |_| {
                                                                                    set_editing_allocation.set(Some(alloc.clone()));
                                                                                    set_show_form.set(true);
                                                                                }
                                                                            }
                                                                        >
                                                                            "Edit"
                                                                        </button>
                                                                        {move || {
                                                                            let is_deleting = deleting_id.get() == Some(allocation_id.clone());
                                                                            view! {
                                                                                <button
                                                                                    class="text-red-600 hover:text-red-800 dark:text-red-400 dark:hover:text-red-300 disabled:opacity-50 disabled:cursor-not-allowed"
                                                                                    disabled=is_deleting
                                                                                    on:click={
                                                                                        let id = allocation_id.clone();
                                                                                        move |_| {
                                                                                            let id_clone = id.clone();
                                                                                            set_deleting_id.set(Some(id_clone.clone()));
                                                                                            spawn_local(async move {
                                                                                                set_error.set(None);

                                                                                                match delete_allocation(id_clone).await {
                                                                                                    Ok(_) => {
                                                                                                        // Reload allocations
                                                                                                        match fetch_allocations().await {
                                                                                                            Ok(data) => set_allocations.set(data),
                                                                                                            Err(e) => set_error.set(Some(e)),
                                                                                                        }
                                                                                                    }
                                                                                                    Err(e) => set_error.set(Some(e)),
                                                                                                }
                                                                                                set_deleting_id.set(None);
                                                                                            });
                                                                                        }
                                                                                    }
                                                                                >
                                                                                    {if is_deleting { "Deleting..." } else { "Delete" }}
                                                                                </button>
                                                                            }
                                                                        }}
                                                                    </div>
                                                                </td>
                                                            </tr>
                                                        }
                                                    }).collect_view()}
                                                </tbody>
                                            </table>
                                        </div>
                                    </div>
                                }.into_view()
                            }
                        }}
                    </div>
                </div>
            </main>

            <Footer/>
        </div>
    }
}

/// Fetch all allocations from API
async fn fetch_allocations() -> Result<Vec<Allocation>, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/allocations")
        .await
        .map_err(|e| format!("Failed to fetch allocations: {}", e))?;

    if response.status().is_success() {
        response
            .json::<Vec<Allocation>>()
            .await
            .map_err(|e| format!("Failed to parse allocations: {}", e))
    } else {
        Err(format!(
            "Failed to fetch allocations: {}",
            response.status()
        ))
    }
}

/// Fetch all resources from API
async fn fetch_resources() -> Result<Vec<Resource>, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/resources")
        .await
        .map_err(|e| format!("Failed to fetch resources: {}", e))?;

    if response.status().is_success() {
        // Parse as generic JSON and extract id and name
        let json_data: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse resources: {}", e))?;

        let resources: Vec<Resource> = json_data
            .into_iter()
            .filter_map(|v| {
                Some(Resource {
                    id: v.get("id")?.as_str()?.parse().ok()?,
                    name: v.get("name")?.as_str()?.to_string(),
                })
            })
            .collect();

        Ok(resources)
    } else {
        Err(format!("Failed to fetch resources: {}", response.status()))
    }
}

/// Fetch all projects from API
async fn fetch_projects() -> Result<Vec<Project>, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/projects")
        .await
        .map_err(|e| format!("Failed to fetch projects: {}", e))?;

    if response.status().is_success() {
        // Parse as generic JSON and extract id and name
        let json_data: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse projects: {}", e))?;

        let projects: Vec<Project> = json_data
            .into_iter()
            .filter_map(|v| {
                Some(Project {
                    id: v.get("id")?.as_str()?.parse().ok()?,
                    name: v.get("name")?.as_str()?.to_string(),
                })
            })
            .collect();

        Ok(projects)
    } else {
        Err(format!("Failed to fetch projects: {}", response.status()))
    }
}

/// Create a new allocation
async fn create_allocation(form_data: AllocationFormData) -> Result<(), String> {
    let resource_id = form_data
        .resource_id
        .parse::<Uuid>()
        .map_err(|_| "Invalid resource ID")?;
    let project_id = form_data
        .project_id
        .parse::<Uuid>()
        .map_err(|_| "Invalid project ID")?;
    let allocation_percentage = form_data
        .allocation_percentage
        .parse::<f64>()
        .map_err(|_| "Invalid allocation percentage")?;

    let response = authenticated_post_json(
        "http://localhost:3000/api/v1/allocations",
        &serde_json::json!({
            "resource_id": resource_id,
            "project_id": project_id,
            "start_date": form_data.start_date,
            "end_date": form_data.end_date,
            "allocation_percentage": allocation_percentage,
            "include_weekend": form_data.include_weekend,
        }),
    )
        .await
        .map_err(|e| format!("Failed to create allocation: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to create allocation: {}", error_text))
    }
}

/// Update an existing allocation (form edit)
async fn update_allocation_form(
    allocation_id: String,
    form_data: AllocationFormData,
) -> Result<(), String> {
    let resource_id = form_data
        .resource_id
        .parse::<Uuid>()
        .map_err(|_| "Invalid resource ID")?;
    let project_id = form_data
        .project_id
        .parse::<Uuid>()
        .map_err(|_| "Invalid project ID")?;
    let allocation_percentage = form_data
        .allocation_percentage
        .parse::<f64>()
        .map_err(|_| "Invalid allocation percentage")?;

    let response = authenticated_put_json(
        &format!(
            "http://localhost:3000/api/v1/allocations/{}",
            allocation_id
        ),
        &serde_json::json!({
            "resource_id": resource_id,
            "project_id": project_id,
            "start_date": form_data.start_date,
            "end_date": form_data.end_date,
            "allocation_percentage": allocation_percentage,
            "include_weekend": form_data.include_weekend,
        }),
    )
        .await
        .map_err(|e| format!("Failed to update allocation: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to update allocation: {}", error_text))
    }
}

/// Update an allocation
async fn update_allocation(
    allocation_id: String,
    start_date: String,
    end_date: String,
) -> Result<(), String> {
    // Extract the base UUID from the item ID (handles "{uuid}-{index}" format)
    let base_id = allocation_id
        .split('-')
        .take(5)
        .collect::<Vec<_>>()
        .join("-");
    let id = base_id
        .parse::<Uuid>()
        .map_err(|_| "Invalid allocation ID")?;

    let response = authenticated_put_json(
        &format!("http://localhost:3000/api/v1/allocations/{}", id),
        &serde_json::json!({
            "start_date": start_date,
            "end_date": end_date,
        }),
    )
        .await
        .map_err(|e| format!("Failed to update allocation: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to update allocation: {}", error_text))
    }
}

/// Delete an allocation
async fn delete_allocation(allocation_id: String) -> Result<(), String> {
    let id = allocation_id
        .parse::<Uuid>()
        .map_err(|_| "Invalid allocation ID")?;

    let response = authenticated_delete(&format!("http://localhost:3000/api/v1/allocations/{}", id))
        .await
        .map_err(|e| format!("Failed to delete allocation: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to delete allocation: {}", error_text))
    }
}

/// Fetch all holidays from API
async fn fetch_holidays() -> Result<Vec<Holiday>, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/holidays")
        .await
        .map_err(|e| format!("Failed to fetch holidays: {}", e))?;

    if response.status().is_success() {
        response
            .json::<Vec<Holiday>>()
            .await
            .map_err(|e| format!("Failed to parse holidays: {}", e))
    } else {
        Err(format!("Failed to fetch holidays: {}", response.status()))
    }
}
