use crate::auth::{logout_user, use_auth};
use leptos::*;
use leptos_router::*;

pub mod allocation_form;
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
    let navigate = use_navigate();

    view! {
        <header class="bg-white dark:bg-gray-800 shadow-sm">
            <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
                <div class="flex items-center justify-between">
                    <div class="flex items-center">
                        <h1 class="text-2xl font-bold text-blue-600 dark:text-blue-400">
                            "Xynergy"
                        </h1>
                    </div>

                    <div class="flex items-center space-x-8">
                        <nav class="hidden md:flex space-x-8">
                            <a href="/" class="text-gray-600 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400">
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
                            <a href="/settings" class="text-gray-600 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400">
                                "Settings"
                            </a>
                        </nav>

                        {move || {
                            if auth.is_authenticated.get() {
                                view! {
                                    <button
                                        class="text-gray-600 dark:text-gray-300 hover:text-red-600 dark:hover:text-red-400 text-sm font-medium"
                                        on:click={
                                            let auth = auth;
                                            let navigate = navigate.clone();
                                            move |_| {
                                                logout_user(&auth);
                                                navigate("/login", Default::default());
                                            }
                                        }
                                    >
                                        "Logout"
                                    </button>
                                }.into_view()
                            } else {
                                view! { <div></div> }.into_view()
                            }
                        }}
                    </div>
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
