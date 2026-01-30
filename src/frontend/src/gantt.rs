use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;

/// Gantt task structure
#[derive(Clone, Debug, PartialEq)]
pub struct GanttTask {
    pub id: String,
    pub name: String,
    pub start: String,
    pub end: String,
    pub progress: f64,
    pub custom_class: Option<String>,
}

impl GanttTask {
    pub fn to_js_object(&self) -> Object {
        let obj = Object::new();
        Reflect::set(&obj, &"id".into(), &self.id.clone().into()).unwrap();
        Reflect::set(&obj, &"name".into(), &self.name.clone().into()).unwrap();
        Reflect::set(&obj, &"start".into(), &self.start.clone().into()).unwrap();
        Reflect::set(&obj, &"end".into(), &self.end.clone().into()).unwrap();
        Reflect::set(&obj, &"progress".into(), &self.progress.into()).unwrap();
        if let Some(class) = &self.custom_class {
            Reflect::set(&obj, &"custom_class".into(), &class.clone().into()).unwrap();
        }
        obj
    }
}

/// Initialize Frappe Gantt
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "Gantt")]
    pub type FrappeGantt;

    #[wasm_bindgen(constructor, js_class = "Gantt")]
    pub fn new(element: &web_sys::HtmlElement, tasks: &JsValue, options: &JsValue) -> FrappeGantt;

    #[wasm_bindgen(method, js_class = "Gantt")]
    pub fn change_view_mode(this: &FrappeGantt, mode: &str);

    #[wasm_bindgen(method, js_class = "Gantt")]
    pub fn refresh(this: &FrappeGantt, tasks: &JsValue);
}

/// Create Gantt options
pub fn create_gantt_options(view_mode: &str, date_format: &str, language: &str) -> Object {
    let options = Object::new();
    Reflect::set(&options, &"view_mode".into(), &view_mode.into()).unwrap();
    Reflect::set(&options, &"date_format".into(), &date_format.into()).unwrap();
    Reflect::set(&options, &"language".into(), &language.into()).unwrap();
    options
}

/// Convert Vec<GanttTask> to JS Array
pub fn tasks_to_js_array(tasks: &[GanttTask]) -> Array {
    let array = Array::new();
    for task in tasks {
        array.push(&task.to_js_object());
    }
    array
}
