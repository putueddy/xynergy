use crate::auth::use_auth;
use crate::components::{Footer, Header, SettingsSidebar};
use crate::pages::{DepartmentsContent, HolidaysContent, UsersContent};
use leptos::*;
use leptos_router::*;

/// Settings page component with sidebar layout
#[component]
pub fn SettingsPage() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();

    {
        let navigate = navigate.clone();
        create_effect(move |_| {
            if !auth.is_authenticated.get() {
                navigate("/login", Default::default());
            }
        });
    }

    view! {
        <div class="min-h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
            <Header/>

            <div class="flex flex-1">
                <SettingsSidebar/>

                <main class="flex-1 p-8">
                    <div class="max-w-6xl mx-auto">
                        <Outlet/>
                    </div>
                </main>
            </div>

            <Footer/>
        </div>
    }
}

/// Settings holidays page
#[component]
pub fn SettingsHolidaysPage() -> impl IntoView {
    view! {
        <HolidaysContent/>
    }
}

/// Settings users page
#[component]
pub fn SettingsUsersPage() -> impl IntoView {
    view! {
        <UsersContent/>
    }
}

/// Settings departments page
#[component]
pub fn SettingsDepartmentsPage() -> impl IntoView {
    view! {
        <DepartmentsContent/>
    }
}
