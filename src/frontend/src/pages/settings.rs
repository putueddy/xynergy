use crate::auth::use_auth;
use crate::components::SettingsSidebar;
use crate::pages::{DepartmentsContent, HolidaysContent, UsersContent};
use leptos::prelude::*;
use leptos_router::hooks::*;
use leptos_router::nested_router::Outlet;

/// Settings page component with sidebar layout
#[component]
pub fn SettingsPage() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();

    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            if !auth.is_authenticated.get() {
                navigate("/login", Default::default());
            }
        });
    }

    view! {
        <div class="h-full flex">

            <div class="flex flex-1">
                <SettingsSidebar/>

                <main class="flex-1 p-8">
                    <div class="max-w-6xl mx-auto">
                        <Outlet/>
                    </div>
                </main>
            </div>

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
