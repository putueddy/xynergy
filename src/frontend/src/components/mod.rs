use crate::auth::use_auth;
use leptos::*;

pub mod allocation_form;
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

/// Header component
#[component]
pub fn Header() -> impl IntoView {
    let auth = use_auth();

    view! {
        <header class="bg-white dark:bg-gray-800 shadow-sm">
            <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
                <div class="flex items-center justify-between">
                    <div class="flex items-center">
                        <h1 class="text-2xl font-bold text-blue-600 dark:text-blue-400">
                            <a href="/" class="text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300">
                                "Xynergy"
                            </a>
                        </h1>
                    </div>

                    <nav class="hidden md:flex space-x-8">
                        <a href="/dashboard" class="text-gray-600 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400">
                            "Dashboard"
                        </a>
                        <a href="/resources" class="text-gray-600 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400">
                            "Resources"
                        </a>
                        <a href="/projects" class="text-gray-600 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400">
                            "Projects"
                        </a>
                        <a href="/allocations" class="text-gray-600 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400">
                            "Allocations"
                        </a>
                        {move || {
                            if auth.user.get().map(|u| u.role == "hr").unwrap_or(false) {
                                view! {
                                    <a href="/ctc" class="text-gray-600 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400">
                                        "CTC"
                                    </a>
                                }
                                    .into_view()
                            } else {
                                view! { <></> }.into_view()
                            }
                        }}
                        <a href="/settings" class="text-gray-600 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400">
                            "Settings"
                        </a>
                    </nav>
                </div>
            </div>
        </header>
    }
}

/// Footer component
#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="bg-gray-100 dark:bg-gray-900 mt-auto">
            <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
                <p class="text-center text-gray-600 dark:text-gray-400">
                    "© 2026 Xynergy. All rights reserved."
                </p>
            </div>
        </footer>
    }
}

/// Button component - Primary style
#[component]
pub fn PrimaryButton(
    #[prop(into)] text: String,
    on_click: Box<dyn Fn() + 'static>,
) -> impl IntoView {
    view! {
        <button
            class="btn-primary"
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
            class="btn-secondary"
            on:click=move |_| on_click()
        >
            {text}
        </button>
    }
}
