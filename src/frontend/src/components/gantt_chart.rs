use crate::gantt::{create_gantt_options, tasks_to_js_array, FrappeGantt, GanttTask};
use leptos::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

/// Gantt Chart component
#[component]
pub fn GanttChart(
    tasks: Signal<Vec<GanttTask>>,
    #[prop(default = "Week")] view_mode: &'static str,
) -> impl IntoView {
    let gantt_ref = create_node_ref::<leptos::html::Div>();

    // Initialize Gantt chart when component mounts
    create_effect(move |_| {
        if let Some(gantt_div) = gantt_ref.get() {
            let task_list = tasks.get();

            if !task_list.is_empty() {
                let js_tasks = tasks_to_js_array(&task_list);
                let options = create_gantt_options(view_mode, "YYYY-MM-DD", "en");

                // Convert Div to HtmlElement using dyn_ref
                if let Some(html_element) = gantt_div.dyn_ref::<web_sys::HtmlElement>() {
                    let _gantt = FrappeGantt::new(
                        html_element,
                        &JsValue::from(js_tasks),
                        &JsValue::from(options),
                    );

                    web_sys::console::log_1(&"Gantt chart initialized".into());
                }
            }
        }
    });

    view! {
        <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 overflow-x-auto">
            <div _ref=gantt_ref class="gantt-container min-w-full" style="height: 400px;">
                {move || {
                    if tasks.get().is_empty() {
                        view! {
                            <div class="flex items-center justify-center h-full text-gray-500 dark:text-gray-400">
                                "No tasks to display"
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

/// Gantt task item component for list view
#[component]
pub fn GanttTaskItem(task: GanttTask) -> impl IntoView {
    view! {
        <div class="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-700 rounded mb-2">
            <div class="flex items-center space-x-3">
                <div class="w-3 h-3 rounded-full bg-blue-500"></div>
                <div>
                    <div class="font-medium text-gray-900 dark:text-white">{task.name.clone()}</div>
                    <div class="text-sm text-gray-500 dark:text-gray-400">
                        {format!("{} - {}", task.start, task.end)}
                    </div>
                </div>
            </div>
            <div class="text-sm text-gray-600 dark:text-gray-300">
                {format!("{}% complete", task.progress)}
            </div>
        </div>
    }
}
