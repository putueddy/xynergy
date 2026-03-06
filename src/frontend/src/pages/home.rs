use crate::components::{PrimaryButton, SecondaryButton};
use leptos::*;

/// Home page component
#[component]
pub fn Home() -> impl IntoView {
    let (count, set_count) = create_signal(0);

    view! {
        <div class="h-full">


            <div class="page-container fade-in">
                <div class="text-center">
                    <h2 class="text-2xl font-semibold text-huly-caption mb-3">
                        "Welcome to Xynergy"
                    </h2>

                    <p class="text-base text-huly-secondary mb-6">
                        "Resource Management and Project Planning Platform"
                    </p>

                    <div class="panel p-4 max-w-md mx-auto">
                        <h3 class="text-base font-semibold mb-3">
                            "Counter Example"
                        </h3>

                        <p class="text-2xl font-semibold text-blue-600 mb-3">
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

                    <div class="mt-8 grid grid-cols-1 md:grid-cols-3 gap-3">
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
            </div>


        </div>
    }
}

/// Feature card component
#[component]
fn FeatureCard(#[prop(into)] title: String, #[prop(into)] description: String) -> impl IntoView {
    view! {
        <div class="stat-card text-left">
            <div>
                <h3 class="stat-label mb-1">{title}</h3>
                <p class="text-sm text-huly-secondary">{description}</p>
            </div>
        </div>
    }
}
