use crate::auth::{
    authenticated_delete, authenticated_get, authenticated_post_json, authenticated_put_json,
};
use crate::components::{HolidayForm, HolidayFormData};
use leptos::either::{Either, EitherOf3};
use leptos::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

/// Holiday data structure
#[derive(Debug, Clone, Deserialize)]
pub struct Holiday {
    pub id: Uuid,
    pub name: String,
    pub date: String,
    pub description: Option<String>,
}

/// Holidays content component (without header/footer)
#[component]
pub fn HolidaysContent() -> impl IntoView {
    let (holidays, set_holidays) = signal(Vec::<Holiday>::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (show_form, set_show_form) = signal(false);
    let (editing_holiday, set_editing_holiday) = signal(Option::<Holiday>::None);
    let (form_submitting, set_form_submitting) = signal(false);
    let (deleting_id, set_deleting_id) = signal(Option::<String>::None);

    // Load holidays on mount
    Effect::new(move |_| {
        set_loading.set(true);
        leptos::task::spawn_local(async move {
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
        leptos::task::spawn_local(async move {
            set_form_submitting.set(true);
            set_error.set(None);

            let result = if let Some(holiday_id) = editing_id {
                update_holiday(holiday_id.to_string(), form_data).await
            } else {
                create_holiday(form_data).await
            };

            match result {
                Ok(_) => match fetch_holidays().await {
                    Ok(data) => {
                        set_holidays.set(data);
                        set_show_form.set(false);
                        set_editing_holiday.set(None);
                    }
                    Err(e) => set_error.set(Some(e)),
                },
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
        <div class="space-y-4">
            <div class="page-header">
                <div>
                    <h1 class="text-xl font-semibold text-huly-caption">
                        "Holiday Management"
                    </h1>
                    <p class="text-huly-secondary mt-1">
                        "Manage company holidays and days off"
                    </p>
                </div>

                <div class="flex items-center gap-2">
                    <button
                        class="btn-primary btn-press"
                        on:click=move |_| {
                            set_editing_holiday.set(None);
                            set_show_form.set(true);
                        }
                    >
                        "Add Holiday"
                    </button>
                </div>
            </div>

            {move || error.get().map(|err| {
                view! {
                    <div class="alert-error mb-6">
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

            {move || {
                if show_form.get() {
                    let is_edit = editing_holiday.get().is_some();
                    let title = if is_edit { "Edit Holiday" } else { "Create Holiday" };
                    let edit_data = editing_holiday.get().map(|h| HolidayFormData {
                        name: h.name,
                        date: h.date,
                        description: h.description.unwrap_or_default(),
                    });
                    Either::Left(view! {
                        <div class="card mb-4 relative">
                            <h2 class="text-xl font-semibold text-huly-caption mb-4">
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
                                    Either::Left(view! {
                                        <div class="absolute inset-0 flex items-center justify-center bg-huly-back/70 rounded-lg">
                                            <div class="text-center">
                                                <div class="spinner mx-auto mb-2"></div>
                                                <p class="text-sm text-huly-secondary">"Saving..."</p>
                                            </div>
                                        </div>
                                    })
                                } else {
                                    Either::Right(view! { <div></div> })
                                }
                            }}
                        </div>
                    })
                } else {
                    Either::Right(view! { <div></div> })
                }
            }}

            {move || {
                if loading.get() {
                    EitherOf3::A(view! {
                        <div class="space-y-3">
                            <div class="toolbar"><div class="skeleton-text w-32 h-3"></div></div>
                            <div class="panel overflow-hidden">
                                <div class="skeleton-row"><div class="skeleton-text w-28"></div><div class="skeleton-text w-24"></div><div class="skeleton-text w-20"></div></div>
                                <div class="skeleton-row"><div class="skeleton-text w-24"></div><div class="skeleton-text w-20"></div><div class="skeleton-text w-16"></div></div>
                                <div class="skeleton-row"><div class="skeleton-text w-32"></div><div class="skeleton-text w-16"></div><div class="skeleton-text w-24"></div></div>
                            </div>
                        </div>
                    })
                } else if holidays.get().is_empty() {
                    EitherOf3::B(view! {
                        <div class="empty-state py-12">
                            <svg class="w-12 h-12 text-huly-ghost mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M6.75 3v2.25M17.25 3v2.25M3 18.75V7.5a2.25 2.25 0 012.25-2.25h13.5A2.25 2.25 0 0121 7.5v11.25m-18 0A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75m-18 0v-7.5A2.25 2.25 0 015.25 9h13.5A2.25 2.25 0 0121 11.25v7.5m-9-6h.008v.008H12v-.008zM12 15h.008v.008H12V15zm0 2.25h.008v.008H12v-.008zM9.75 15h.008v.008H9.75V15zm0 2.25h.008v.008H9.75v-.008zM7.5 15h.008v.008H7.5V15zm0 2.25h.008v.008H7.5v-.008zm6.75-4.5h.008v.008h-.008v-.008zm0 2.25h.008v.008h-.008V15zm0 2.25h.008v.008h-.008v-.008zm2.25-4.5h.008v.008H16.5v-.008zm0 2.25h.008v.008H16.5V15z" />
                            </svg>
                            <p class="text-huly-secondary text-sm">"No holidays found."</p>
                            <p class="text-huly-muted text-xs mt-1">"Click 'Add Holiday' to create one."</p>
                        </div>
                    })
                } else {
                    EitherOf3::C(view! {
                        <div class="panel">
                            <div class="toolbar">
                                <h2 class="text-sm font-semibold text-huly-caption">"Holiday Management"</h2>
                            </div>
                            <table class="min-w-full divide-y divide-huly-divider">
                                <thead class="bg-huly-surface-2">
                                    <tr>
                                        <th class="th-cell-compact">"Name"</th>
                                        <th class="th-cell-compact">"Date"</th>
                                        <th class="th-cell-compact">"Description"</th>
                                        <th class="th-cell-compact">"Actions"</th>
                                    </tr>
                                </thead>
                                <tbody class="bg-huly-surface divide-y divide-huly-divider">
                                    {move || holidays.get().into_iter().map(|holiday| {
                                        let holiday_id = holiday.id.to_string();
                                        view! {
                                            <tr class="table-row-hover">
                                                <td class="td-cell-compact whitespace-nowrap font-medium text-huly-caption">
                                                    {holiday.name.clone()}
                                                </td>
                                                <td class="td-cell-compact whitespace-nowrap text-huly-muted">
                                                    {holiday.date.clone()}
                                                </td>
                                                <td class="td-cell-compact text-huly-muted">
                                                    {holiday.description.clone().unwrap_or_else(|| "-".to_string())}
                                                </td>
                                                <td class="td-cell-compact whitespace-nowrap text-huly-muted">
                                                    <div class="flex items-center gap-2">
                                                        <button
                                                            class="link"
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
                                                                    class="link-danger disabled:opacity-50 disabled:cursor-not-allowed"
                                                                    disabled=is_deleting
                                                                    on:click={
                                                                        let id = holiday_id.clone();
                                                                        move |_| {
                                                                            let id_clone = id.clone();
                                                                            set_deleting_id.set(Some(id_clone.clone()));
                                                                            leptos::task::spawn_local(async move {
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
                    })
                }
            }}
        </div>
    }
}

/// Fetch all holidays from API
async fn fetch_holidays() -> Result<Vec<Holiday>, String> {
    let response = authenticated_get("/api/v1/holidays")
        .await
        .map_err(|e| format!("Failed to fetch holidays: {}", e))?;

    if response.status().is_success() {
        response
            .json::<Vec<Holiday>>()
            .await
            .map_err(|e| format!("Failed to parse holidays: {}", e))
    } else {
        Err(format!("Failed to fetch holidays: {}", response.status()))
    }
}

/// Create a new holiday
async fn create_holiday(form_data: HolidayFormData) -> Result<(), String> {
    let response = authenticated_post_json("/api/v1/holidays", &form_data)
        .await
        .map_err(|e| format!("Failed to create holiday: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to create holiday: {}", error_text))
    }
}

/// Update a holiday
async fn update_holiday(holiday_id: String, form_data: HolidayFormData) -> Result<(), String> {
    let response = authenticated_put_json(
        &format!("/api/v1/holidays/{}", holiday_id),
        &form_data,
    )
    .await
    .map_err(|e| format!("Failed to update holiday: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to update holiday: {}", error_text))
    }
}

/// Delete a holiday
async fn delete_holiday(holiday_id: String) -> Result<(), String> {
    let response = authenticated_delete(&format!(
        "/api/v1/holidays/{}",
        holiday_id
    ))
    .await
    .map_err(|e| format!("Failed to delete holiday: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to delete holiday: {}", error_text))
    }
}
