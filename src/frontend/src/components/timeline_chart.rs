use crate::timeline::{
    create_holiday_background_items, create_timeline_options, groups_to_js_array,
    items_to_js_array, Timeline, TimelineGroup, TimelineItem,
};
use js_sys::{Function, Reflect};
use leptos::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

/// Timeline Chart component using Vis-timeline
#[component]
pub fn TimelineChart(
    groups: Signal<Vec<TimelineGroup>>,
    items: Signal<Vec<TimelineItem>>,
    #[prop(default = 15)] days_before: i64,
    #[prop(default = 15)] days_after: i64,
    #[prop(default = Vec::new())] holidays: Vec<String>,
    #[prop(optional)] on_item_move: Option<Callback<(String, String, String)>>, // (item_id, new_start, new_end)
) -> impl IntoView {
    let timeline_ref = create_node_ref::<leptos::html::Div>();
    // Use StoredValue instead of create_signal because Timeline doesn't implement Clone
    let timeline_instance = store_value::<Option<Timeline>>(None);

    // Calculate date range centered around today
    let (start_date, end_date) = {
        let today = chrono::Local::now().date_naive();
        let start = today - chrono::Duration::days(days_before);
        let end = today + chrono::Duration::days(days_after);
        (
            start.format("%Y-%m-%d").to_string(),
            end.format("%Y-%m-%d").to_string(),
        )
    };

    // Initialize timeline when component mounts
    create_effect(move |_| {
        if let Some(container) = timeline_ref.get() {
            let groups_data = groups.get();
            let items_data = items.get();

            if !groups_data.is_empty() {
                let should_create =
                    timeline_instance.with_value(|timeline_opt| timeline_opt.is_none());
                if !should_create {
                    return;
                }

                let js_groups = groups_to_js_array(&groups_data);
                let mut js_items = items_to_js_array(&items_data);

                // Add holiday background items
                let holiday_items = create_holiday_background_items(&holidays);
                for i in 0..holiday_items.length() {
                    let item = holiday_items.get(i);
                    if !item.is_undefined() && !item.is_null() {
                        js_items.push(&item);
                    }
                }

                let options =
                    create_timeline_options(&start_date, &end_date, true, true, &holidays);

                if let Some(html_element) = container.dyn_ref::<web_sys::HtmlElement>() {
                    // Set up onMove handler in options for item drag events
                    if let Some(callback) = on_item_move {
                        let callback_clone = callback.clone();
                        let move_handler =
                            Closure::wrap(Box::new(move |item: JsValue, handler: Function| {
                                web_sys::console::log_1(&"MOVE EVENT TRIGGERED!".into());
                                if let Ok(props) = item.clone().dyn_into::<js_sys::Object>() {
                                    let js_value_to_date = |value: JsValue| -> String {
                                        if let Some(text) = value.as_string() {
                                            return text;
                                        }
                                        if value.is_instance_of::<js_sys::Date>() {
                                            let date = js_sys::Date::from(value);
                                            let year = date.get_full_year();
                                            let month = date.get_month() + 1;
                                            let day = date.get_date();
                                            return format!(
                                                "{:04}-{:02}-{:02}",
                                                year as i32, month, day
                                            );
                                        }
                                        String::new()
                                    };

                                    let id = Reflect::get(&props, &"id".into())
                                        .ok()
                                        .and_then(|v| v.as_string())
                                        .unwrap_or_default();
                                    let start = Reflect::get(&props, &"start".into())
                                        .ok()
                                        .map(js_value_to_date)
                                        .unwrap_or_default();
                                    let mut end = Reflect::get(&props, &"end".into())
                                        .ok()
                                        .map(js_value_to_date)
                                        .unwrap_or_default();

                                    if end.is_empty() {
                                        end = start.clone();
                                    }

                                    web_sys::console::log_1(
                                        &format!("Item moved: {} from {} to {}", id, start, end)
                                            .into(),
                                    );

                                    // Show alert for debugging
                                    web_sys::window()
                                        .unwrap()
                                        .alert_with_message(&format!(
                                            "Dragged: {} to {} - {}",
                                            id, start, end
                                        ))
                                        .unwrap();

                                    callback_clone.call((id, start, end));
                                } else {
                                    web_sys::console::log_1(
                                        &"Failed to convert item to object".into(),
                                    );
                                }

                                // Confirm the move with vis-timeline
                                let _ = handler.call1(&JsValue::NULL, &item);
                            })
                                as Box<dyn FnMut(JsValue, Function)>);

                        Reflect::set(
                            &options,
                            &"onMove".into(),
                            move_handler.as_ref().unchecked_ref(),
                        )
                        .unwrap();
                        web_sys::console::log_1(&"onMove handler registered".into());
                        move_handler.forget(); // Keep the closure alive
                    } else {
                        web_sys::console::log_1(&"No on_item_move callback provided".into());
                    }

                    let timeline = Timeline::new(
                        html_element,
                        &JsValue::from(js_items),
                        &JsValue::from(js_groups),
                        &JsValue::from(options),
                    );

                    timeline_instance.set_value(Some(timeline));
                    web_sys::console::log_1(
                        &"Timeline initialized with Vis-timeline (editable)".into(),
                    );
                }
            }
        }
    });

    // Update timeline when data changes
    create_effect(move |_| {
        let items_data = items.get();
        let groups_data = groups.get();

        timeline_instance.with_value(|timeline_opt| {
            if let Some(timeline) = timeline_opt {
                if !groups_data.is_empty() {
                    let js_items = items_to_js_array(&items_data);
                    let js_groups = groups_to_js_array(&groups_data);

                    timeline.set_items(&JsValue::from(js_items));
                    timeline.set_groups(&JsValue::from(js_groups));
                    timeline.redraw();
                }
            }
        });
    });

    view! {
        <div class="timeline-container w-full h-full min-h-[400px]">
            <div _ref=timeline_ref class="vis-timeline-wrapper w-full h-full">
                {move || {
                    if groups.get().is_empty() {
                        view! {
                            <div class="flex items-center justify-center h-full text-gray-500 dark:text-gray-400">
                                "No resources to display"
                            </div>
                        }.into_view()
                    } else {
                        view! { <div></div> }.into_view()
                    }
                }}
            </div>
        </div>
    }
}

