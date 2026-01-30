use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use wasm_bindgen::JsCast;

pub mod components;
pub mod pages;
pub mod auth;

use pages::{home::Home, login::Login, dashboard::Dashboard, not_found::NotFound, resources::Resources};
use auth::{AuthContext, provide_auth_context};

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

        // content for this welcome page
        <Router>
            <main>
                <Routes>
                    <Route path="/" view=Home/>
                    <Route path="/login" view=Login/>
                    <Route path="/dashboard" view=Dashboard/>
                    <Route path="/resources" view=Resources/>
                    <Route path="/*any" view=NotFound/>
                </Routes>
            </main>
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
        .and_then(|d| d.get_element_by_id("root")) {
        web_sys::console::log_1(&"Found root element, mounting...".into());
        leptos::mount_to(root.unchecked_into(), App);
        web_sys::console::log_1(&"Xynergy app mounted to root".into());
    } else {
        web_sys::console::error_1(&"Could not find root element!".into());
    }
}
