use leptos::*;
use leptos_router::*;
use crate::auth::use_auth;
use crate::components::{Header, Footer, HolidayForm, HolidayFormData};
use uuid::Uuid;
use serde::Deserialize;

/// Holiday data structure
#[derive(Debug, Clone, Deserialize)]
pub struct Holiday {
    pub id: Uuid,
    pub name: String,
    pub date: String,
    pub description: Option<String>,
}

/// Holiday management page
#[component]
pub fn HolidaysPage() -> impl IntoView {
    let auth = use_auth();
    
    // Redirect if not authenticated
    create_effect(move |_| {
        if !auth.is_authenticated.get() {
            // Use navigate to redirect to login
            let navigate = leptos_router::use_navigate();
            navigate("/login", Default::default());
        }
    });

    let (holidays, set_holidays) = create_signal(Vec::<Holiday>::new());
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);
    let (show_form, set_show_form) = create_signal(false);
    let (editing_holiday, set_editing_holiday) = create_signal(Option::<Holiday>::None);
    let (form_submitting, set_form_submitting) = create_signal(false);
    let (deleting_id, set_deleting_id) = create_signal(Option::<String>::None);

    // Load holidays on mount
    create_effect(move |_| {
        set_loading.set(true);
        spawn_local(async move {
            match fetch_holidays().await {
                Ok(data) => set_holidays.set(data),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    });

    // Handle form submission
    let handle_submit = move |form_data: HolidayFormData| {
        let editing_id = editing_holiday.get().map(|h| h.id);
        spawn_local(async move {
            set_form_submitting.set(true);
            set_error.set(None);

            let result = if let Some(holiday_id) = editing_id {
                update_holiday(holiday_id.to_string(), form_data).await
            } else {
                create_holiday(form_data).await
            };

            match result {
                Ok(_) => {
                    match fetch_holidays().await {
                        Ok(data) => {
                            set_holidays.set(data);
                            set_show_form.set(false);
                            set_editing_holiday.set(None);
                        }
                        Err(e) => set_error.set(Some(e)),
                    }
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_form_submitting.set(false);
        });
    };

    let handle_cancel = move |_| {
        set_show_form.set(false);
        set_editing_holiday.set(None);
    };

    view! {
        <div class="min-h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
            <Header/>
            
            <main class="flex-grow max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
                <div class="mb-8">
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white mb-2">
                        "Holiday Management"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400">
                        "Manage company holidays and days off"
                    </p>
                </div>

                <div class="mb-6 flex justify-between items-center">
                    <button
                        class="btn-primary"
                        on:click=move |_| {
                            set_editing_holiday.set(None);
                            set_show_form.set(true);
                        }
                    >
                        "Add Holiday"
                    </button>
                </div>

                {move || error.get().map(|err| {
                    view! {
                        <div class="rounded-md bg-red-50 p-4 mb-6 dark:bg-red-900/20">
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

                {move || {
                    if show_form.get() {
                        let is_edit = editing_holiday.get().is_some();
                        let title = if is_edit { "Edit Holiday" } else { "Create Holiday" };
                        let edit_data = editing_holiday.get().map(|h| HolidayFormData {
                            name: h.name,
                            date: h.date,
                            description: h.description.unwrap_or_default(),
                        });
                        view! {
                            <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 mb-6 relative">
                                <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">
                                    {title}
                                </h2>
                                <HolidayForm
                                    editing_holiday={edit_data}
                                    is_submitting=form_submitting.get()
                                    on_submit=Callback::new(handle_submit)
                                    on_cancel=Callback::new(handle_cancel)
                                />
                                {move || {
                                    if form_submitting.get() {
                                        view! {
                                            <div class="absolute inset-0 flex items-center justify-center bg-white/70 dark:bg-gray-800/70 rounded-lg">
                                                <div class="text-center">
                                                    <div class="spinner mx-auto mb-2"></div>
                                                    <p class="text-sm text-gray-600 dark:text-gray-400">"Saving..."</p>
                                                </div>
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! { <div></div> }.into_view()
                                    }
                                }}
                            </div>
                        }.into_view()
                    } else {
                        view! { <div></div> }.into_view()
                    }
                }}

                {move || {
                    if loading.get() {
                        view! {
                            <div class="text-center py-12 bg-white dark:bg-gray-800 rounded-lg shadow">
                                <div class="spinner mx-auto mb-4"></div>
                                <p class="text-gray-600 dark:text-gray-400">"Loading holidays..."</p>
                            </div>
                        }.into_view()
                    } else if holidays.get().is_empty() {
                        view! {
                            <div class="text-center py-12 bg-white dark:bg-gray-800 rounded-lg shadow">
                                <p class="text-gray-600 dark:text-gray-400">"No holidays found."</p>
                                <p class="text-sm text-gray-500 dark:text-gray-500 mt-2">"Click 'Add Holiday' to create one."</p>
                            </div>
                        }.into_view()
                    } else {
                        view! {
                            <div class="bg-white dark:bg-gray-800 shadow rounded-lg overflow-hidden">
                                <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                    <thead class="bg-gray-50 dark:bg-gray-700">
                                        <tr>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Name"</th>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Date"</th>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Description"</th>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Actions"</th>
                                        </tr>
                                    </thead>
                                    <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                                        {move || holidays.get().into_iter().map(|holiday| {
                                            let holiday_id = holiday.id.to_string();
                                            view! {
                                                <tr class="hover:bg-gray-50 dark:hover:bg-gray-700">
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900 dark:text-white">
                                                        {holiday.name.clone()}
                                                    </td>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                        {holiday.date.clone()}
                                                    </td>
                                                    <td class="px-6 py-4 text-sm text-gray-500 dark:text-gray-400">
                                                        {holiday.description.clone().unwrap_or_else(|| "-".to_string())}
                                                    </td>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                        <div class="flex items-center space-x-2">
                                                            <button
                                                                class="text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
                                                                on:click={
                                                                    let h = holiday.clone();
                                                                    move |_| {
                                                                        set_editing_holiday.set(Some(h.clone()));
                                                                        set_show_form.set(true);
                                                                    }
                                                                }
                                                            >
                                                                "Edit"
                                                            </button>
                                                            {move || {
                                                                let is_deleting = deleting_id.get() == Some(holiday_id.clone());
                                                                view! {
                                                                    <button
                                                                        class="text-red-600 hover:text-red-800 dark:text-red-400 dark:hover:text-red-300 disabled:opacity-50 disabled:cursor-not-allowed"
                                                                        disabled=is_deleting
                                                                        on:click={
                                                                            let id = holiday_id.clone();
                                                                            move |_| {
                                                                                let id_clone = id.clone();
                                                                                set_deleting_id.set(Some(id_clone.clone()));
                                                                                spawn_local(async move {
                                                                                    set_error.set(None);
                                                                                    
                                                                                    match delete_holiday(id_clone).await {
                                                                                        Ok(_) => {
                                                                                            match fetch_holidays().await {
                                                                                                Ok(data) => set_holidays.set(data),
                                                                                                Err(e) => set_error.set(Some(e)),
                                                                                            }
                                                                                        }
                                                                                        Err(e) => set_error.set(Some(e)),
                                                                                    }
                                                                                    set_deleting_id.set(None);
                                                                                });
                                                                            }
                                                                        }
                                                                    >
                                                                        {if is_deleting { "Deleting..." } else { "Delete" }}
                                                                    </button>
                                                                }
                                                            }}
                                                        </div>
                                                    </td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </div>
                        }.into_view()
                    }
                }}
            </main>
            
            <Footer/>
        </div>
    }
}

/// Fetch all holidays from API
async fn fetch_holidays() -> Result<Vec<Holiday>, String> {
    let response = reqwest::get("http://localhost:3000/api/v1/holidays")
        .await
        .map_err(|e| format!("Failed to fetch holidays: {}", e))?;
    
    if response.status().is_success() {
        response.json::<Vec<Holiday>>()
            .await
            .map_err(|e| format!("Failed to parse holidays: {}", e))
    } else {
        Err(format!("Failed to fetch holidays: {}", response.status()))
    }
}

/// Create a new holiday
async fn create_holiday(form_data: HolidayFormData) -> Result<(), String> {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:3000/api/v1/holidays")
        .json(&form_data)
        .send()
        .await
        .map_err(|e| format!("Failed to create holiday: {}", e))?;
    
    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to create holiday: {}", error_text))
    }
}

/// Update a holiday
async fn update_holiday(holiday_id: String, form_data: HolidayFormData) -> Result<(), String> {
    let client = reqwest::Client::new();
    let response = client
        .put(&format!("http://localhost:3000/api/v1/holidays/{}", holiday_id))
        .json(&form_data)
        .send()
        .await
        .map_err(|e| format!("Failed to update holiday: {}", e))?;
    
    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to update holiday: {}", error_text))
    }
}

/// Delete a holiday
async fn delete_holiday(holiday_id: String) -> Result<(), String> {
    let client = reqwest::Client::new();
    let response = client
        .delete(&format!("http://localhost:3000/api/v1/holidays/{}", holiday_id))
        .send()
        .await
        .map_err(|e| format!("Failed to delete holiday: {}", e))?;
    
    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to delete holiday: {}", error_text))
    }
}