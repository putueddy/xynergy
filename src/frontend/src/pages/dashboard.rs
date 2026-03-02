use crate::auth::{authenticated_get, logout_user, use_auth};
use crate::components::{Footer, Header};
use chrono::NaiveDate;
use leptos::*;
use leptos_router::*;
use serde::Deserialize;

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
    let (resources_count, set_resources_count) = create_signal(0usize);
    let (active_projects_count, set_active_projects_count) = create_signal(0usize);
    let (allocations_count, set_allocations_count) = create_signal(0usize);
    let (upcoming_deadlines, set_upcoming_deadlines) = create_signal(Vec::<ProjectSummary>::new());
    let (recent_activity, set_recent_activity) = create_signal(Vec::<AuditLogEntry>::new());
    let (dashboard_error, set_dashboard_error) = create_signal(Option::<String>::None);
    let (loading, set_loading) = create_signal(false);

    create_effect(move |_| {
        set_loading.set(true);
        let navigate = navigate.clone();
        spawn_local(async move {
            let mut had_error = None;
            let mut session_expired = false;

            match fetch_resources_count().await {
                Ok(count) => set_resources_count.set(count),
                Err(e) => {
                    if e == "SESSION_EXPIRED" {
                        session_expired = true;
                    }
                    had_error = Some(e)
                }
            }

            match fetch_allocations_count().await {
                Ok(count) => set_allocations_count.set(count),
                Err(e) => {
                    if e == "SESSION_EXPIRED" {
                        session_expired = true;
                    }
                    had_error = Some(e)
                }
            }

            match fetch_projects().await {
                Ok(projects) => {
                    let active = projects.iter().filter(|p| p.status == "active").count();
                    set_active_projects_count.set(active);

                    let today = chrono::Local::now().date_naive();
                    let mut upcoming: Vec<ProjectSummary> = projects
                        .into_iter()
                        .filter(|p| parse_date(&p.end_date).map(|d| d >= today).unwrap_or(false))
                        .collect();
                    upcoming.sort_by(|a, b| a.end_date.cmp(&b.end_date));
                    upcoming.truncate(5);
                    set_upcoming_deadlines.set(upcoming);
                }
                Err(e) => {
                    if e == "SESSION_EXPIRED" {
                        session_expired = true;
                    }
                    had_error = Some(e)
                }
            }

            match fetch_audit_logs().await {
                Ok(entries) => set_recent_activity.set(entries),
                Err(e) => {
                    if e == "SESSION_EXPIRED" {
                        session_expired = true;
                    }
                    had_error = Some(e)
                }
            }

            if session_expired {
                logout_user(&auth);
                set_dashboard_error.set(Some(
                    "Your session expired. Please sign in again.".to_string(),
                ));
                navigate("/login", Default::default());
                set_loading.set(false);
                return;
            }

            set_dashboard_error.set(had_error);
            set_loading.set(false);
        });
    });

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
                                    <p class="text-2xl font-semibold text-gray-900 dark:text-white">
                                        {move || resources_count.get().to_string()}
                                    </p>
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
                                    <h3 class="text-lg font-medium text-gray-900 dark:text-white">"Active Projects"</h3>
                                    <p class="text-2xl font-semibold text-gray-900 dark:text-white">
                                        {move || active_projects_count.get().to_string()}
                                    </p>
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
                                    <p class="text-2xl font-semibold text-gray-900 dark:text-white">
                                        {move || allocations_count.get().to_string()}
                                    </p>
                                </div>
                            </div>
                        </div>
                    </div>

                    {move || dashboard_error.get().map(|err| {
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

                    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                        <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6">
                            <div class="flex items-center justify-between mb-4">
                                <h3 class="text-lg font-medium text-gray-900 dark:text-white">"Upcoming Deadlines"</h3>
                                {move || {
                                    if loading.get() {
                                        view! { <span class="text-sm text-gray-500">"Loading..."</span> }.into_view()
                                    } else {
                                        view! { <span></span> }.into_view()
                                    }
                                }}
                            </div>
                            {move || {
                                let items = upcoming_deadlines.get();
                                if items.is_empty() {
                                    view! { <p class="text-sm text-gray-500 dark:text-gray-400">"No upcoming deadlines."</p> }.into_view()
                                } else {
                                    view! {
                                        <ul class="space-y-3">
                                            {items.into_iter().map(|p| {
                                                view! {
                                                    <li class="flex items-center justify-between">
                                                        <span class="text-sm font-medium text-gray-900 dark:text-white">{p.name}</span>
                                                        <span class="text-sm text-gray-500 dark:text-gray-400">{p.end_date}</span>
                                                    </li>
                                                }
                                            }).collect_view()}
                                        </ul>
                                    }.into_view()
                                }
                            }}
                        </div>

                        <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6">
                            <div class="flex items-center justify-between mb-4">
                                <h3 class="text-lg font-medium text-gray-900 dark:text-white">"Recent Activity"</h3>
                                {move || {
                                    if loading.get() {
                                        view! { <span class="text-sm text-gray-500">"Loading..."</span> }.into_view()
                                    } else {
                                        view! { <span></span> }.into_view()
                                    }
                                }}
                            </div>
                            {move || {
                                let items = recent_activity.get();
                                if items.is_empty() {
                                    view! { <p class="text-sm text-gray-500 dark:text-gray-400">"No recent activity."</p> }.into_view()
                                } else {
                                    view! {
                                        <ul class="space-y-3">
                                            {items.into_iter().map(|entry| {
                                                let user_label = entry.user_name.unwrap_or_else(|| "System".to_string());
                                                let date_label = entry
                                                    .created_at
                                                    .split('T')
                                                    .next()
                                                    .unwrap_or(&entry.created_at)
                                                    .to_string();
                                                let action_label = format!("{} {}", entry.action, entry.entity_type);
                                                view! {
                                                    <li class="flex items-center justify-between">
                                                        <span class="text-sm text-gray-700 dark:text-gray-300">
                                                            {format!("{} {}", user_label, action_label)}
                                                        </span>
                                                        <span class="text-xs text-gray-500 dark:text-gray-400">{date_label}</span>
                                                    </li>
                                                }
                                            }).collect_view()}
                                        </ul>
                                    }.into_view()
                                }
                            }}
                        </div>
                    </div>
                </div>
            </main>

            <Footer/>
        </div>
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ProjectSummary {
    name: String,
    end_date: String,
    status: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AuditLogEntry {
    user_name: Option<String>,
    action: String,
    entity_type: String,
    created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AuditLogsEnvelope {
    entries: Vec<AuditLogEntry>,
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

async fn fetch_resources_count() -> Result<usize, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/resources")
        .await
        .map_err(|e| {
            if e == "SESSION_EXPIRED" {
                e
            } else {
                format!("Failed to fetch resources: {}", e)
            }
        })?;

    if response.status().is_success() {
        let items: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse resources: {}", e))?;
        Ok(items.len())
    } else {
        Err(format!("Failed to fetch resources: {}", response.status()))
    }
}

async fn fetch_allocations_count() -> Result<usize, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/allocations")
        .await
        .map_err(|e| {
            if e == "SESSION_EXPIRED" {
                e
            } else {
                format!("Failed to fetch allocations: {}", e)
            }
        })?;

    if response.status().is_success() {
        let items: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse allocations: {}", e))?;
        Ok(items.len())
    } else {
        Err(format!(
            "Failed to fetch allocations: {}",
            response.status()
        ))
    }
}

async fn fetch_projects() -> Result<Vec<ProjectSummary>, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/projects")
        .await
        .map_err(|e| {
            if e == "SESSION_EXPIRED" {
                e
            } else {
                format!("Failed to fetch projects: {}", e)
            }
        })?;

    if response.status().is_success() {
        response
            .json::<Vec<ProjectSummary>>()
            .await
            .map_err(|e| format!("Failed to parse projects: {}", e))
    } else {
        Err(format!("Failed to fetch projects: {}", response.status()))
    }
}

async fn fetch_audit_logs() -> Result<Vec<AuditLogEntry>, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/audit-logs?limit=10")
        .await
        .map_err(|e| {
            if e == "SESSION_EXPIRED" {
                e
            } else {
                format!("Failed to fetch audit logs: {}", e)
            }
        })?;

    if response.status().is_success() {
        let body = response
            .json::<AuditLogsEnvelope>()
            .await
            .map_err(|e| format!("Failed to parse audit logs: {}", e))?;
        Ok(body.entries)
    } else if response.status() == reqwest::StatusCode::FORBIDDEN {
        Ok(Vec::new())
    } else {
        Err(format!("Failed to fetch audit logs: {}", response.status()))
    }
}
