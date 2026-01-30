use leptos::*;

/// 404 Not Found page
#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900">
            <div class="text-center">
                <h1 class="text-6xl font-bold text-blue-600 dark:text-blue-400 mb-4">
                    "404"
                </h1>

                <h2 class="text-2xl font-semibold text-gray-900 dark:text-white mb-4">
                    "Page Not Found"
                </h2>

                <p class="text-gray-600 dark:text-gray-300 mb-8">
                    "The page you're looking for doesn't exist."
                </p>

                <a
                    href="/"
                    class="btn-primary"
                >
                    "Go Home"
                </a>
            </div>
        </div>
    }
}
