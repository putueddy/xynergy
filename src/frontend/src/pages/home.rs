use crate::components::{Footer, Header, PrimaryButton, SecondaryButton};
use leptos::*;

/// Home page component
#[component]
pub fn Home() -> impl IntoView {
    let (count, set_count) = create_signal(0);

    view! {
        <div class="min-h-screen flex flex-col">
            <Header/>

            <main class="flex-grow max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
                <div class="text-center">
                    <h2 class="text-4xl font-bold text-gray-900 dark:text-white mb-4">
                        "Welcome to Xynergy"
                    </h2>

                    <p class="text-xl text-gray-600 dark:text-gray-300 mb-8">
                        "Resource Management and Project Planning Platform"
                    </p>

                    <div class="card max-w-md mx-auto">
                        <h3 class="text-lg font-semibold mb-4">
                            "Counter Example"
                        </h3>

                        <p class="text-3xl font-bold text-blue-600 mb-4">
                            {move || count.get()}
                        </p>

                        <div class="flex justify-center space-x-4">
                            <PrimaryButton
                                text="Increment".to_string()
                                on_click=Box::new(move || set_count.update(|n| *n += 1))
                            />

                            <SecondaryButton
                                text="Decrement".to_string()
                                on_click=Box::new(move || set_count.update(|n| *n -= 1))
                            />
                        </div>
                    </div>

                    <div class="mt-12 grid grid-cols-1 md:grid-cols-3 gap-6">
                        <FeatureCard
                            title="Resource Planning"
                            description="Efficiently allocate and manage resources across projects"
                        />

                        <FeatureCard
                            title="Project Tracking"
                            description="Monitor project progress with interactive Gantt charts"
                        />

                        <FeatureCard
                            title="Team Collaboration"
                            description="Real-time updates and seamless team coordination"
                        />
                    </div>
                </div>
            </main>

            <Footer/>
        </div>
    }
}

/// Feature card component
#[component]
fn FeatureCard(#[prop(into)] title: String, #[prop(into)] description: String) -> impl IntoView {
    view! {
        <div class="card">
            <h3 class="text-xl font-semibold text-gray-900 dark:text-white mb-2">
                {title}
            </h3>
            <p class="text-gray-600 dark:text-gray-300">
                {description}
            </p>
        </div>
    }
}
