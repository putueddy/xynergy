use crate::auth::{authenticated_get, authenticated_post_json, use_auth};
use gloo_timers::callback::Interval;
use leptos::either::{Either, EitherOf3};
use leptos::prelude::*;
use leptos_router::hooks::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
struct CashFlowEntry {
    entry_type: String,
    category: String,
    amount_idr: i64,
    entry_date: String,
    description: String,
    project_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProjectOption {
    id: Uuid,
    name: String,
}

#[derive(Debug, Clone, Serialize)]
struct CreateCashFlowEntryPayload {
    entry_type: String,
    category: String,
    amount_idr: i64,
    entry_date: String,
    description: String,
    project_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
struct CashFlowDashboardData {
    start_date: String,
    end_date: String,
    project_id: Option<Uuid>,
    total_cash_in_idr: i64,
    total_cash_out_idr: i64,
    net_cash_flow_idr: i64,
    ending_cumulative_position_idr: i64,
    months: Vec<DashboardMonth>,
}

#[derive(Debug, Clone, Deserialize)]
struct DashboardMonth {
    year: i32,
    month: u32,
    month_label: String,
    cash_in_idr: i64,
    cash_out_idr: i64,
    net_cash_flow_idr: i64,
    cumulative_position_idr: i64,
    entries: Vec<DashboardEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct DashboardEntry {
    id: Uuid,
    entry_type: String,
    category: String,
    amount_idr: i64,
    entry_date: String,
    description: String,
    project_id: Option<Uuid>,
}

fn category_options(entry_type: &str) -> Vec<(&'static str, &'static str)> {
    if entry_type == "cash_out" {
        vec![
            ("payroll", "Payroll"),
            ("vendor_payment", "Vendor Payment"),
            ("expense", "Expense"),
            ("tax", "Tax"),
        ]
    } else {
        vec![
            ("client_payment", "Client Payment"),
            ("interest", "Interest"),
            ("other_income", "Other Income"),
        ]
    }
}

fn category_label(category: &str) -> &'static str {
    match category {
        "client_payment" => "Client Payment",
        "interest" => "Interest",
        "other_income" => "Other Income",
        "payroll" => "Payroll",
        "vendor_payment" => "Vendor Payment",
        "expense" => "Expense",
        "tax" => "Tax",
        _ => "Unknown",
    }
}

fn format_idr(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut reversed_grouped = String::new();

    for (idx, ch) in digits.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            reversed_grouped.push('.');
        }
        reversed_grouped.push(ch);
    }

    let mut grouped: String = reversed_grouped.chars().rev().collect();
    if value < 0 {
        grouped = format!("-{}", grouped);
    }

    format!("Rp {}", grouped)
}

async fn fetch_cash_flow_entries(project_id: Option<&str>) -> Result<Vec<CashFlowEntry>, String> {
    let url = match project_id {
        Some(id) if !id.is_empty() => format!("/api/v1/cash-flow/entries?project_id={}", id),
        _ => "/api/v1/cash-flow/entries".to_string(),
    };

    let response = authenticated_get(&url)
        .await
        .map_err(|e| format!("Failed to fetch cash flow entries: {}", e))?;

    if response.status().is_success() {
        response
            .json::<Vec<CashFlowEntry>>()
            .await
            .map_err(|e| format!("Failed to parse cash flow entries: {}", e))
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Failed to fetch cash flow entries: {} {}", status, body))
    }
}

async fn fetch_projects() -> Result<Vec<ProjectOption>, String> {
    let response = authenticated_get("/api/v1/projects")
        .await
        .map_err(|e| format!("Failed to fetch projects: {}", e))?;

    if response.status().is_success() {
        response
            .json::<Vec<ProjectOption>>()
            .await
            .map_err(|e| format!("Failed to parse projects: {}", e))
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Failed to fetch projects: {} {}", status, body))
    }
}

async fn create_cash_flow_entry(payload: &CreateCashFlowEntryPayload) -> Result<(), String> {
    let response = authenticated_post_json("/api/v1/cash-flow/entries", payload)
        .await
        .map_err(|e| format!("Failed to create cash flow entry: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Failed to create cash flow entry: {} {}", status, body))
    }
}

