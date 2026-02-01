use js_sys::{Array, Function, Object, Reflect};
use wasm_bindgen::prelude::*;

/// Vis-timeline Timeline struct
#[wasm_bindgen]
#[derive(Clone)]
extern "C" {
    #[wasm_bindgen(js_name = "vis.Timeline")]
    pub type Timeline;

    /// Create a new Timeline
    #[wasm_bindgen(constructor, js_class = "vis.Timeline")]
    pub fn new(
        container: &web_sys::HtmlElement,
        items: &JsValue,
        groups: &JsValue,
        options: &JsValue,
    ) -> Timeline;

    /// Set items data
    #[wasm_bindgen(method, js_class = "vis.Timeline", js_name = setItems)]
    pub fn set_items(this: &Timeline, items: &JsValue);

    /// Set groups data
    #[wasm_bindgen(method, js_class = "vis.Timeline", js_name = setGroups)]
    pub fn set_groups(this: &Timeline, groups: &JsValue);

    /// Set options
    #[wasm_bindgen(method, js_class = "vis.Timeline", js_name = setOptions)]
    pub fn set_options(this: &Timeline, options: &JsValue);

    /// Fit timeline to show all items
    #[wasm_bindgen(method, js_class = "vis.Timeline")]
    pub fn fit(this: &Timeline);

    /// Move window to specific time
    #[wasm_bindgen(method, js_class = "vis.Timeline", js_name = moveTo)]
    pub fn move_to(this: &Timeline, time: &str);

    /// Set window range
    #[wasm_bindgen(method, js_class = "vis.Timeline", js_name = setWindow)]
    pub fn set_window(this: &Timeline, start: &str, end: &str);

    /// Redraw timeline
    #[wasm_bindgen(method, js_class = "vis.Timeline")]
    pub fn redraw(this: &Timeline);

    /// Add event listener
    #[wasm_bindgen(method, js_class = "vis.Timeline", js_name = on)]
    pub fn on(this: &Timeline, event: &str, callback: &Function);

    /// Remove event listener
    #[wasm_bindgen(method, js_class = "vis.Timeline", js_name = off)]
    pub fn off(this: &Timeline, event: &str, callback: &Function);
}

/// Vis-timeline DataSet
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "vis.DataSet")]
    pub type DataSet;

    /// Create a new DataSet
    #[wasm_bindgen(constructor, js_class = "vis.DataSet")]
    pub fn new(data: &JsValue) -> DataSet;

    /// Add item to dataset
    #[wasm_bindgen(method, js_class = "vis.DataSet")]
    pub fn add(this: &DataSet, item: &JsValue);

    /// Update item in dataset
    #[wasm_bindgen(method, js_class = "vis.DataSet")]
    pub fn update(this: &DataSet, item: &JsValue);

    /// Remove item from dataset
    #[wasm_bindgen(method, js_class = "vis.DataSet", js_name = remove)]
    pub fn remove(this: &DataSet, id: &JsValue);

    /// Clear all items
    #[wasm_bindgen(method, js_class = "vis.DataSet")]
    pub fn clear(this: &DataSet);

    /// Get all items
    #[wasm_bindgen(method, js_class = "vis.DataSet")]
    pub fn get(this: &DataSet) -> JsValue;
}

/// Timeline item structure
#[derive(Debug, Clone)]
pub struct TimelineItem {
    pub id: String,
    pub group: String,
    pub content: String,
    pub start: String,
    pub end: Option<String>,
    pub class_name: Option<String>,
    pub style: Option<String>,
    pub editable: Option<bool>,
}

impl TimelineItem {
    pub fn to_js_object(&self) -> Object {
        let obj = Object::new();
        Reflect::set(&obj, &"id".into(), &self.id.clone().into()).unwrap();
        Reflect::set(&obj, &"group".into(), &self.group.clone().into()).unwrap();
        Reflect::set(&obj, &"content".into(), &self.content.clone().into()).unwrap();
        Reflect::set(&obj, &"start".into(), &self.start.clone().into()).unwrap();

        if let Some(end) = &self.end {
            Reflect::set(&obj, &"end".into(), &end.clone().into()).unwrap();
        }

        if let Some(class_name) = &self.class_name {
            Reflect::set(&obj, &"className".into(), &class_name.clone().into()).unwrap();
        }

        if let Some(style) = &self.style {
            Reflect::set(&obj, &"style".into(), &style.clone().into()).unwrap();
        }

        // Set editable to true by default for drag support
        Reflect::set(
            &obj,
            &"editable".into(),
            &self.editable.unwrap_or(true).into(),
        )
        .unwrap();

        obj
    }
}

/// Timeline group structure
#[derive(Debug, Clone)]
pub struct TimelineGroup {
    pub id: String,
    pub content: String,
    pub class_name: Option<String>,
    pub style: Option<String>,
}

impl TimelineGroup {
    pub fn to_js_object(&self) -> Object {
        let obj = Object::new();
        Reflect::set(&obj, &"id".into(), &self.id.clone().into()).unwrap();
        Reflect::set(&obj, &"content".into(), &self.content.clone().into()).unwrap();

        if let Some(class_name) = &self.class_name {
            Reflect::set(&obj, &"className".into(), &class_name.clone().into()).unwrap();
        }

        if let Some(style) = &self.style {
            Reflect::set(&obj, &"style".into(), &style.clone().into()).unwrap();
        }

        obj
    }
}