/// Resource group for timeline
#[derive(Debug, Clone)]
pub struct ResourceGroup {
    pub id: String,
    pub name: String,
    pub allocation_percentage: f64,
}

impl ResourceGroup {
    pub fn to_timeline_group(&self) -> TimelineGroup {
        let allocation_class = if self.allocation_percentage >= 100.0 {
            "allocation-full"
        } else if self.allocation_percentage >= 80.0 {
            "allocation-high"
        } else {
            "allocation-normal"
        };

        TimelineGroup {
            id: self.id.clone(),
            content: format!(
                "<div class='resource-label'>{} <span class='allocation-badge {}'>{:.0}%</span></div>",
                self.name, allocation_class, self.allocation_percentage
            ),
            class_name: Some(allocation_class.to_string()),
            style: None,
        }
    }
}

/// Allocation item for timeline
#[derive(Debug, Clone)]
pub struct AllocationItem {
    pub id: String,
    pub resource_id: String,
    pub project_name: String,
    pub start: String,
    pub end: String,
    pub percentage: f64,
}

impl AllocationItem {
    pub fn to_timeline_item(&self) -> TimelineItem {
        let color_class = if self.percentage >= 100.0 {
            "bg-red-500"
        } else if self.percentage >= 80.0 {
            "bg-yellow-500"
        } else {
            "bg-blue-500"
        };

        TimelineItem {
            id: self.id.clone(),
            group: Some(self.resource_id.clone()),
            content: format!(
                "<div class='allocation-item {}'>{} ({:.0}%)</div>",
                color_class, self.project_name, self.percentage
            ),
            start: self.start.clone(),
            end: Some(self.end.clone()),
            class_name: Some(color_class.to_string()),
            style: Some(format!(
                "background-color: {}; border-color: {}",
                self.get_color(),
                self.get_color()
            )),
            editable: Some(true),
            item_type: None,
        }
    }

    fn get_color(&self) -> String {
        if self.percentage >= 100.0 {
            "#ef4444".to_string() // red-500
        } else if self.percentage >= 80.0 {
            "#eab308".to_string() // yellow-500
        } else {
            "#3b82f6".to_string() // blue-500
        }
    }
}
