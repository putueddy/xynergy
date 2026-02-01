use crate::components::{Footer, Header, SettingsSidebar};
use crate::pages::{HolidaysContent, UsersContent};
use leptos::*;
use leptos_router::*;

/// Settings page component with sidebar layout
#[component]
pub fn SettingsPage() -> impl IntoView {
    view! {
        <div class="min-h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
            <Header/>

            <div class="flex flex-1">
                <SettingsSidebar/>

                <main class="flex-1 p-8">
                    <div class="max-w-6xl mx-auto">
                        <Routes>
                            <Route path="/holidays" view=HolidaysContent/>
                            <Route path="/users" view=UsersContent/>
                            <Route path="" view=SettingsRedirect/>
                        </Routes>
                    </div>
                </main>
            </div>

            <Footer/>
        </div>
    }
}

/// Redirect to holidays as default settings page
#[component]
fn SettingsRedirect() -> impl IntoView {
    view! {
        <div class="flex items-center justify-center h-64">
            <p class="text-gray-500 dark:text-gray-400">
                "Select a settings category from the sidebar"
            </p>
        </div>
    }
}
