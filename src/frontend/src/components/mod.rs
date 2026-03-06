use leptos::prelude::*;

pub mod allocation_form;
pub mod app_sidebar;
pub mod department_form;
pub mod gantt_chart;
pub mod holiday_form;
pub mod project_form;
pub mod project_list;
pub mod resource_form;
pub mod resource_list;
pub mod settings_sidebar;
pub mod timeline_chart;
pub mod user_form;

pub use app_sidebar::AppSidebar;
pub use allocation_form::{
    AllocationEditData, AllocationForm, AllocationFormData, ProjectOption, ResourceOption,
};
pub use department_form::{DepartmentEditData, DepartmentForm, DepartmentFormData, HeadCandidate};
pub use gantt_chart::{GanttChart, GanttTaskItem};
pub use holiday_form::{HolidayForm, HolidayFormData};
pub use project_form::ProjectForm;
pub use project_list::ProjectList;
pub use resource_form::ResourceForm;
pub use resource_list::ResourceList;
pub use settings_sidebar::SettingsSidebar;
pub use timeline_chart::{AllocationItem, ResourceGroup, TimelineChart};
pub use user_form::{DepartmentOption, UserEditData, UserForm, UserFormData};


/// Button component - Primary style
#[component]
pub fn PrimaryButton(
    #[prop(into)] text: String,
    on_click: Box<dyn Fn() + 'static>,
) -> impl IntoView {
    view! {
        <button
            class="btn-primary btn-press"
            on:click=move |_| on_click()
        >
            {text}
        </button>
    }
}

/// Button component - Secondary style
#[component]
pub fn SecondaryButton(
    #[prop(into)] text: String,
    on_click: Box<dyn Fn() + 'static>,
) -> impl IntoView {
    view! {
        <button
            class="btn-secondary btn-press"
            on:click=move |_| on_click()
        >
            {text}
        </button>
    }
}