async fn fetch_dashboard(
    start_date: &str,
    end_date: &str,
    project_id: Option<&str>,
) -> Result<CashFlowDashboardData, String> {
    let mut url = format!(
        "/api/v1/cash-flow/dashboard?start_date={}&end_date={}",
        start_date, end_date
    );
    if let Some(pid) = project_id {
        if !pid.is_empty() {
            url.push_str(&format!("&project_id={}", pid));
        }
    }

    let response = authenticated_get(&url)
        .await
        .map_err(|e| format!("Failed to fetch dashboard: {}", e))?;

    if response.status().is_success() {
        response
            .json::<CashFlowDashboardData>()
            .await
            .map_err(|e| format!("Failed to parse dashboard: {}", e))
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Failed to fetch dashboard: {} {}", status, body))
    }
}

#[component]
pub fn CashFlowPage() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();

    let user_role = Signal::derive(move || auth.user.get().map(|u| u.role).unwrap_or_default());
    let is_admin = Signal::derive(move || user_role.get() == "admin");
    let is_finance = Signal::derive(move || user_role.get() == "finance");
    let can_access = Signal::derive(move || is_finance.get() || is_admin.get());

    let (entries, set_entries) = signal(Vec::<CashFlowEntry>::new());
    let (projects, set_projects) = signal(Vec::<ProjectOption>::new());
    let (loading, set_loading) = signal(false);
    let (submitting, set_submitting) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (success, set_success) = signal(Option::<String>::None);

    let (entry_type, set_entry_type) = signal(String::from("cash_in"));
    let (category, set_category) = signal(String::from("client_payment"));
    let (amount_idr, set_amount_idr) = signal(String::new());
    let (entry_date, set_entry_date) = signal(chrono::Local::now().date_naive().to_string());
    let (description, set_description) = signal(String::new());
    let (project_id, set_project_id) = signal(String::new());
    let (list_project_filter, set_list_project_filter) = signal(String::new());

    let now = chrono::Local::now().date_naive();
    let default_start = format!("{}-01-01", now.format("%Y"));
    let default_end = format!("{}-12-31", now.format("%Y"));
    let (dash_start, set_dash_start) = signal(default_start);
    let (dash_end, set_dash_end) = signal(default_end);
    let (dash_project, set_dash_project) = signal(String::new());
    let (dash_reload_nonce, set_dash_reload_nonce) = signal(0u64);
    let (dashboard_snapshot, set_dashboard_snapshot) = signal(Option::<CashFlowDashboardData>::None);
    let (dash_loading, set_dash_loading) = signal(false);
    let (dash_error, set_dash_error) = signal(Option::<String>::None);
    let (expanded_month, set_expanded_month) = signal(Option::<(i32, u32)>::None);
    let (dash_hover_month, set_dash_hover_month) = signal(Option::<(i32, u32)>::None);

    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            if !auth.is_authenticated.get() {
                navigate("/login", Default::default());
            }
        });
    }

    Effect::new(move |_| {
        let current_type = entry_type.get();
        let options = category_options(&current_type);
        let current_category = category.get();
        if !options.iter().any(|(value, _)| *value == current_category) {
            set_category.set(options[0].0.to_string());
        }
    });

    let load_entries = move || {
        set_loading.set(true);
        set_error.set(None);
        leptos::task::spawn_local(async move {
            let selected_filter = list_project_filter.get();
            let entries_result = if selected_filter.is_empty() {
                fetch_cash_flow_entries(None).await
            } else {
                fetch_cash_flow_entries(Some(&selected_filter)).await
            };

            match entries_result {
                Ok(data) => set_entries.set(data),
                Err(e) => set_error.set(Some(e)),
            }

            set_loading.set(false);
        });
    };

    let dashboard_resource = LocalResource::new(move || async move {
        let start_date = dash_start.get();
        let end_date = dash_end.get();
        let project = dash_project.get();
        let _reload_nonce = dash_reload_nonce.get();

        if !auth.is_authenticated.get() || !can_access.get() {
            return None;
        }

        if start_date.trim().is_empty() || end_date.trim().is_empty() {
            return Some(Err("Start date and end date are required".to_string()));
        }

        let project_ref = if project.is_empty() {
            None
        } else {
            Some(project.as_str())
        };

        Some(fetch_dashboard(&start_date, &end_date, project_ref).await)
    });

    let dashboard_data = Signal::derive(move || {
        dashboard_resource
            .get()
            .and_then(|result| result)
            .and_then(|result| result.ok())
    });

    // Load projects once when authenticated and authorized
    Effect::new(move |_| {
        if auth.is_authenticated.get() && can_access.get() {
            leptos::task::spawn_local(async move {
                match fetch_projects().await {
                    Ok(data) => set_projects.set(data),
                    Err(e) => {
                        if error.get().is_none() {
                            set_error.set(Some(e));
                        }
                    }
                }
            });
        }
    });

    // Reload entries when project filter changes
    Effect::new(move |_| {
        let _ = list_project_filter.get();
        if auth.is_authenticated.get() && can_access.get() {
            load_entries();
        }
    });

    Effect::new(move |_| {
        if auth.is_authenticated.get() && can_access.get() {
            let _ = dash_start.get();
            let _ = dash_end.get();
            let _ = dash_project.get();
            let _ = dash_reload_nonce.get();
            set_dash_loading.set(true);
            set_dash_error.set(None);
            set_expanded_month.set(None);
            set_dash_hover_month.set(None);
        } else {
            set_dash_loading.set(false);
        }
    });

    Effect::new(move |_| {
        match dashboard_resource.get() {
            Some(Some(Ok(data))) => {
                set_dashboard_snapshot.set(Some(data));
                set_dash_loading.set(false);
                set_dash_error.set(None);
            }
            Some(Some(Err(err))) => {
                set_dash_loading.set(false);
                set_dash_error.set(Some(err));
            }
            Some(None) => {
                set_dash_loading.set(false);
            }
            None => {}
        }
    });

    Effect::new(move |_| {
        if auth.is_authenticated.get() && can_access.get() {
            let interval = Interval::new(30_000, move || {
                set_dash_reload_nonce.update(|value| *value += 1);
            });
            let _keep = StoredValue::new_local(Some(interval));
        }
    });

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        set_error.set(None);
        set_success.set(None);

        let amount = match amount_idr.get().trim().parse::<i64>() {
            Ok(value) if value > 0 => value,
            _ => {
                set_error.set(Some("Amount must be a positive integer".to_string()));
                return;
            }
        };

        let current_description = description.get();
        if current_description.trim().is_empty() {
            set_error.set(Some("Description is required".to_string()));
            return;
        }

        let selected_project_id = if project_id.get().is_empty() {
            None
        } else {
            match Uuid::parse_str(&project_id.get()) {
                Ok(value) => Some(value),
                Err(_) => {
                    set_error.set(Some("Invalid project selection".to_string()));
                    return;
                }
            }
        };

        let payload = CreateCashFlowEntryPayload {
            entry_type: entry_type.get(),
            category: category.get(),
            amount_idr: amount,
            entry_date: entry_date.get(),
            description: current_description,
            project_id: selected_project_id,
        };

        set_submitting.set(true);
        leptos::task::spawn_local(async move {
            match create_cash_flow_entry(&payload).await {
                Ok(_) => {
                    set_success.set(Some("Cash flow entry created successfully".to_string()));
                    set_amount_idr.set(String::new());
                    set_description.set(String::new());
                    set_project_id.set(String::new());
                    set_entry_date.set(chrono::Local::now().date_naive().to_string());

                    let selected_filter = list_project_filter.get();
                    let list_result = if selected_filter.is_empty() {
                        fetch_cash_flow_entries(None).await
                    } else {
                        fetch_cash_flow_entries(Some(&selected_filter)).await
                    };

                    match list_result {
                        Ok(data) => set_entries.set(data),
                        Err(e) => set_error.set(Some(e)),
                    }

                    set_dash_reload_nonce.update(|value| *value += 1);
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_submitting.set(false);
        });
    };

    view! {
        <div class="h-full">
            <div class="page-container fade-in">
                <div class="space-y-4">
                    <div class="page-header">
                        <h1 class="text-xl font-semibold text-huly-caption">"Cash Flow"</h1>
                    </div>

                    {move || {
                        if !can_access.get() {
                            Either::Left(view! {
                                <div class="alert-error">
                                    "Access denied. Cash flow is available to Finance and Admin roles only."
                                </div>
                            })
                        } else {
                            Either::Right(view! {
                                <div class="space-y-4">
                                    {move || error.get().map(|err| view! { <div class="alert-error">{err}</div> })}
                                    {move || success.get().map(|msg| view! { <div class="alert-success">{msg}</div> })}

                                    <div class="panel p-4">
                                        <h2 class="section-header mb-4">"Add Cash Entry"</h2>
                                        <form class="space-y-4" on:submit=handle_submit>
                                            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                                <div>
                                                    <label class="label">"Type"</label>
                                                    <select
                                                        class="input mt-1"
                                                        prop:value=entry_type
                                                        on:change=move |ev| set_entry_type.set(event_target_value(&ev))
                                                        required
                                                    >
                                                        <option value="cash_in">"Cash In"</option>
                                                        <option value="cash_out">"Cash Out"</option>
                                                    </select>
                                                </div>

                                                <div>
                                                    <label class="label">"Category"</label>
                                                    <select
                                                        class="input mt-1"
                                                        prop:value=category
                                                        on:change=move |ev| set_category.set(event_target_value(&ev))
                                                        required
                                                    >
                                                        {move || {
                                                            category_options(&entry_type.get())
                                                                .into_iter()
                                                                .map(|(value, label)| {
                                                                    view! { <option value=value>{label}</option> }
                                                                })
                                                                .collect_view()
                                                        }}
                                                    </select>
                                                </div>

                                                <div>
                                                    <label class="label">"Amount (IDR)"</label>
                                                    <input
                                                        type="text"
                                                        inputmode="numeric"
                                                        pattern="[0-9]*"
                                                        autocomplete="off"
                                                        class="input mt-1"
                                                        prop:value=amount_idr
                                                        on:input=move |ev| set_amount_idr.set(event_target_value(&ev))
                                                        required
                                                    />
                                                </div>

                                                <div>
                                                    <label class="label">"Date"</label>
                                                    <input
                                                        type="date"
                                                        class="input mt-1"
                                                        prop:value=entry_date
                                                        on:input=move |ev| set_entry_date.set(event_target_value(&ev))
                                                        required
                                                    />
                                                </div>

                                                <div class="md:col-span-2">
                                                    <label class="label">"Description"</label>
                                                    <textarea
                                                        class="input mt-1 min-h-[84px]"
                                                        prop:value=description
                                                        on:input=move |ev| set_description.set(event_target_value(&ev))
                                                        required
                                                    ></textarea>
                                                </div>

                                                <div class="md:col-span-2">
                                                    <label class="label">"Project (Optional)"</label>
                                                    <select
                                                        class="input mt-1"
                                                        prop:value=project_id
                                                        on:change=move |ev| set_project_id.set(event_target_value(&ev))
                                                    >
                                                        <option value="">"No linked project"</option>
                                                        {move || {
                                                            projects
                                                                .get()
                                                                .into_iter()
                                                                .map(|project| {
                                                                    view! { <option value={project.id.to_string()}>{project.name}</option> }
                                                                })
                                                                .collect_view()
                                                        }}
                                                    </select>
                                                </div>
                                            </div>

                                            <div class="flex justify-end">
                                                <button class="btn-primary btn-press" type="submit" disabled=move || submitting.get()>
                                                    {move || if submitting.get() { "Saving..." } else { "Save Entry" }}
                                                </button>
                                            </div>
                                        </form>
                                    </div>

                                    <div class="panel p-4">
                                        <div class="flex flex-wrap items-end gap-4 mb-4">
                                            <h2 class="text-lg font-semibold text-huly-caption mr-auto">"Cash Flow Dashboard"</h2>
                                            <div>
                                                <label class="label text-xs" for="dash-start">"Start Date"</label>
                                                <input
                                                    id="dash-start"
                                                    type="date"
                                                    class="input mt-1 w-40"
                                                    prop:value=dash_start
                                                    on:input=move |ev| set_dash_start.set(event_target_value(&ev))
                                                />
                                            </div>
                                            <div>
                                                <label class="label text-xs" for="dash-end">"End Date"</label>
                                                <input
                                                    id="dash-end"
                                                    type="date"
                                                    class="input mt-1 w-40"
                                                    prop:value=dash_end
                                                    on:input=move |ev| set_dash_end.set(event_target_value(&ev))
                                                />
                                            </div>
                                            <div>
                                                <label class="label text-xs" for="dash-project">"Project"</label>
                                                <select
                                                    id="dash-project"
                                                    class="input mt-1 w-48"
                                                    prop:value=dash_project
                                                    on:change=move |ev| set_dash_project.set(event_target_value(&ev))
                                                >
                                                    <option value="">"All Projects"</option>
                                                    {move || {
                                                        projects
                                                            .get()
                                                            .into_iter()
                                                            .map(|p| {
                                                                view! { <option value={p.id.to_string()}>{p.name}</option> }
                                                            })
                                                            .collect_view()
                                                    }}
                                                </select>
                                            </div>
                                            <button
                                                class="btn-primary btn-press text-sm"
                                                on:click=move |_| set_dash_reload_nonce.update(|value| *value += 1)
                                                disabled=move || dash_loading.get()
                                                aria-label="Refresh dashboard"
                                            >
                                                {move || if dash_loading.get() { "Refreshing..." } else { "Refresh" }}
                                            </button>
                                        </div>

                                        {move || dash_error.get().map(|err| view! { <div class="alert-error mb-4">{err}</div> })}

                                        {move || {
                                            let current_dashboard = dashboard_data.get().or_else(|| dashboard_snapshot.get());

                                            if dash_loading.get() && current_dashboard.is_none() {
                                                Some(view! { <div class="p-4 text-sm text-huly-muted">"Loading dashboard..."</div> }.into_any())
                                            } else if let Some(data) = current_dashboard {
                                                let project_active = data.project_id.is_some();
                                                let net_color = if data.net_cash_flow_idr >= 0 { "text-positive-default" } else { "text-negative-default" };
                                                let cum_color = if data.ending_cumulative_position_idr >= 0 { "text-positive-default" } else { "text-negative-default" };

                                                let months_for_chart = data.months.clone();
                                                let months_for_table = data.months.clone();

                                                let mut max_abs: i64 = 1;
                                                for m in &data.months {
                                                    let abs_cum = m.cumulative_position_idr.abs();
                                                    if abs_cum > max_abs { max_abs = abs_cum; }
                                                }

                                                let chart_width = 800.0_f64;
                                                let chart_height = 150.0_f64;
                                                let chart_padding = 40.0_f64;
                                                let num_months = months_for_chart.len().max(1) as f64;
                                                let step = (chart_width - chart_padding * 2.0) / (num_months - 1.0).max(1.0);

                                                let zero_y = chart_padding + chart_height / 2.0;
                                                let scale = (chart_height / 2.0) / max_abs as f64;

                                                let points: Vec<(f64, f64)> = months_for_chart.iter().enumerate().map(|(i, m)| {
                                                    let x = chart_padding + i as f64 * step;
                                                    let y = zero_y - (m.cumulative_position_idr as f64 * scale);
                                                    (x, y)
                                                }).collect();

                                                let line_path = if points.is_empty() {
                                                    String::new()
                                                } else {
                                                    let mut path = format!("M {:.1} {:.1}", points[0].0, points[0].1);
                                                    for &(x, y) in &points[1..] {
                                                        path.push_str(&format!(" L {:.1} {:.1}", x, y));
                                                    }
                                                    path
                                                };

                                                let chart_total_height = chart_height + chart_padding * 2.0;

                                                Some(view! {
                                                    <p class="mb-4 text-xs text-huly-muted">
                                                        {format!("Showing whole-month buckets from {} to {}.", data.start_date, data.end_date)}
                                                    </p>

                                                    <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
                                                        <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                            <p class="text-sm text-huly-muted mb-1">"Total Cash In"</p>
                                                            <p class="text-xl font-bold text-positive-default">{format_idr(data.total_cash_in_idr)}</p>
                                                        </div>
                                                        <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                            <p class="text-sm text-huly-muted mb-1">"Total Cash Out"</p>
                                                            <p class="text-xl font-bold text-negative-default">{format_idr(data.total_cash_out_idr)}</p>
                                                        </div>
                                                        <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                            <p class="text-sm text-huly-muted mb-1">"Net Cash Flow"</p>
                                                            <p class=format!("text-xl font-bold {}", net_color)>{format_idr(data.net_cash_flow_idr)}</p>
                                                        </div>
                                                        <div class="bg-huly-surface-2 rounded-lg p-4 border border-huly-divider">
                                                            <p class="text-sm text-huly-muted mb-1">
                                                                {if project_active { "Project Net Position" } else { "Cumulative Position" }}
                                                            </p>
                                                            <p class=format!("text-xl font-bold {}", cum_color)>{format_idr(data.ending_cumulative_position_idr)}</p>
                                                        </div>
                                                    </div>

                                                    <div class="mb-6 border border-huly-divider rounded-lg p-4">
                                                        <h3 class="text-sm font-medium text-huly-content mb-4">"Cumulative Cash Position"</h3>
                                                        <div class="w-full overflow-x-auto">
                                                            <svg
                                                                width="100%"
                                                                height=chart_total_height
                                                                viewBox=format!("0 0 {} {}", chart_width, chart_total_height)
                                                                preserveAspectRatio="xMidYMid meet"
                                                                role="img"
                                                                aria-label="Cumulative cash position line chart"
                                                            >
                                                                <line
                                                                    x1=chart_padding
                                                                    y1=zero_y
                                                                    x2={chart_width - chart_padding}
                                                                    y2=zero_y
                                                                    stroke="#4b5563"
                                                                    stroke-width="1"
                                                                    stroke-dasharray="4"
                                                                />
                                                                <text x={chart_padding - 4.0} y={zero_y + 4.0} text-anchor="end" class="fill-huly-muted text-[10px]">"0"</text>

                                                                {if !line_path.is_empty() {
                                                                    Some(view! {
                                                                        <path
                                                                            d=line_path.clone()
                                                                            fill="none"
                                                                            stroke="#10b981"
                                                                            stroke-width="2"
                                                                            stroke-linejoin="round"
                                                                        />
                                                                    })
                                                                } else {
                                                                    None
                                                                }}

                                                                {points.iter().enumerate().map(|(i, &(x, y))| {
                                                                    let m = &months_for_chart[i];
                                                                    let label_text = format!(
                                                                        "{} {}: {}",
                                                                        m.month_label, m.year,
                                                                        format_idr(m.cumulative_position_idr)
                                                                    );
                                                                    let yr = m.year;
                                                                    let mo = m.month;
                                                                    let month_label = m.month_label.clone();
                                                                    view! {
                                                                        <circle
                                                                            cx=x
                                                                            cy=y
                                                                            r="4"
                                                                            fill="#10b981"
                                                                            tabindex="0"
                                                                            focusable="true"
                                                                            aria-label=label_text
                                                                            on:mouseenter=move |_| set_dash_hover_month.set(Some((yr, mo)))
                                                                            on:mouseleave=move |_| set_dash_hover_month.set(None)
                                                                            on:focus=move |_| set_dash_hover_month.set(Some((yr, mo)))
                                                                            on:blur=move |_| set_dash_hover_month.set(None)
                                                                        />
                                                                        <text x=x y={chart_total_height - 5.0} text-anchor="middle" class="fill-huly-muted text-[10px]">
                                                                            {month_label}
                                                                        </text>
                                                                    }
                                                                }).collect_view()}
                                                            </svg>

                                                            {move || {
                                                                dash_hover_month
                                                                    .get()
                                                                    .and_then(|(yr, mo)| {
                                                                        dashboard_snapshot.get().and_then(|d| {
                                                                            d.months.iter().find(|m| m.year == yr && m.month == mo).cloned()
                                                                        })
                                                                    })
                                                                    .map(|m| {
                                                                        view! {
                                                                            <div class="mt-3 bg-huly-tooltip-bg text-huly-content rounded-md px-3 py-2 text-xs space-y-1" role="status" aria-live="polite">
                                                                                <p><span class="font-semibold">"Cash In: "</span>{format_idr(m.cash_in_idr)}</p>
                                                                                <p><span class="font-semibold">"Cash Out: "</span>{format_idr(m.cash_out_idr)}</p>
                                                                                <p><span class="font-semibold">"Net: "</span>{format_idr(m.net_cash_flow_idr)}</p>
                                                                                <p><span class="font-semibold">"Cumulative: "</span>{format_idr(m.cumulative_position_idr)}</p>
                                                                            </div>
                                                                        }
                                                                    })
                                                            }}
                                                        </div>
                                                    </div>

                                                    <div class="overflow-x-auto mb-4">
                                                        <table class="min-w-full divide-y divide-huly-divider">
                                                            <thead class="table-head">
                                                                <tr>
                                                                    <th class="th-cell-compact">"Month"</th>
                                                                    <th class="th-cell-compact text-right">"Cash In"</th>
                                                                    <th class="th-cell-compact text-right">"Cash Out"</th>
                                                                    <th class="th-cell-compact text-right">"Net"</th>
                                                                    <th class="th-cell-compact text-right">"Cumulative"</th>
                                                                    <th class="th-cell-compact text-center">"Details"</th>
                                                                </tr>
                                                            </thead>
                                                            <tbody class="divide-y divide-huly-divider">
                                                                {months_for_table.into_iter().map(|m| {
                                                                    let yr = m.year;
                                                                    let mo = m.month;
                                                                    let month_name = m.month_label.clone();
                                                                    let net_c = if m.net_cash_flow_idr >= 0 { "text-positive-default" } else { "text-negative-default" };
                                                                    let cum_c = if m.cumulative_position_idr >= 0 { "text-positive-default" } else { "text-negative-default" };
                                                                    let has_entries = !m.entries.is_empty();
                                                                    let entries_for_detail = m.entries.clone();

                                                                    view! {
                                                                        <tr class="table-row-hover">
                                                                            <td class="td-cell-compact font-medium text-huly-caption">{format!("{} {}", m.month_label, yr)}</td>
                                                                            <td class="td-cell-compact text-right text-positive-default">{format_idr(m.cash_in_idr)}</td>
                                                                            <td class="td-cell-compact text-right text-negative-default">{format_idr(m.cash_out_idr)}</td>
                                                                            <td class=format!("td-cell-compact text-right font-medium {}", net_c)>{format_idr(m.net_cash_flow_idr)}</td>
                                                                            <td class=format!("td-cell-compact text-right font-medium {}", cum_c)>{format_idr(m.cumulative_position_idr)}</td>
                                                                            <td class="td-cell-compact text-center text-huly-muted text-xs">
                                                                                {if has_entries {
                                                                                    Either::Left(view! {
                                                                                        <button
                                                                                            type="button"
                                                                                            class="inline-flex min-h-[44px] min-w-[44px] items-center justify-center rounded-md px-2 text-xs font-medium text-huly-caption hover:bg-huly-surface-2 focus:outline-none focus:ring-2 focus:ring-primary"
                                                                                            on:click=move |_| {
                                                                                                set_expanded_month.update(|current| {
                                                                                                    if *current == Some((yr, mo)) {
                                                                                                        *current = None;
                                                                                                    } else {
                                                                                                        *current = Some((yr, mo));
                                                                                                    }
                                                                                                });
                                                                                            }
                                                                                            aria-expanded=move || expanded_month.get() == Some((yr, mo))
                                                                                            aria-label=format!("Toggle details for {} {}", month_name, yr)
                                                                                        >
                                                                                            {move || if expanded_month.get() == Some((yr, mo)) { "Hide" } else { "Show" }}
                                                                                        </button>
                                                                                    })
                                                                                } else {
                                                                                    Either::Right("—")
                                                                                }}
                                                                            </td>
                                                                        </tr>
                                                                        {move || {
                                                                            if expanded_month.get() == Some((yr, mo)) && !entries_for_detail.is_empty() {
                                                                                Some(entries_for_detail.iter().map(|e| {
                                                                                    let badge_cls = if e.entry_type == "cash_in" { "badge-success" } else { "badge-warning" };
                                                                                    let type_lbl = if e.entry_type == "cash_in" { "In" } else { "Out" };
                                                                                    let project_lbl = e.project_id.map(|id| {
                                                                                        projects
                                                                                            .get()
                                                                                            .into_iter()
                                                                                            .find(|p| p.id == id)
                                                                                            .map(|p| p.name.clone())
                                                                                            .unwrap_or_else(|| id.to_string())
                                                                                    }).unwrap_or_else(|| "-".to_string());
                                                                                    view! {
                                                                                        <tr class="bg-huly-surface-2" data-entry-id={e.id.to_string()}>
                                                                                            <td class="td-cell-compact text-huly-muted pl-8 text-xs">{e.entry_date.clone()}</td>
                                                                                            <td class="td-cell-compact text-right">
                                                                                                <span class=format!("{} text-xs", badge_cls)>{type_lbl}</span>
                                                                                            </td>
                                                                                            <td class="td-cell-compact text-right text-xs text-huly-content">{category_label(&e.category)}</td>
                                                                                            <td class="td-cell-compact text-right text-xs font-medium text-huly-caption">{format_idr(e.amount_idr)}</td>
                                                                                            <td class="td-cell-compact text-xs text-huly-content" colspan="2">
                                                                                                {e.description.clone()}
                                                                                                <span class="ml-2 text-huly-muted">{project_lbl}</span>
                                                                                            </td>
                                                                                        </tr>
                                                                                    }
                                                                                }).collect_view())
                                                                            } else {
                                                                                None
                                                                            }
                                                                        }}
                                                                    }
                                                                }).collect_view()}
                                                            </tbody>
                                                        </table>
                                                    </div>
                                                }.into_any())
                                            } else {
                                                None
                                            }
                                        }}
                                    </div>

                                    <div class="panel overflow-hidden">
                                        <div class="toolbar flex items-center justify-between gap-3">
                                            <h2 class="text-sm font-semibold text-huly-secondary uppercase tracking-wider">"Entry List"</h2>
                                            <div class="w-56">
                                                <select
                                                    class="input"
                                                    prop:value=list_project_filter
                                                    on:change=move |ev| set_list_project_filter.set(event_target_value(&ev))
                                                >
                                                    <option value="">"All projects"</option>
                                                    {move || {
                                                        projects
                                                            .get()
                                                            .into_iter()
                                                            .map(|project| {
                                                                view! { <option value={project.id.to_string()}>{project.name}</option> }
                                                            })
                                                            .collect_view()
                                                    }}
                                                </select>
                                            </div>
                                        </div>

                                        {move || {
                                            if loading.get() {
                                                EitherOf3::A(view! {
                                                    <div class="p-4 text-sm text-huly-muted">"Loading entries..."</div>
                                                })
                                            } else {
                                                let rows = entries.get();
                                                if rows.is_empty() {
                                                    EitherOf3::B(view! {
                                                        <div class="empty-state py-10">
                                                            <p class="text-huly-muted text-sm">"No cash flow entries yet."</p>
                                                        </div>
                                                    })
                                                } else {
                                                    EitherOf3::C(view! {
                                                        <div class="overflow-x-auto">
                                                            <table class="min-w-full divide-y divide-huly-divider">
                                                                <thead class="table-head">
                                                                    <tr>
                                                                        <th class="th-cell-compact">"Date"</th>
                                                                        <th class="th-cell-compact">"Type"</th>
                                                                        <th class="th-cell-compact">"Category"</th>
                                                                        <th class="th-cell-compact text-right">"Amount"</th>
                                                                        <th class="th-cell-compact">"Description"</th>
                                                                        <th class="th-cell-compact">"Project"</th>
                                                                    </tr>
                                                                </thead>
                                                                <tbody class="divide-y divide-huly-divider bg-huly-surface">
                                                                    {rows
                                                                        .into_iter()
                                                                        .map(|entry| {
                                                                            let badge_class = if entry.entry_type == "cash_in" {
                                                                                "badge-success"
                                                                            } else {
                                                                                "badge-warning"
                                                                            };
                                                                            let type_label = if entry.entry_type == "cash_in" {
                                                                                "Cash In"
                                                                            } else {
                                                                                "Cash Out"
                                                                            };
                                                                            let project_label = entry.project_id.map(|id| {
                                                                                projects
                                                                                    .get()
                                                                                    .into_iter()
                                                                                    .find(|project| project.id == id)
                                                                                    .map(|project| project.name)
                                                                                    .unwrap_or_else(|| id.to_string())
                                                                            }).unwrap_or_else(|| "-".to_string());
                                                                            view! {
                                                                                <tr class="table-row-hover">
                                                                                    <td class="td-cell-compact text-huly-content">{entry.entry_date}</td>
                                                                                    <td class="td-cell-compact">
                                                                                        <span class=badge_class>{type_label}</span>
                                                                                    </td>
                                                                                    <td class="td-cell-compact text-huly-content">{category_label(&entry.category)}</td>
                                                                                    <td class="td-cell-compact text-right font-medium text-huly-caption">{format_idr(entry.amount_idr)}</td>
                                                                                    <td class="td-cell-compact text-huly-content">{entry.description}</td>
                                                                                    <td class="td-cell-compact text-huly-muted">{project_label}</td>
                                                                                </tr>
                                                                            }
                                                                        })
                                                                        .collect_view()}
                                                                </tbody>
                                                            </table>
                                                        </div>
                                                    })
                                                }
                                            }
                                        }}
                                    </div>
                                </div>
                            })
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