/// Convert Vec<TimelineItem> to JS Array
pub fn items_to_js_array(items: &[TimelineItem]) -> Array {
    let array = Array::new();
    for item in items {
        array.push(&item.to_js_object());
    }
    array
}

/// Convert Vec<TimelineGroup> to JS Array
pub fn groups_to_js_array(groups: &[TimelineGroup]) -> Array {
    let array = Array::new();
    for group in groups {
        array.push(&group.to_js_object());
    }
    array
}

/// Create background items for holidays
pub fn create_holiday_background_items(holidays: &[String]) -> Array {
    let array = Array::new();
    for (index, holiday_date) in holidays.iter().enumerate() {
        let obj = Object::new();
        Reflect::set(&obj, &"id".into(), &format!("holiday-{}", index).into()).unwrap();
        Reflect::set(
            &obj,
            &"start".into(),
            &format!("{}T00:00:00", holiday_date).into(),
        )
        .unwrap();
        Reflect::set(
            &obj,
            &"end".into(),
            &format!("{}T23:59:59", holiday_date).into(),
        )
        .unwrap();
        Reflect::set(&obj, &"type".into(), &"background".into()).unwrap();
        Reflect::set(&obj, &"className".into(), &"holiday-background".into()).unwrap();
        // No group assignment - background spans all groups
        array.push(&obj);
    }
    array
}

/// Create timeline options
pub fn create_timeline_options(
    start: &str,
    end: &str,
    editable: bool,
    stack: bool,
    _holidays: &[String],
) -> Object {
    let options = Object::new();

    Reflect::set(&options, &"start".into(), &start.into()).unwrap();
    Reflect::set(&options, &"end".into(), &end.into()).unwrap();
    // Enable editable; deletion is hidden via CSS
    Reflect::set(&options, &"editable".into(), &editable.into()).unwrap();

    Reflect::set(&options, &"stack".into(), &stack.into()).unwrap();

    // Set orientation to top
    let orientation = Object::new();
    Reflect::set(&orientation, &"axis".into(), &"top".into()).unwrap();
    Reflect::set(&orientation, &"item".into(), &"top".into()).unwrap();
    Reflect::set(&options, &"orientation".into(), &orientation).unwrap();

    // Configure time axis for daily view
    let time_axis = Object::new();
    Reflect::set(&time_axis, &"scale".into(), &"day".into()).unwrap();
    Reflect::set(&time_axis, &"step".into(), &1.into()).unwrap();
    Reflect::set(&options, &"timeAxis".into(), &time_axis).unwrap();

    // Format to show day and date
    let format = Object::new();
    let minor_labels = Object::new();
    Reflect::set(&minor_labels, &"millisecond".into(), &"SSS".into()).unwrap();
    Reflect::set(&minor_labels, &"second".into(), &"s".into()).unwrap();
    Reflect::set(&minor_labels, &"minute".into(), &"HH:mm".into()).unwrap();
    Reflect::set(&minor_labels, &"hour".into(), &"HH:mm".into()).unwrap();
    Reflect::set(&minor_labels, &"weekday".into(), &"ddd D".into()).unwrap();
    Reflect::set(&minor_labels, &"day".into(), &"D".into()).unwrap();
    Reflect::set(&minor_labels, &"week".into(), &"w".into()).unwrap();
    Reflect::set(&minor_labels, &"month".into(), &"MMM".into()).unwrap();
    Reflect::set(&minor_labels, &"year".into(), &"YYYY".into()).unwrap();
    Reflect::set(&format, &"minorLabels".into(), &minor_labels).unwrap();

    let major_labels = Object::new();
    Reflect::set(&major_labels, &"millisecond".into(), &"HH:mm:ss".into()).unwrap();
    Reflect::set(&major_labels, &"second".into(), &"D MMMM HH:mm".into()).unwrap();
    Reflect::set(&major_labels, &"minute".into(), &"ddd D MMMM".into()).unwrap();
    Reflect::set(&major_labels, &"hour".into(), &"ddd D MMMM".into()).unwrap();
    Reflect::set(&major_labels, &"weekday".into(), &"MMMM YYYY".into()).unwrap();
    Reflect::set(&major_labels, &"day".into(), &"MMMM YYYY".into()).unwrap();
    Reflect::set(&major_labels, &"week".into(), &"MMMM YYYY".into()).unwrap();
    Reflect::set(&major_labels, &"month".into(), &"YYYY".into()).unwrap();
    Reflect::set(&major_labels, &"year".into(), &"".into()).unwrap();
    Reflect::set(&format, &"majorLabels".into(), &major_labels).unwrap();
    Reflect::set(&options, &"format".into(), &format).unwrap();

    // Add weekend and holiday highlighting using CSS classes
    // Vis-timeline automatically adds vis-saturday and vis-sunday classes
    // We style these in CSS with gray 40% background

    options
}
