use crate::auth::{authenticated_get, authenticated_post_json, use_auth};
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
