use crate::auth::{logout_user, use_auth};
use crate::components::{Footer, Header};
use leptos::*;
use leptos_router::*;

/// Dashboard page component
#[component]
pub fn Dashboard() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();

    // Redirect if not logged in
    {
        let navigate = navigate.clone();
        create_effect(move |_| {
            if !auth.is_authenticated.get() {
                navigate("/login", Default::default());
            }
        });
    }

    let handle_logout = {
        let navigate = navigate.clone();
        move |_| {
            logout_user(&auth);
            navigate("/", Default::default());
        }
    };

    let user = auth.user;

    view! {
        <div class="min-h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
            <Header/>

            <main class="flex-grow max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
                <div class="space-y-8">
                    <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6">
                        <div class="flex items-center justify-between">
                            <div>
                                <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                                    {move || user.get().map(|u| format!("Welcome, {}!", u.first_name)).unwrap_or_else(|| "Welcome!".to_string())}
                                </h1>
                                <p class="mt-2 text-gray-600 dark:text-gray-300">
                                    {move || user.get().map(|u| format!("Role: {}", u.role)).unwrap_or_default()}
                                </p>
                                <p class="text-sm text-gray-500 dark:text-gray-400">
                                    {move || user.get().map(|u| u.email).unwrap_or_default()}
                                </p>
                            </div>
                            <button
                                on:click=handle_logout
                                class="btn-secondary"
                            >
                                "Logout"
                            </button>
                        </div>
                    </div>

                    <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
                        <div class="card">
                            <div class="flex items-center">
                                <div class="flex-shrink-0">
                                    <svg class="h-6 w-6 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"/>
                                    </svg>
                                </div>
                                <div class="ml-4">
                                    <h3 class="text-lg font-medium text-gray-900 dark:text-white">"Resources"</h3>
                                    <p class="text-sm text-gray-500 dark:text-gray-400">"Manage team members and assets"</p>
                                </div>
                            </div>
                        </div>

                        <div class="card">
                            <div class="flex items-center">
                                <div class="flex-shrink-0">
                                    <svg class="h-6 w-6 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01"/>
                                    </svg>
                                </div>
                                <div class="ml-4">
                                    <h3 class="text-lg font-medium text-gray-900 dark:text-white">"Projects"</h3>
                                    <p class="text-sm text-gray-500 dark:text-gray-400">"View and manage projects"</p>
                                </div>
                            </div>
                        </div>

                        <div class="card">
                            <div class="flex items-center">
                                <div class="flex-shrink-0">
                                    <svg class="h-6 w-6 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>
                                    </svg>
                                </div>
                                <div class="ml-4">
                                    <h3 class="text-lg font-medium text-gray-900 dark:text-white">"Allocations"</h3>
                                    <p class="text-sm text-gray-500 dark:text-gray-400">"Schedule resources"</p>
                                </div>
                            </div>
                        </div>
                    </div>

                    <div class="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-6">
                        <h3 class="text-lg font-medium text-blue-900 dark:text-blue-200 mb-2">"Coming Soon"</h3>
                        <ul class="list-disc list-inside text-blue-800 dark:text-blue-300 space-y-1">
                            <li>"Interactive Gantt charts"</li>
                            <li>"Resource allocation interface"</li>
                            <li>"Project management tools"</li>
                            <li>"Team collaboration features"</li>
                        </ul>
                    </div>
                </div>
            </main>

            <Footer/>
        </div>
    }
}
