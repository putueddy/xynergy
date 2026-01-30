use leptos::*;
use leptos_router::*;
use crate::auth::{use_auth, login_user};
use crate::components::{Header, Footer};

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
                navigate("/dashboard", Default::default());
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
                        auth.user.set(Some(response.user));
                        auth.token.set(Some(response.token));
                        navigate("/dashboard", Default::default());
                    }
                    Err(e) => {
                        set_error.set(Some(e));
                        set_loading.set(false);
                    }
                }
            });
        }
    };
    
    view! {
        <div class="min-h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
            <Header/>
            
            <main class="flex-grow flex items-center justify-center px-4 sm:px-6 lg:px-8">
                <div class="max-w-md w-full space-y-8">
                    <div>
                        <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900 dark:text-white">
                            "Sign in to your account"
                        </h2>
                        <p class="mt-2 text-center text-sm text-gray-600 dark:text-gray-400">
                            "Xynergy Resource Management"
                        </p>
                    </div>
                    
                    <form class="mt-8 space-y-6" on:submit=handle_submit>
                        <div class="rounded-md shadow-sm -space-y-px">
                            <div>
                                <label for="email" class="sr-only">"Email address"</label>
                                <input
                                    id="email"
                                    name="email"
                                    type="email"
                                    required
                                    class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-t-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm dark:bg-gray-800 dark:border-gray-600 dark:text-white"
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
                                    class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-b-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm dark:bg-gray-800 dark:border-gray-600 dark:text-white"
                                    placeholder="Password"
                                    prop:value=password
                                    on:input=move |ev| set_password.set(event_target_value(&ev))
                                />
                            </div>
                        </div>

                        {move || error.get().map(|err| {
                            view! {
                                <div class="rounded-md bg-red-50 p-4 dark:bg-red-900/20">
                                    <div class="flex">
                                        <div class="ml-3">
                                            <h3 class="text-sm font-medium text-red-800 dark:text-red-200">
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
                        
                        <div class="text-center text-sm text-gray-600 dark:text-gray-400">
                            <p>"Default credentials:"</p>
                            <p>"Email: admin@xynergy.com"</p>
                            <p>"Password: admin123"</p>
                        </div>
                    </form>
                </div>
            </main>
            
            <Footer/>
        </div>
    }
}
