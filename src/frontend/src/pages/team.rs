use crate::auth::{
    AuthContext, authenticated_get, auth_token, clear_auth_storage, use_auth, validate_token,
};
use crate::components::{Footer, Header};
use leptos::*;
use leptos_router::*;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
struct AssignmentInfo {
    project_name: String,
    allocation_pct: f64,
    start_date: String,
    end_date: String,
}

#[derive(Clone, Debug, PartialEq)]
struct TeamMember {
    resource_id: String,
    name: String,
    role: String,
    department_name: String,
    daily_rate: Option<i64>,
    ctc_status: String,
    total_allocation_pct: f64,
    active_assignments: Vec<AssignmentInfo>,
}

fn current_access_token(auth: &AuthContext) -> Option<String> {
    if let Ok(stored) = auth_token() {
        if auth.token.get() != Some(stored.clone()) {
            auth.token.set(Some(stored.clone()));
        }
        return Some(stored);
    }
    auth.token.get()
}

fn value_to_i64(value: &Value) -> Option<i64> {
    if let Some(v) = value.as_i64() {
        return Some(v);
    }
    value.as_str()?.parse::<i64>().ok()
}

fn value_to_f64(value: &Value) -> Option<f64> {
    if let Some(v) = value.as_f64() {
        return Some(v);
    }
    if let Some(v) = value.as_i64() {
        return Some(v as f64);
    }
    value.as_str()?.parse::<f64>().ok()
}

async fn fetch_team_members() -> Result<Vec<TeamMember>, String> {
    let response = authenticated_get("http://localhost:3000/api/v1/team")
        .await
        .map_err(|e| format!("Failed to fetch team: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Failed to fetch team: {}", response.status()));
    }
    
    let body: Value = response.json().await
        .map_err(|e| format!("Failed to parse team response: {}", e))?;
    
    let arr = body.as_array().cloned().unwrap_or_default();
    
    let mut members = Vec::new();
    for item in arr {
        let active_assignments = item.get("active_assignments")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().map(|a| AssignmentInfo {
                    project_name: a.get("project_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    allocation_pct: a.get("allocation_pct").and_then(value_to_f64).unwrap_or(0.0),
                    start_date: a.get("start_date").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    end_date: a.get("end_date").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                }).collect()
            })
            .unwrap_or_default();

        members.push(TeamMember {
            resource_id: item.get("resource_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            name: item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            role: item.get("role").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            department_name: item.get("department_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            daily_rate: item.get("daily_rate").and_then(value_to_i64),
            ctc_status: item.get("ctc_status").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            total_allocation_pct: item.get("total_allocation_pct").and_then(value_to_f64).unwrap_or(0.0),
            active_assignments,
        });
    }
    
    Ok(members)
}

fn format_idr(amount: i64) -> String {
    let s = amount.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    format!("Rp {}", result.chars().rev().collect::<String>())
}

fn allocation_color(pct: f64) -> &'static str {
    if pct > 100.0 {
        "text-red-800 dark:text-red-300 font-bold"
    } else if pct >= 100.0 {
        "text-red-600 dark:text-red-400"
    } else if pct >= 81.0 {
        "text-yellow-600 dark:text-yellow-400"
    } else {
        "text-green-600 dark:text-green-400"
    }
}

