use leptos::*;
use leptos_router::*;

/// Settings sidebar component
#[component]
pub fn SettingsSidebar() -> impl IntoView {
    let location = use_location();

    // Get current pathname as a signal
    let pathname = Signal::derive(move || location.pathname.get());

    view! {
        <aside class="w-64 bg-white dark:bg-gray-800 shadow-sm min-h-screen">
            <div class="p-6">
                <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">
                    "Settings"
                </h2>

                <nav class="space-y-2">
                    <SidebarLink
                        href="/settings/holidays"
                        label="Holidays"
                        icon={holiday_icon()}
                        current_path=pathname
                    />

                    <SidebarLink
                        href="/settings/users"
                        label="Users"
                        icon={users_icon()}
                        current_path=pathname
                    />

                    <SidebarLink
                        href="/settings/departments"
                        label="Departments"
                        icon={department_icon()}
                        current_path=pathname
                    />
                </nav>
            </div>
        </aside>
    }
}

/// Sidebar link component
#[component]
fn SidebarLink(
    href: &'static str,
    label: &'static str,
    icon: impl IntoView,
    current_path: Signal<String>,
) -> impl IntoView {
    let is_active = Signal::derive(move || current_path.get().starts_with(href));

    view! {
        <a
            href={href}
            class={move || {
                let base_classes = "flex items-center px-4 py-3 text-sm font-medium rounded-lg transition-colors";
                if is_active.get() {
                    format!("{} bg-blue-50 text-blue-700 dark:bg-blue-900/20 dark:text-blue-300", base_classes)
                } else {
                    format!("{} text-gray-700 hover:bg-gray-50 dark:text-gray-300 dark:hover:bg-gray-700", base_classes)
                }
            }}
        >
            <span class="w-5 h-5 mr-3">{icon}</span>
            {label}
        </a>
    }
}

/// Holiday icon SVG
fn holiday_icon() -> impl IntoView {
    view! {
        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"></path>
        </svg>
    }
}

/// Users icon SVG
fn users_icon() -> impl IntoView {
    view! {
        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z"></path>
        </svg>
    }
}

/// Department icon SVG
fn department_icon() -> impl IntoView {
    view! {
        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4"></path>
        </svg>
    }
}
