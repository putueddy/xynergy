use leptos::prelude::*;

/// 404 Not Found page
#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="h-full flex items-center justify-center px-4">
            <div class="panel empty-state max-w-md w-full">
                <h1 class="text-4xl font-semibold text-primary-400 mb-2">
                    "404"
                </h1>

                <h2 class="text-xl font-semibold text-huly-caption mb-2">
                    "Page Not Found"
                </h2>

                <p class="text-huly-secondary mb-4">
                    "The page you're looking for doesn't exist."
                </p>

                <a
                    href="/"
                    class="btn-primary btn-press"
                >
                    "Go Home"
                </a>
            </div>
        </div>
    }
}
