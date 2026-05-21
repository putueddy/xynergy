use crate::auth::{authenticated_get, logout_user, use_auth};

use leptos::either::Either;
use leptos::prelude::*;
use leptos_router::hooks::*;
use serde::Deserialize;
use uuid::Uuid;

// ── Response shape (mirror of backend RoleDashboardResponse) ──────────────

#[derive(Debug, Clone, Deserialize)]
struct RoleDashboardResponse {
    role: String,
    generated_at: String,
    #[serde(default)]
    hr: Option<HrDashboard>,
    #[serde(default)]
    department_head: Option<DepartmentHeadDashboard>,
    #[serde(default)]
    project_manager: Option<ProjectManagerDashboard>,
    #[serde(default)]
    finance: Option<FinanceDashboard>,
    #[serde(default)]
    admin: Option<AdminDashboard>,
}

// HR

#[derive(Debug, Clone, Deserialize)]
struct HrDashboard {
    completeness: CompletenessReport,
    pending_updates: HrPendingUpdates,
    #[serde(default)]
    recent_changes: Vec<RecentCtcChange>,
    compliance_alerts: ComplianceAlertSummary,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CompletenessReport {
    total_employees: i64,
    total_with_ctc: i64,
    total_missing: i64,
    overall_completion_pct: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct HrPendingUpdates {
    missing_count: i64,
    #[serde(default)]
    sample: Vec<HrMissingEmployee>,
}

#[derive(Debug, Clone, Deserialize)]
struct HrMissingEmployee {
    name: String,
    department: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RecentCtcChange {
    resource_name: String,
    revision_number: i32,
    #[serde(default)]
    changed_by_name: Option<String>,
    created_at: String,
    reason: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ComplianceAlertSummary {
    start_date: String,
    end_date: String,
    total_validated: i64,
    total_passed: i64,
    total_discrepancies: i64,
    compliance_rate_pct: f64,
    #[serde(default)]
    top_risks: Vec<ComplianceTopRisk>,
}

#[derive(Debug, Clone, Deserialize)]
struct ComplianceTopRisk {
    name: String,
    variance_amount: i64,
}

// Department Head

#[derive(Debug, Clone, Deserialize)]
struct DepartmentHeadDashboard {
    utilization: UtilizationSummary,
    #[serde(default)]
    budget: Option<DepartmentBudgetSummary>,
    overallocations: OverallocationSummary,
    #[serde(default)]
    upcoming_assignments: Vec<UpcomingAssignment>,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct UtilizationSummary {
    average_utilization_pct: f64,
    overallocated_count: i64,
    #[serde(default)]
    top_at_risk: Vec<UtilizationAtRisk>,
}

#[derive(Debug, Clone, Deserialize)]
struct UtilizationAtRisk {
    resource_name: String,
    current_allocation_pct: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct DepartmentBudgetSummary {
    department_name: String,
    budget_period: String,
    total_budget_idr: i64,
    total_committed_idr: i64,
    remaining_idr: i64,
    utilization_percentage: f64,
    budget_health: String,
    budget_configured: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct OverallocationSummary {
    overallocated_count: i64,
    #[serde(default)]
    members: Vec<UtilizationAtRisk>,
}

#[derive(Debug, Clone, Deserialize)]
struct UpcomingAssignment {
    resource_name: String,
    project_name: String,
    start_date: String,
    end_date: String,
    allocation_percentage: f64,
}

// Project Manager

#[derive(Debug, Clone, Deserialize)]
struct ProjectManagerDashboard {
    #[serde(default)]
    active_projects: Vec<ProjectHealthCard>,
    #[serde(default)]
    margin_alerts: Vec<MarginAlert>,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProjectHealthCard {
    project_name: String,
    status: String,
    end_date: String,
    total_budget_idr: i64,
    budget_spent_idr: i64,
    budget_remaining_idr: i64,
    budget_status: String,
    total_revenue_idr: i64,
    total_cost_idr: i64,
    gross_profit_idr: i64,
    margin_pct: f64,
    #[serde(default)]
    margin_alert: Option<String>,
    #[serde(default)]
    warning: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct MarginAlert {
    project_name: String,
    margin_pct: f64,
    message: String,
}

// Finance

#[derive(Debug, Clone, Deserialize)]
struct FinanceDashboard {
    cash_position: CashPositionSummary,
    ctc_validation: CtcValidationStatus,
    audit_alerts: AuditAlertsSummary,
    export_requests: ExportRequestsSummary,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CashPositionSummary {
    start_date: String,
    end_date: String,
    total_cash_in_idr: i64,
    total_cash_out_idr: i64,
    net_cash_flow_idr: i64,
    ending_cumulative_position_idr: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct CtcValidationStatus {
    status: String,
    start_date: String,
    end_date: String,
    #[serde(default)]
    total_compared: Option<i64>,
    #[serde(default)]
    total_matches: Option<i64>,
    #[serde(default)]
    total_discrepancies: Option<i64>,
    #[serde(default)]
    match_rate_pct: Option<f64>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AuditAlertsSummary {
    access_denied_count: i64,
    login_failed_count: i64,
    login_blocked_count: i64,
    chain_verification_failure_count: i64,
    #[serde(default)]
    recent: Vec<AuditAlertEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct AuditAlertEntry {
    action: String,
    entity_type: String,
    created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ExportRequestsSummary {
    pending_count: i64,
    #[serde(default)]
    latest_pending: Vec<PendingExportRequest>,
}

#[derive(Debug, Clone, Deserialize)]
struct PendingExportRequest {
    status: String,
    created_at: String,
    #[serde(default)]
    report_type: Option<String>,
}

// Admin

#[derive(Debug, Clone, Deserialize)]
struct AdminDashboard {
    total_users: i64,
    total_departments: i64,
    total_active_projects: i64,
    total_active_ctc_records: i64,
    pending_export_requests: i64,
    access_denied_24h: i64,
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn format_idr(amount: i64) -> String {
    let sign = if amount < 0 { "-" } else { "" };
    let abs = amount.unsigned_abs();
    let raw = abs.to_string();
    let bytes = raw.as_bytes();
    let mut out = String::with_capacity(raw.len() + raw.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(*b as char);
    }
    format!("Rp {}{}", sign, out)
}

fn format_date(value: &str) -> String {
    value.split('T').next().unwrap_or(value).to_string()
}

fn role_display(role: &str) -> &'static str {
    match role {
        "hr" => "HR",
        "department_head" => "Department Head",
        "project_manager" => "Project Manager",
        "finance" => "Finance",
        "admin" => "Administrator",
        _ => "User",
    }
}

async fn fetch_role_dashboard() -> Result<RoleDashboardResponse, String> {
    let response = authenticated_get("/api/v1/dashboard").await.map_err(|e| {
        if e == "SESSION_EXPIRED" {
            e
        } else {
            format!("Failed to fetch dashboard: {}", e)
        }
    })?;

    if response.status().is_success() {
        response
            .json::<RoleDashboardResponse>()
            .await
            .map_err(|e| format!("Failed to parse dashboard: {}", e))
    } else {
        Err(format!("Failed to fetch dashboard: {}", response.status()))
    }
}

#[derive(Debug, Clone)]
struct DashboardState {
    data: Option<RoleDashboardResponse>,
    error: Option<String>,
    loading: bool,
}

impl DashboardState {
    fn initial() -> Self {
        Self {
            data: None,
            error: None,
            loading: true,
        }
    }
}

#[component]
pub fn Dashboard() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();

    // Redirect if not logged in
    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
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
    let (state, set_state) = signal(DashboardState::initial());
    let (refresh_token, set_refresh_token) = signal(0u32);

    let load_dashboard = {
        let navigate = navigate.clone();
        move || {
            set_state.update(|s| {
                s.loading = true;
                s.error = None;
            });
            let navigate = navigate.clone();
            leptos::task::spawn_local(async move {
                match fetch_role_dashboard().await {
                    Ok(data) => {
                        set_state.update(|s| {
                            s.data = Some(data);
                            s.error = None;
                            s.loading = false;
                        });
                    }
                    Err(e) if e == "SESSION_EXPIRED" => {
                        logout_user(&auth);
                        set_state.update(|s| {
                            s.error =
                                Some("Your session expired. Please sign in again.".to_string());
                            s.loading = false;
                        });
                        navigate("/login", Default::default());
                    }
                    Err(e) => {
                        set_state.update(|s| {
                            s.error = Some(e);
                            s.loading = false;
                        });
                    }
                }
            });
        }
    };

    {
        let load = load_dashboard.clone();
        Effect::new(move |_| {
            let _ = refresh_token.get();
            load();
        });
    }

    let refresh_click = {
        move |_| {
            set_refresh_token.update(|n| *n = n.wrapping_add(1));
        }
    };

    view! {
        <div class="h-full">
            <div class="page-container fade-in">
                <div class="page-header">
                    <div>
                        <h1 class="text-xl font-semibold text-huly-caption">
                            {move || user.get().map(|u| format!("Welcome, {}!", u.first_name)).unwrap_or_else(|| "Welcome!".to_string())}
                        </h1>
                        <p class="text-xs text-huly-muted mt-0.5">
                            {move || user.get().map(|u| format!("{} · {}", role_display(&u.role), u.email)).unwrap_or_default()}
                        </p>
                        <p class="text-xs text-huly-muted mt-0.5">
                            {move || {
                                state.get().data.as_ref().map(|d| format!("Last updated: {}", format_datetime(&d.generated_at))).unwrap_or_default()
                            }}
                        </p>
                    </div>
                    <div class="flex items-center gap-2">
                        <button
                            on:click=refresh_click
                            class="btn-ghost text-xs"
                            disabled=move || state.get().loading
                        >
                            {move || if state.get().loading { "Refreshing…" } else { "Refresh" }}
                        </button>
                        <button
                            on:click=handle_logout
                            class="btn-ghost text-xs"
                        >
                            "Sign out"
                        </button>
                    </div>
                </div>

                {move || state.get().error.map(|err| view! {
                    <div class="alert-error mb-3">
                        <span class="text-sm">{err}</span>
                    </div>
                })}

                {move || {
                    let s = state.get();
                    match s.data {
                        Some(data) => Either::Left(view! { <DashboardBody data=data /> }),
                        None if s.loading => Either::Right(view! {
                            <div class="panel p-6 text-center text-huly-muted text-sm">
                                "Loading dashboard…"
                            </div>
                        }),
                        None => Either::Right(view! {
                            <div class="panel p-6 text-center text-huly-muted text-sm">
                                "No dashboard data available."
                            </div>
                        }),
                    }
                }}
            </div>
        </div>
    }
}

fn format_datetime(value: &str) -> String {
    // generated_at is RFC3339 UTC; the bare date+HH:MM is enough for users.
    let trimmed = value.split('.').next().unwrap_or(value);
    trimmed.replace('T', " ")
}

#[component]
fn DashboardBody(data: RoleDashboardResponse) -> impl IntoView {
    let role = data.role.clone();
    let no_widgets = data.hr.is_none()
        && data.department_head.is_none()
        && data.project_manager.is_none()
        && data.finance.is_none()
        && data.admin.is_none();
    view! {
        <div class="space-y-4">
            {data.hr.map(|hr| view! { <HrPanel hr=hr /> })}
            {data.department_head.map(|dh| view! { <DepartmentHeadPanel dh=dh /> })}
            {data.project_manager.map(|pm| view! { <ProjectManagerPanel pm=pm /> })}
            {data.finance.map(|f| view! { <FinancePanel finance=f /> })}
            {data.admin.map(|a| view! { <AdminPanel admin=a /> })}
            {if no_widgets {
                Some(view! {
                    <div class="panel p-4 text-center text-huly-muted text-sm">
                        {format!("No dashboard widgets available for role '{}'.", role)}
                    </div>
                })
            } else {
                None
            }}
        </div>
    }
}

#[component]
fn DashboardWarnings(warnings: Vec<String>) -> impl IntoView {
    view! {
        {if warnings.is_empty() {
            None
        } else {
            Some(view! {
                <div class="alert-error">
                    <div class="space-y-1">
                        {warnings.into_iter().map(|warning| view! {
                            <p class="text-xs">{warning}</p>
                        }).collect_view()}
                    </div>
                </div>
            })
        }}
    }
}

// ── HR Panel ─────────────────────────────────────────────────────────────

#[component]
fn HrPanel(hr: HrDashboard) -> impl IntoView {
    let warnings = hr.warnings.clone();
    let completeness = hr.completeness.clone();
    let pending = hr.pending_updates.clone();
    let compliance = hr.compliance_alerts.clone();
    let changes = hr.recent_changes.clone();

    view! {
        <div class="space-y-4">
            <DashboardWarnings warnings=warnings />
            <div class="stat-grid">
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"CTC Completeness"</p>
                        <p class="stat-value">{format!("{:.1}%", completeness.overall_completion_pct)}</p>
                        <p class="text-xs text-huly-muted">{format!("{} / {} employees", completeness.total_with_ctc, completeness.total_employees)}</p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Pending Updates"</p>
                        <p class="stat-value">{pending.missing_count.to_string()}</p>
                        <p class="text-xs text-huly-muted">"Missing CTC records"</p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Compliance Rate"</p>
                        <p class="stat-value">{format!("{:.1}%", compliance.compliance_rate_pct)}</p>
                        <p class="text-xs text-huly-muted">{format!("{} discrepancies", compliance.total_discrepancies)}</p>
                    </div>
                </div>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
                <div class="panel">
                    <div class="toolbar flex items-center justify-between">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Recent CTC Changes"</h3>
                        <a href="/ctc/completeness" class="text-xs text-primary-400 hover:text-primary-300">"View completeness →"</a>
                    </div>
                    <div class="p-3">
                        {if changes.is_empty() {
                            Either::Left(view! {
                                <div class="empty-state py-4">
                                    <p class="text-huly-muted text-xs">"No recent CTC changes."</p>
                                </div>
                            })
                        } else {
                            Either::Right(view! {
                                <div>
                                    {changes.into_iter().map(|c| {
                                        let summary = format!("{} · rev {} · {}", c.resource_name, c.revision_number, c.changed_by_name.unwrap_or_else(|| "System".to_string()));
                                        view! {
                                            <div class="activity-item">
                                                <span class="text-sm text-huly-content flex-1">{summary}</span>
                                                <span class="text-xs text-huly-muted whitespace-nowrap">{format_date(&c.created_at)}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>
                </div>

                <div class="panel">
                    <div class="toolbar flex items-center justify-between">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Pending CTC Updates"</h3>
                        <a href="/ctc/completeness" class="text-xs text-primary-400 hover:text-primary-300">"Open list →"</a>
                    </div>
                    <div class="p-3">
                        {if pending.sample.is_empty() {
                            Either::Left(view! {
                                <div class="empty-state py-4">
                                    <p class="text-huly-muted text-xs">"All employees have a CTC record."</p>
                                </div>
                            })
                        } else {
                            Either::Right(view! {
                                <div>
                                    {pending.sample.into_iter().map(|e| {
                                        view! {
                                            <div class="activity-item">
                                                <span class="text-sm text-huly-content flex-1">{e.name.clone()}</span>
                                                <span class="text-xs text-huly-muted whitespace-nowrap">{e.department.clone()}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>
                </div>
            </div>

            <div class="panel">
                <div class="toolbar">
                    <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">
                        {format!("Compliance Alerts ({} → {})", format_date(&compliance.start_date), format_date(&compliance.end_date))}
                    </h3>
                </div>
                <div class="p-3 text-sm text-huly-content">
                    <div class="grid grid-cols-3 gap-3 mb-3">
                        <div>
                            <p class="stat-label">"Validated"</p>
                            <p class="stat-value">{compliance.total_validated.to_string()}</p>
                        </div>
                        <div>
                            <p class="stat-label">"Passed"</p>
                            <p class="stat-value">{compliance.total_passed.to_string()}</p>
                        </div>
                        <div>
                            <p class="stat-label">"Discrepancies"</p>
                            <p class="stat-value">{compliance.total_discrepancies.to_string()}</p>
                        </div>
                    </div>
                    {if compliance.top_risks.is_empty() {
                        Either::Left(view! {
                            <p class="text-huly-muted text-xs">"No compliance discrepancies in window."</p>
                        })
                    } else {
                        Either::Right(view! {
                            <div>
                                {compliance.top_risks.into_iter().map(|r| {
                                    view! {
                                        <div class="activity-item">
                                            <span class="text-sm text-huly-content flex-1">{r.name.clone()}</span>
                                            <span class="text-xs text-huly-muted whitespace-nowrap">
                                                {format!("Δ {}", format_idr(r.variance_amount))}
                                            </span>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        })
                    }}
                </div>
            </div>
        </div>
    }
}

// ── Department Head Panel ────────────────────────────────────────────────

#[component]
fn DepartmentHeadPanel(dh: DepartmentHeadDashboard) -> impl IntoView {
    let warnings = dh.warnings.clone();
    let utilization = dh.utilization.clone();
    let overallocations = dh.overallocations.clone();
    let upcoming = dh.upcoming_assignments.clone();
    let budget = dh.budget.clone();

    view! {
        <div class="space-y-4">
            <DashboardWarnings warnings=warnings />
            <div class="stat-grid">
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Avg Utilization (30d)"</p>
                        <p class="stat-value">{format!("{:.1}%", utilization.average_utilization_pct)}</p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Overallocated"</p>
                        <p class="stat-value">{overallocations.overallocated_count.to_string()}</p>
                    </div>
                </div>
                {budget.clone().map(|b| view! {
                    <div class="stat-card card-hover">
                        <div>
                            <p class="stat-label">{format!("Budget ({})", b.budget_period.clone())}</p>
                            <p class="stat-value">{format!("{:.0}%", b.utilization_percentage)}</p>
                            <p class="text-xs text-huly-muted">{format!("{} of {}", format_idr(b.total_committed_idr), format_idr(b.total_budget_idr))}</p>
                        </div>
                    </div>
                })}
            </div>

            {budget.clone().map(|b| view! {
                <div class="panel">
                    <div class="toolbar flex items-center justify-between">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Budget Status"</h3>
                        <a href="/team" class="text-xs text-primary-400 hover:text-primary-300">"Open team →"</a>
                    </div>
                    <div class="p-3 text-sm text-huly-content">
                        <p>
                            <span class="text-huly-muted">"Department: "</span>
                            <span>{b.department_name.clone()}</span>
                        </p>
                        <p>
                            <span class="text-huly-muted">"Remaining: "</span>
                            <span>{format_idr(b.remaining_idr)}</span>
                        </p>
                        <p>
                            <span class="text-huly-muted">"Health: "</span>
                            <span>{b.budget_health.clone()}</span>
                        </p>
                        {if !b.budget_configured {
                            Some(view! { <p class="text-xs text-huly-muted mt-1">"(Budget not yet configured for this period.)"</p> })
                        } else { None }}
                    </div>
                </div>
            })}

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
                <div class="panel">
                    <div class="toolbar">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"At-risk Members"</h3>
                    </div>
                    <div class="p-3">
                        {if utilization.top_at_risk.is_empty() {
                            Either::Left(view! {
                                <div class="empty-state py-4">
                                    <p class="text-huly-muted text-xs">"No members at risk."</p>
                                </div>
                            })
                        } else {
                            Either::Right(view! {
                                <div>
                                    {utilization.top_at_risk.into_iter().map(|m| view! {
                                        <div class="activity-item">
                                            <span class="text-sm text-huly-content flex-1">{m.resource_name.clone()}</span>
                                            <span class="text-xs text-huly-muted whitespace-nowrap">{format!("{:.0}%", m.current_allocation_pct)}</span>
                                        </div>
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>
                </div>

                <div class="panel">
                    <div class="toolbar flex items-center justify-between">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Upcoming Assignments"</h3>
                        <a href="/team" class="text-xs text-primary-400 hover:text-primary-300">"Open team →"</a>
                    </div>
                    <div class="p-3">
                        {if upcoming.is_empty() {
                            Either::Left(view! {
                                <div class="empty-state py-4">
                                    <p class="text-huly-muted text-xs">"No upcoming assignments."</p>
                                </div>
                            })
                        } else {
                            Either::Right(view! {
                                <div>
                                    {upcoming.into_iter().map(|u| {
                                        let label = format!("{} → {} · {:.0}%", u.resource_name, u.project_name, u.allocation_percentage);
                                        view! {
                                            <div class="activity-item">
                                                <span class="text-sm text-huly-content flex-1">{label}</span>
                                                <span class="text-xs text-huly-muted whitespace-nowrap">{format!("{} → {}", format_date(&u.start_date), format_date(&u.end_date))}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>
                </div>
            </div>
        </div>
    }
}

// ── Project Manager Panel ────────────────────────────────────────────────

#[component]
fn ProjectManagerPanel(pm: ProjectManagerDashboard) -> impl IntoView {
    let warnings = pm.warnings.clone();
    let alerts = pm.margin_alerts.clone();
    let projects = pm.active_projects.clone();

    view! {
        <div class="space-y-4">
            <DashboardWarnings warnings=warnings />
            <div class="stat-grid">
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Active Projects"</p>
                        <p class="stat-value">{projects.len().to_string()}</p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Margin Alerts"</p>
                        <p class="stat-value">{alerts.len().to_string()}</p>
                    </div>
                </div>
            </div>

            {if !alerts.is_empty() {
                Some(view! {
                    <div class="panel">
                        <div class="toolbar">
                            <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Margin Alerts"</h3>
                        </div>
                        <div class="p-3">
                            {alerts.into_iter().map(|a| view! {
                                <div class="activity-item">
                                    <span class="text-sm text-huly-content flex-1">{a.project_name.clone()}</span>
                                    <span class="text-xs text-huly-muted whitespace-nowrap">{format!("{:.1}% · {}", a.margin_pct, a.message)}</span>
                                </div>
                            }).collect_view()}
                        </div>
                    </div>
                })
            } else { None }}

            <div class="panel">
                <div class="toolbar flex items-center justify-between">
                    <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Project Health"</h3>
                    <a href="/projects" class="text-xs text-primary-400 hover:text-primary-300">"Open projects →"</a>
                </div>
                <div class="p-3">
                    {if projects.is_empty() {
                        Either::Left(view! {
                            <div class="empty-state py-4">
                                <p class="text-huly-muted text-xs">"No active projects assigned."</p>
                            </div>
                        })
                    } else {
                        Either::Right(view! {
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
                                {projects.into_iter().map(|p| view! {
                                    <div class="panel p-3">
                                        <div class="flex items-center justify-between mb-1">
                                            <span class="text-sm font-medium text-huly-caption">{p.project_name.clone()}</span>
                                            <span class="text-xs text-huly-muted">{p.status.clone()}</span>
                                        </div>
                                        <p class="text-xs text-huly-muted">{format!("Ends {}", format_date(&p.end_date))}</p>
                                        <div class="mt-2 text-xs text-huly-content space-y-0.5">
                                            <p>{format!("Budget: {} · {}", p.budget_status, format_idr(p.total_budget_idr))}</p>
                                            <p>{format!("Budget spent: {} · remaining {}", format_idr(p.budget_spent_idr), format_idr(p.budget_remaining_idr))}</p>
                                            <p>{format!("Revenue: {}", format_idr(p.total_revenue_idr))}</p>
                                            <p>{format!("Cost: {}", format_idr(p.total_cost_idr))}</p>
                                            <p>{format!("Profit: {} ({:.1}%)", format_idr(p.gross_profit_idr), p.margin_pct)}</p>
                                            {p.warning.as_ref().map(|w| view! { <p class="text-xs text-negative-default">{w.clone()}</p> })}
                                            {p.margin_alert.as_ref().map(|a| view! { <p class="text-xs text-negative-default">{a.clone()}</p> })}
                                        </div>
                                    </div>
                                }).collect_view()}
                            </div>
                        })
                    }}
                </div>
            </div>
        </div>
    }
}

// ── Finance Panel ────────────────────────────────────────────────────────

#[component]
fn FinancePanel(finance: FinanceDashboard) -> impl IntoView {
    let warnings = finance.warnings.clone();
    let cash = finance.cash_position.clone();
    let validation = finance.ctc_validation.clone();
    let audit = finance.audit_alerts.clone();
    let exports = finance.export_requests.clone();

    let validation_status_label = match validation.status.as_str() {
        "ok" => "Validated",
        "no_data" => "No payroll data",
        "error" => "Error",
        _ => "Unknown",
    };

    view! {
        <div class="space-y-4">
            <DashboardWarnings warnings=warnings />
            <div class="stat-grid">
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Cash Position (YTD)"</p>
                        <p class="stat-value">{format_idr(cash.ending_cumulative_position_idr)}</p>
                        <p class="text-xs text-huly-muted">{format!("Net: {}", format_idr(cash.net_cash_flow_idr))}</p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"CTC Validation"</p>
                        <p class="stat-value">{validation_status_label}</p>
                        <p class="text-xs text-huly-muted">{validation.match_rate_pct.map(|p| format!("Match {:.1}%", p)).unwrap_or_else(|| "—".to_string())}</p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Audit Alerts (7d)"</p>
                        <p class="stat-value">{(audit.access_denied_count + audit.login_failed_count + audit.login_blocked_count + audit.chain_verification_failure_count).to_string()}</p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Pending Exports"</p>
                        <p class="stat-value">{exports.pending_count.to_string()}</p>
                    </div>
                </div>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
                <div class="panel">
                    <div class="toolbar flex items-center justify-between">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Cash Flow Summary"</h3>
                        <a href="/finance/cash-flow" class="text-xs text-primary-400 hover:text-primary-300">"Open dashboard →"</a>
                    </div>
                    <div class="p-3 text-sm text-huly-content space-y-1">
                        <p>{format!("Period: {} → {}", format_date(&cash.start_date), format_date(&cash.end_date))}</p>
                        <p>{format!("Cash in: {}", format_idr(cash.total_cash_in_idr))}</p>
                        <p>{format!("Cash out: {}", format_idr(cash.total_cash_out_idr))}</p>
                        <p>{format!("Net: {}", format_idr(cash.net_cash_flow_idr))}</p>
                        <p>{format!("Ending position: {}", format_idr(cash.ending_cumulative_position_idr))}</p>
                    </div>
                </div>

                <div class="panel">
                    <div class="toolbar flex items-center justify-between">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"CTC Validation"</h3>
                        <a href="/finance/ctc-validation" class="text-xs text-primary-400 hover:text-primary-300">"Open report →"</a>
                    </div>
                    <div class="p-3 text-sm text-huly-content space-y-1">
                        <p>{format!("Window: {} → {}", format_date(&validation.start_date), format_date(&validation.end_date))}</p>
                        {match validation.status.as_str() {
                            "ok" => Either::Left(view! {
                                <div class="space-y-1">
                                    <p>{format!("Compared: {}", validation.total_compared.unwrap_or(0))}</p>
                                    <p>{format!("Matches: {}", validation.total_matches.unwrap_or(0))}</p>
                                    <p>{format!("Discrepancies: {}", validation.total_discrepancies.unwrap_or(0))}</p>
                                </div>
                            }),
                            _ => Either::Right(view! {
                                <p class="text-xs text-huly-muted">{validation.message.clone().unwrap_or_else(|| "No payroll data available for the window.".to_string())}</p>
                            }),
                        }}
                    </div>
                </div>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
                <div class="panel">
                    <div class="toolbar flex items-center justify-between">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Audit Alerts"</h3>
                        <a href="/finance/audit-reports" class="text-xs text-primary-400 hover:text-primary-300">"Open report →"</a>
                    </div>
                    <div class="p-3 text-sm text-huly-content space-y-1">
                        <p>{format!("Access denied: {}", audit.access_denied_count)}</p>
                        <p>{format!("Login failed: {}", audit.login_failed_count)}</p>
                        <p>{format!("Login blocked: {}", audit.login_blocked_count)}</p>
                        <p>{format!("Chain failures: {}", audit.chain_verification_failure_count)}</p>
                        {if audit.recent.is_empty() {
                            Either::Left(view! {
                                <p class="text-xs text-huly-muted">"No recent alert events."</p>
                            })
                        } else {
                            Either::Right(view! {
                                <div>
                                    {audit.recent.into_iter().map(|e| view! {
                                        <div class="activity-item">
                                            <span class="text-sm text-huly-content flex-1">{format!("{} · {}", e.action, e.entity_type)}</span>
                                            <span class="text-xs text-huly-muted whitespace-nowrap">{format_date(&e.created_at)}</span>
                                        </div>
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>
                </div>

                <div class="panel">
                    <div class="toolbar flex items-center justify-between">
                        <h3 class="text-xs font-semibold text-huly-secondary uppercase tracking-wider">"Pending Export Requests"</h3>
                        <a href="/finance/audit-reports" class="text-xs text-primary-400 hover:text-primary-300">"Open report →"</a>
                    </div>
                    <div class="p-3">
                        {if exports.latest_pending.is_empty() {
                            Either::Left(view! {
                                <div class="empty-state py-4">
                                    <p class="text-huly-muted text-xs">"No pending export requests."</p>
                                </div>
                            })
                        } else {
                            Either::Right(view! {
                                <div>
                                    {exports.latest_pending.into_iter().map(|p| {
                                        let label = format!("{} · {}", p.report_type.unwrap_or_else(|| "generic".to_string()), p.status);
                                        view! {
                                            <div class="activity-item">
                                                <span class="text-sm text-huly-content flex-1">{label}</span>
                                                <span class="text-xs text-huly-muted whitespace-nowrap">{format_date(&p.created_at)}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        }}
                    </div>
                </div>
            </div>
        </div>
    }
}

// ── Admin Panel ──────────────────────────────────────────────────────────

#[component]
fn AdminPanel(admin: AdminDashboard) -> impl IntoView {
    view! {
        <div class="space-y-4">
            <div class="stat-grid">
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Users"</p>
                        <p class="stat-value">{admin.total_users.to_string()}</p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Departments"</p>
                        <p class="stat-value">{admin.total_departments.to_string()}</p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Active Projects"</p>
                        <p class="stat-value">{admin.total_active_projects.to_string()}</p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Active CTC"</p>
                        <p class="stat-value">{admin.total_active_ctc_records.to_string()}</p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Pending Exports"</p>
                        <p class="stat-value">{admin.pending_export_requests.to_string()}</p>
                    </div>
                </div>
                <div class="stat-card card-hover">
                    <div>
                        <p class="stat-label">"Access Denied (24h)"</p>
                        <p class="stat-value">{admin.access_denied_24h.to_string()}</p>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[allow(dead_code)]
fn _unused_uuid_marker(_id: Uuid) {}