#[component]
pub fn TeamPage() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();
    let (auth_checked, set_auth_checked) = create_signal(false);
    let (auth_check_in_progress, set_auth_check_in_progress) = create_signal(false);

    let (team_members, set_team_members) = create_signal(Vec::<TeamMember>::new());
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(None::<String>);
    let (sort_by, set_sort_by) = create_signal("name".to_string());
    let (filter_status, set_filter_status) = create_signal("all".to_string());

    {
        let navigate = navigate.clone();
        create_effect(move |_| {
            if !auth.is_authenticated.get() {
                navigate("/login", Default::default());
                return;
            }

            if let Some(user) = auth.user.get() {
                set_auth_checked.set(true);
                if user.role != "hr" && user.role != "department_head" && user.role != "admin" {
                    navigate("/dashboard", Default::default());
                }
                return;
            }

            if auth_check_in_progress.get() {
                return;
            }

            let token = match current_access_token(&auth) {
                Some(t) => t,
                None => {
                    navigate("/login", Default::default());
                    return;
                }
            };

            set_auth_check_in_progress.set(true);
            let navigate = navigate.clone();
            spawn_local(async move {
                match validate_token(token).await {
                    Ok(user) => {
                        auth.user.set(Some(user));
                    }
                    Err(_) => {
                        auth.user.set(None);
                        auth.token.set(None);
                        auth.refresh_token.set(None);
                        clear_auth_storage();
                        navigate("/login", Default::default());
                    }
                }
                set_auth_checked.set(true);
                set_auth_check_in_progress.set(false);
            });
        });
    }

    let is_authorized = Signal::derive(move || {
        auth.user
            .get()
            .map(|u| u.role == "hr" || u.role == "department_head" || u.role == "admin")
            .unwrap_or(false)
    });

    create_effect(move |_| {
        if auth.token.get().is_some() {
            if !is_authorized.get() {
                return;
            }
            set_loading.set(true);
            spawn_local(async move {
                match fetch_team_members().await {
                    Ok(members) => {
                        set_team_members.set(members);
                        set_error.set(None);
                    }
                    Err(e) => set_error.set(Some(e)),
                }
                set_loading.set(false);
            });
        }
    });

    let filtered_and_sorted = Signal::derive(move || {
        let members = team_members.get();
        let filter = filter_status.get();
        let sort = sort_by.get();
        
        let mut filtered: Vec<TeamMember> = members.into_iter().filter(|m| {
            match filter.as_str() {
                "active" => m.ctc_status == "Active",
                "missing" => m.ctc_status == "Missing",
                _ => true,
            }
        }).collect();
        
        match sort.as_str() {
            "rate" => filtered.sort_by(|a, b| b.daily_rate.unwrap_or(0).cmp(&a.daily_rate.unwrap_or(0))),
            "allocation" => filtered.sort_by(|a, b| b.total_allocation_pct.partial_cmp(&a.total_allocation_pct).unwrap_or(std::cmp::Ordering::Equal)),
            "status" => filtered.sort_by(|a, b| a.ctc_status.cmp(&b.ctc_status)),
            _ => filtered.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        }
        
        filtered
    });

    let total_members = Signal::derive(move || team_members.get().len());
    
    let avg_daily_rate = Signal::derive(move || {
        let members = team_members.get();
        let mut sum = 0;
        let mut count = 0;
        for m in members {
            if let Some(rate) = m.daily_rate {
                sum += rate;
                count += 1;
            }
        }
        if count > 0 { sum / count } else { 0 }
    });

    let avg_allocation = Signal::derive(move || {
        let members = team_members.get();
        if members.is_empty() { return 0.0; }
        let sum: f64 = members.iter().map(|m| m.total_allocation_pct).sum();
        sum / members.len() as f64
    });

    let missing_ctc_count = Signal::derive(move || {
        team_members.get().iter().filter(|m| m.ctc_status == "Missing").count()
    });

    view! {
        <div class="min-h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
            <Header/>

            <main class="flex-grow max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12 w-full">
                {move || {
                    if !auth_checked.get() {
                        return view! {
                            <div class="rounded-md bg-blue-50 p-4 dark:bg-blue-900/20 text-blue-800 dark:text-blue-200">
                                "Checking access..."
                            </div>
                        }.into_view();
                    }

                    if !is_authorized.get() {
                        return view! {
                            <div class="rounded-md bg-red-50 p-4 dark:bg-red-900/20 text-red-800 dark:text-red-200">
                                "Access denied."
                            </div>
                        }.into_view();
                    }

                    view! {
                        <div class="space-y-8">
                            <div class="flex items-center justify-between">
                                <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                                    "My Team"
                                </h1>
                            </div>

                            {move || error.get().map(|err| view! {
                                <div class="rounded-md bg-red-50 p-4 dark:bg-red-900/20 text-red-800 dark:text-red-200">{err}</div>
                            })}

                            <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                                <div class="p-4 bg-white dark:bg-gray-800 shadow rounded-lg">
                                    <div class="text-sm text-gray-500 dark:text-gray-400">"Total Team Members"</div>
                                    <div class="text-2xl font-bold text-gray-900 dark:text-white">{move || total_members.get()}</div>
                                </div>
                                <div class="p-4 bg-white dark:bg-gray-800 shadow rounded-lg">
                                    <div class="text-sm text-gray-500 dark:text-gray-400">"Avg Daily Rate"</div>
                                    <div class="text-2xl font-bold text-gray-900 dark:text-white font-mono">{move || format_idr(avg_daily_rate.get())}</div>
                                </div>
                                <div class="p-4 bg-white dark:bg-gray-800 shadow rounded-lg">
                                    <div class="text-sm text-gray-500 dark:text-gray-400">"Avg Allocation %"</div>
                                    <div class="text-2xl font-bold text-gray-900 dark:text-white">{move || format!("{:.1}%", avg_allocation.get())}</div>
                                </div>
                                <div class="p-4 bg-white dark:bg-gray-800 shadow rounded-lg">
                                    <div class="text-sm text-gray-500 dark:text-gray-400">"CTC Missing"</div>
                                    <div class="text-2xl font-bold text-red-600 dark:text-red-400">{move || missing_ctc_count.get()}</div>
                                </div>
                            </div>

                            <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 space-y-4">
                                <div class="flex items-center gap-4">
                                    <div class="flex items-center gap-2">
                                        <label class="text-sm font-medium text-gray-700 dark:text-gray-300">"Filter Status:"</label>
                                        <select
                                            class="border rounded px-3 py-2 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                                            prop:value=filter_status
                                            on:change=move |ev| set_filter_status.set(event_target_value(&ev))
                                        >
                                            <option value="all">"All"</option>
                                            <option value="active">"Active"</option>
                                            <option value="missing">"CTC Missing"</option>
                                        </select>
                                    </div>
                                    <div class="flex items-center gap-2">
                                        <label class="text-sm font-medium text-gray-700 dark:text-gray-300">"Sort By:"</label>
                                        <select
                                            class="border rounded px-3 py-2 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                                            prop:value=sort_by
                                            on:change=move |ev| set_sort_by.set(event_target_value(&ev))
                                        >
                                            <option value="name">"Name"</option>
                                            <option value="rate">"Daily Rate"</option>
                                            <option value="allocation">"Allocation %"</option>
                                            <option value="status">"Status"</option>
                                        </select>
                                    </div>
                                    {move || loading.get().then(|| view! {
                                        <div class="text-sm text-gray-500">"Loading..."</div>
                                    })}
                                </div>

                                <div class="mt-4 overflow-x-auto">
                                    <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                        <thead class="bg-gray-50 dark:bg-gray-700">
                                            <tr>
                                                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Name"</th>
                                                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Role"</th>
                                                <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">"Daily Rate"</th>
                                                <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">"Current Allocation %"</th>
                                                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Projects"</th>
                                                <th class="px-4 py-3 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">"Status"</th>
                                                <th class="px-4 py-3 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">"Actions"</th>
                                            </tr>
                                        </thead>
                                        <tbody class="divide-y divide-gray-200 dark:divide-gray-700 bg-white dark:bg-gray-800">
                                            <For
                                                each=move || filtered_and_sorted.get()
                                                key=|m| m.resource_id.clone()
                                                children=move |m| {
                                                    let is_missing = m.ctc_status == "Missing";
                                                    let row_class = if is_missing {
                                                        "text-gray-400 bg-gray-50/50 dark:bg-gray-800/50"
                                                    } else {
                                                        "text-gray-900 dark:text-gray-100"
                                                    };
                                                    
                                                    let rate_text = match m.daily_rate {
                                                        Some(rate) => format_idr(rate),
                                                        None => "—".to_string(),
                                                    };

                                                    let alloc_class = if is_missing {
                                                        "text-gray-400 font-mono text-right".to_string()
                                                    } else {
                                                        format!("font-mono text-right {}", allocation_color(m.total_allocation_pct))
                                                    };
                                                    
                                                    let projects_tooltip = m.active_assignments.iter()
                                                        .map(|a| format!("{}: {} to {} ({:.0}%)", a.project_name, a.start_date, a.end_date, a.allocation_pct))
                                                        .collect::<Vec<_>>()
                                                        .join("\n");

                                                    let projects_text = m.active_assignments.iter()
                                                        .map(|a| format!("{} ({:.0}%)", a.project_name, a.allocation_pct))
                                                        .collect::<Vec<_>>()
                                                        .join(", ");
                                                        
                                                    let projects_display = if projects_text.is_empty() {
                                                        "—".to_string()
                                                    } else {
                                                        projects_text
                                                    };

                                                    let projects_title = if projects_tooltip.is_empty() {
                                                        None
                                                    } else {
                                                        Some(projects_tooltip)
                                                    };

                                                    let status_badge = if is_missing {
                                                        "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300 border-red-200"
                                                    } else {
                                                        "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300 border-green-200"
                                                    };
                                                    
                                                    let display_status = if is_missing {
                                                        "CTC Missing"
                                                    } else {
                                                        "Active"
                                                    };

                                                    view! {
                                                        <tr class=row_class>
                                                            <td class="px-4 py-3 text-sm font-medium">{m.name.clone()}</td>
                                                            <td class="px-4 py-3 text-sm">{m.role.clone()}</td>
                                                            <td class="px-4 py-3 text-sm font-mono text-right">{rate_text}</td>
                                                            <td class=format!("px-4 py-3 text-sm {}", alloc_class)>{format!("{:.1}%", m.total_allocation_pct)}</td>
                                                            <td class="px-4 py-3 text-sm" title=projects_title>{projects_display}</td>
                                                            <td class="px-4 py-3 text-sm text-center">
                                                                <span class=format!("px-2 py-1 inline-flex text-xs leading-5 font-semibold rounded-full border {}", status_badge)>
                                                                    {display_status}
                                                                </span>
                                                            </td>
                                                            <td class="px-4 py-3 text-sm text-center">
                                                                {if is_missing {
                                                                    view! {
                                                                        <button
                                                                            disabled=true
                                                                            title="CTC data required \u{2014} contact HR"
                                                                            class="px-3 py-1 text-xs font-medium rounded bg-gray-200 text-gray-400 cursor-not-allowed dark:bg-gray-700 dark:text-gray-500"
                                                                        >
                                                                            "Assign"
                                                                        </button>
                                                                    }.into_view()
                                                                } else {
                                                                    view! {
                                                                        <button
                                                                            class="px-3 py-1 text-xs font-medium rounded bg-blue-600 text-white hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600"
                                                                        >
                                                                            "Assign"
                                                                        </button>
                                                                    }.into_view()
                                                                }}
                                                            </td>
                                                        </tr>
                                                    }
                                                }
                                            />
                                            {move || filtered_and_sorted.get().is_empty().then(|| view! {
                                                <tr>
                                                    <td colspan="7" class="px-4 py-8 text-center text-sm text-gray-500">
                                                        "No team members found."
                                                    </td>
                                                </tr>
                                            })}
                                        </tbody>
                                    </table>
                                </div>
                            </div>
                        </div>
                    }.into_view()
                }}
            </main>
            <Footer/>
        </div>
    }
}