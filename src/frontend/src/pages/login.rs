use crate::auth::{login_user, use_auth};

use leptos::*;
use leptos_router::*;

fn role_dashboard_path(role: &str) -> &'static str {
    match role {
        "admin" => "/settings/users",
        "hr" => "/resources",
        "department_head" => "/allocations",
        "project_manager" => "/projects",
        "finance" => "/dashboard",
        _ => "/dashboard",
    }
}

/// Login page component
#[component]
pub fn Login() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();

    // Form state
    let (email, set_email) = create_signal("".to_string());
    let (password, set_password) = create_signal("".to_string());
    let (error, set_error) = create_signal(Option::<String>::None);
    let (loading, set_loading) = create_signal(false);

    // Redirect if already logged in
    {
        let navigate = navigate.clone();
        create_effect(move |_| {
            if auth.is_authenticated.get() {
                let path = auth
                    .user
                    .get()
                    .map(|u| role_dashboard_path(&u.role).to_string())
                    .unwrap_or_else(|| "/dashboard".to_string());
                navigate(&path, Default::default());
            }
        });
    }

    // Handle form submission
    let handle_submit = {
        let navigate = navigate.clone();
        move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();

            set_loading.set(true);
            set_error.set(None);

            let email_val = email.get();
            let password_val = password.get();
            let navigate = navigate.clone();

            spawn_local(async move {
                match login_user(email_val, password_val).await {
                    Ok(response) => {
                        let destination = role_dashboard_path(&response.user.role).to_string();
                        auth.user.set(Some(response.user));
                        auth.token.set(Some(response.token));
                        auth.refresh_token.set(Some(response.refresh_token));
                        navigate(&destination, Default::default());
                    }
                    Err(e) => {
                        // Display generic error message (no user enumeration)
                        set_error.set(Some(e));
                        set_loading.set(false);
                    }
                }
            });
        }
    };

    view! {
        <div class="h-full flex flex-col bg-huly-back">


            <div class="flex-grow flex items-center justify-center px-4">
                <div class="max-w-md w-full space-y-4">
                    <div>
                        <h2 class="mt-4 text-center text-xl font-semibold text-huly-caption">
                            "Sign in to your account"
                        </h2>
                        <p class="mt-2 text-center text-sm text-huly-secondary">
                            "Xynergy Resource Management"
                        </p>
                    </div>

                    <form class="panel p-4 space-y-4" on:submit=handle_submit>
                        <div class="rounded-md shadow-sm -space-y-px">
                            <div>
                                <label for="email" class="sr-only">"Email address"</label>
                                <input
                                    id="email"
                                    name="email"
                                    type="email"
                                    required
                                    class="input rounded-t-md"
                                    placeholder="Email address"
                                    prop:value=email
                                    on:input=move |ev| set_email.set(event_target_value(&ev))
                                />
                            </div>
                            <div>
                                <label for="password" class="sr-only">"Password"</label>
                                <input
                                    id="password"
                                    name="password"
                                    type="password"
                                    required
                                    class="input rounded-b-md"
                                    placeholder="Password"
                                    prop:value=password
                                    on:input=move |ev| set_password.set(event_target_value(&ev))
                                />
                            </div>
                        </div>

                        {move || error.get().map(|err| {
                            view! {
                                <div class="alert-error">
                                    <div class="flex">
                                        <div class="ml-3">
                                            <h3 class="text-sm font-medium text-negative-default">
                                                {err}
                                            </h3>
                                        </div>
                                    </div>
                                </div>
                            }
                        })}

                        <div>
                            <button
                                type="submit"
                                disabled=loading
                                class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                {move || if loading.get() {
                                    "Signing in..."
                                } else {
                                    "Sign in"
                                }}
                            </button>
                        </div>

                        <div class="text-center text-sm text-huly-secondary">
                            <p>"Default credentials:"</p>
                            <p>"Email: admin@xynergy.com"</p>
                            <p>"Password: admin123"</p>
                        </div>
                    </form>
                </div>
            </div>


        </div>
    }
}
