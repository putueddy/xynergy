use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use wasm_bindgen::JsCast;

pub mod auth;
pub mod components;
pub mod gantt;
pub mod pages;
pub mod timeline;

use auth::{provide_auth_context, use_auth};
use components::AppSidebar;
use pages::{
    allocations::Allocations,
    ctc::CtcManagement,
    ctc_completeness::CtcCompleteness,
    dashboard::Dashboard,
    home::Home,
    login::Login,
    not_found::NotFound,
    projects::Projects,
    resources::Resources,
    settings::{SettingsDepartmentsPage, SettingsHolidaysPage, SettingsPage, SettingsUsersPage},
    team::TeamPage,
    thr::ThrManagement,
};

/// App shell — conditionally renders sidebar for authenticated routes
#[component]
fn AppShell(children: Children) -> impl IntoView {
    let auth = use_auth();
    let location = use_location();

    // Mobile sidebar toggle
    let (mobile_open, set_mobile_open) = create_signal(false);

    // Hide sidebar on public routes (home + login)
    let show_sidebar = Signal::derive(move || {
        let path = location.pathname.get();
        let is_public = path == "/" || path == "/login";
        let is_authenticated = auth.is_authenticated.get();
        !is_public && is_authenticated
    });

    view! {
        <div class="h-screen w-screen flex overflow-hidden bg-huly-back">
            // Desktop sidebar (hidden on mobile)
            <Show when=move || show_sidebar.get()>
                <div class="hidden md:flex">
                    <AppSidebar />
                </div>
            </Show>

            // Mobile sidebar overlay
            <Show when=move || show_sidebar.get() && mobile_open.get()>
                <div
                    class="fixed inset-0 z-40 bg-black/50 md:hidden"
                    on:click=move |_| set_mobile_open.set(false)
                ></div>
                <div class="fixed inset-y-0 left-0 z-50 md:hidden">
                    <AppSidebar />
                </div>
            </Show>

            <div class="flex-1 flex flex-col min-w-0 overflow-hidden">
                // Mobile header bar with hamburger
                <Show when=move || show_sidebar.get()>
                    <div class="md:hidden flex items-center h-12 px-3 border-b border-huly-divider flex-shrink-0 bg-huly-nav">
                        <button
                            class="p-1.5 rounded-md text-huly-content hover:bg-huly-btn-hover transition-colors duration-150"
                            on:click=move |_| set_mobile_open.update(|v| *v = !*v)
                        >
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.5">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5" />
                            </svg>
                        </button>
                        <span class="ml-3 text-sm font-semibold text-huly-caption">"Xynergy"</span>
                    </div>
                </Show>
                <main class="flex-1 overflow-y-auto">
                    {children()}
                </main>
            </div>
        </div>
    }
}

/// Main application component
#[component]
pub fn App() -> impl IntoView {
    web_sys::console::log_1(&"App component starting...".into());

    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    // Provide authentication context
    provide_auth_context();

    web_sys::console::log_1(&"Contexts provided, rendering routes...".into());

    view! {
        // sets the document title
        <Title text="Xynergy - Resource Management"/>

        // injects metadata in the <head> of the page
        <Meta charset="UTF-8"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>

        <Router>
            <AppShell>
                <Routes>
                    <Route path="/" view=Home/>
                    <Route path="/login" view=Login/>
                    <Route path="/dashboard" view=Dashboard/>
                    <Route path="/resources" view=Resources/>
                    <Route path="/projects" view=Projects/>
                    <Route path="/allocations" view=Allocations/>
                    <Route path="/team" view=TeamPage/>
                    <Route path="/ctc" view=CtcManagement/>
                    <Route path="/ctc/completeness" view=CtcCompleteness/>
                    <Route path="/thr" view=ThrManagement/>
                    <Route path="/settings" view=SettingsPage>
                        <Route path="/holidays" view=SettingsHolidaysPage/>
                        <Route path="/users" view=SettingsUsersPage/>
                        <Route path="/departments" view=SettingsDepartmentsPage/>
                        <Route path="" view=SettingsHolidaysPage/>
                    </Route>
                    <Route path="/*any" view=NotFound/>
                </Routes>
            </AppShell>
        </Router>
    }
}

/// Initialize the application (called from JavaScript)
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"Starting Xynergy app...".into());

    // Mount to the root div instead of body
    if let Some(root) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("root"))
    {
        web_sys::console::log_1(&"Found root element, mounting...".into());
        leptos::mount_to(root.unchecked_into(), App);
        web_sys::console::log_1(&"Xynergy app mounted to root".into());
    } else {
        web_sys::console::error_1(&"Could not find root element!".into());
    }
}
