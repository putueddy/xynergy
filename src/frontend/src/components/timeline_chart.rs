use crate::timeline::{
    create_timeline_options, groups_to_js_array, items_to_js_array, Timeline, TimelineGroup,
    TimelineItem,
};
use leptos::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

/// Timeline Chart component using Vis-timeline
#[component]
pub fn TimelineChart(
    groups: Signal<Vec<TimelineGroup>>,
    items: Signal<Vec<TimelineItem>>,
    #[prop(default = "2026-01-01")] start_date: &'static str,
    #[prop(default = "2026-03-31")] end_date: &'static str,
) -> impl IntoView {
    let timeline_ref = create_node_ref::<leptos::html::Div>();
    let (timeline_instance, set_timeline_instance) = create_signal(Option::<Timeline>::None);

    // Initialize timeline when component mounts
    create_effect(move |_| {
        if let Some(container) = timeline_ref.get() {
            let groups_data = groups.get();
            let items_data = items.get();

            if !groups_data.is_empty() {
                let js_groups = groups_to_js_array(&groups_data);
                let js_items = items_to_js_array(&items_data);
                let options = create_timeline_options(start_date, end_date, false, true);

                if let Some(html_element) = container.dyn_ref::<web_sys::HtmlElement>() {
                    let timeline = Timeline::new(
                        html_element,
                        &JsValue::from(js_items),
                        &JsValue::from(js_groups),
                        &JsValue::from(options),
                    );

                    set_timeline_instance.set(Some(timeline));
                    web_sys::console::log_1(&"Timeline initialized with Vis-timeline".into());
                }
            }
        }
    });

    // Update timeline when data changes
    create_effect(move |_| {
        let items_data = items.get();
        let groups_data = groups.get();

        if let Some(timeline) = timeline_instance.get() {
            if !groups_data.is_empty() {
                let js_items = items_to_js_array(&items_data);
                let js_groups = groups_to_js_array(&groups_data);

                timeline.set_items(&JsValue::from(js_items));
                timeline.set_groups(&JsValue::from(js_groups));
                timeline.redraw();
            }
        }
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
            group: self.resource_id.clone(),
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
