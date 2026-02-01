use leptos::*;
use leptos_router::*;

/// Settings sidebar component
#[component]
pub fn SettingsSidebar() -> impl IntoView {
    let location = use_location();

    // Helper to check if a path is active
    let is_active = move |path: &str| location.pathname.get().starts_with(path);

    view! {
        <aside class="w-64 bg-white dark:bg-gray-800 shadow-sm min-h-screen">
            <div class="p-6">
                <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">
                    "Settings"
                </h2>

                <nav class="space-y-2">
                    <a
                        href="/settings/holidays"
                        class={move || {
                            let base_classes = "flex items-center px-4 py-3 text-sm font-medium rounded-lg transition-colors";
                            let active_classes = "bg-blue-50 text-blue-700 dark:bg-blue-900/20 dark:text-blue-300";
                            let inactive_classes = "text-gray-700 hover:bg-gray-50 dark:text-gray-300 dark:hover:bg-gray-700";

                            if is_active("/settings/holidays") {
                                format!("{} {}", base_classes, active_classes)
                            } else {
                                format!("{} {}", base_classes, inactive_classes)
                            }
                        }}
                    >
                        <svg class="w-5 h-5 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                  d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"></path>
                        </svg>
                        "Holidays"
                    </a>

                    <a
                        href="/settings/users"
                        class={move || {
                            let base_classes = "flex items-center px-4 py-3 text-sm font-medium rounded-lg transition-colors";
                            let active_classes = "bg-blue-50 text-blue-700 dark:bg-blue-900/20 dark:text-blue-300";
                            let inactive_classes = "text-gray-700 hover:bg-gray-50 dark:text-gray-300 dark:hover:bg-gray-700";

                            if is_active("/settings/users") {
                                format!("{} {}", base_classes, active_classes)
                            } else {
                                format!("{} {}", base_classes, inactive_classes)
                            }
                        }}
                    >
                        <svg class="w-5 h-5 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                  d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z"></path>
                        </svg>
                        "Users"
                    </a>
                </nav>
            </div>
        </aside>
    }
}
