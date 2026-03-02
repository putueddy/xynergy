use crate::auth::{
    auth_token, authenticated_get, authenticated_post_json, clear_auth_storage, use_auth,
    validate_token, AuthContext,
};
use crate::components::timeline_chart::{AllocationItem, ResourceGroup, TimelineChart};
use crate::components::{Footer, Header};
use crate::timeline::{TimelineGroup, TimelineItem};
use gloo_timers::callback::Timeout;
use js_sys::Date;
use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;

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
    current_allocation_percentage: f64,
    is_overallocated: bool,
    active_assignments: Vec<AssignmentInfo>,
}

#[derive(Clone, Debug, PartialEq)]
struct AssignableProject {
    id: String,
    name: String,
    start_date: String,
    end_date: String,
    status: String,
}

#[derive(Clone, Debug, Serialize)]
struct CreateAssignmentRequest {
    project_id: String,
    resource_id: String,
    start_date: String,
    end_date: String,
    allocation_percentage: f64,
    include_weekend: bool,
    confirm_overallocation: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct OverallocationWarning {
    resource_id: String,
    resource_name: String,
    current_allocation_percentage: f64,
    requested_allocation_percentage: f64,
    projected_allocation_percentage: f64,
    warning_message: String,
    requires_confirmation: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct CapacityPeriod {
    period: String,
    total_allocation_percentage: f64,
    is_overallocated: bool,
    allocation_count: i32,
}

#[derive(Clone, Debug, Deserialize)]
struct EmployeeCapacity {
    resource_id: String,
    resource_name: String,
    periods: Vec<CapacityPeriod>,
}

#[derive(Clone, Debug, Deserialize)]
struct CapacityReportResponse {
    start_date: String,
    end_date: String,
    employees: Vec<EmployeeCapacity>,
}

#[derive(Clone, Debug, Deserialize)]
struct CostPreviewMonthlyBucket {
    month: String,
    working_days: i32,
    cost_idr: i64,
}

#[derive(Clone, Debug, Deserialize)]
struct CostPreviewBudgetImpact {
    department_budget_total_idr: i64,
    current_committed_idr: i64,
    projected_committed_idr: i64,
    remaining_after_assignment_idr: i64,
    utilization_percentage: f64,
    budget_health: String,
}

#[derive(Clone, Debug, Deserialize)]
struct CostPreviewResponse {
    daily_rate_idr: i64,
    working_days: i32,
    allocation_percentage: f64,
    total_cost_idr: i64,
    monthly_breakdown: Vec<CostPreviewMonthlyBucket>,
    budget_impact: Option<CostPreviewBudgetImpact>,
    warning: Option<String>,
    requires_approval: bool,
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
    let response = authenticated_get("/api/v1/team")
        .await
        .map_err(|e| format!("Failed to fetch team: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch team: {}", response.status()));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse team response: {}", e))?;

    let arr = body.as_array().cloned().unwrap_or_default();

    let mut members = Vec::new();
    for item in arr {
        let active_assignments = item
            .get("active_assignments")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|a| AssignmentInfo {
                        project_name: a
                            .get("project_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        allocation_pct: a
                            .get("allocation_pct")
                            .and_then(value_to_f64)
                            .unwrap_or(0.0),
                        start_date: a
                            .get("start_date")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        end_date: a
                            .get("end_date")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        members.push(TeamMember {
            resource_id: item
                .get("resource_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            name: item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            role: item
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            department_name: item
                .get("department_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            daily_rate: item.get("daily_rate").and_then(value_to_i64),
            ctc_status: item
                .get("ctc_status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            total_allocation_pct: item
                .get("total_allocation_pct")
                .and_then(value_to_f64)
                .unwrap_or(0.0),
            current_allocation_percentage: item
                .get("current_allocation_percentage")
                .and_then(value_to_f64)
                .unwrap_or_else(|| {
                    item.get("total_allocation_pct")
                        .and_then(value_to_f64)
                        .unwrap_or(0.0)
                }),
            is_overallocated: item
                .get("is_overallocated")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            active_assignments,
        });
    }

    Ok(members)
}

async fn fetch_assignable_projects() -> Result<Vec<AssignableProject>, String> {
    let response = authenticated_get("/api/v1/projects/assignable")
        .await
        .map_err(|e| format!("Failed to fetch projects: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch assignable projects: {}",
            response.status()
        ));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse projects response: {}", e))?;

    let arr = body.as_array().cloned().unwrap_or_default();
    let projects = arr
        .into_iter()
        .map(|item| AssignableProject {
            id: item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            name: item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            start_date: item
                .get("start_date")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            end_date: item
                .get("end_date")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            status: item
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .collect();
    Ok(projects)
}

async fn fetch_cost_preview(
    resource_id: &str,
    project_id: &str,
    start_date: &str,
    end_date: &str,
    allocation_percentage: f64,
) -> Result<CostPreviewResponse, String> {
    let url = format!(
        "/api/v1/allocations/cost-preview?resource_id={}&project_id={}&start_date={}&end_date={}&allocation_percentage={}&include_weekend=false",
        resource_id, project_id, start_date, end_date, allocation_percentage
    );
    let response = authenticated_get(&url)
        .await
        .map_err(|e| format!("Failed to fetch cost preview: {}", e))?;
    if !response.status().is_success() {
        let body: Value = response.json().await.unwrap_or_default();
        let msg = body
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .or_else(|| body.get("message").and_then(|v| v.as_str()))
            .unwrap_or("Failed to fetch cost preview.");
        return Err(msg.to_string());
    }
    response
        .json::<CostPreviewResponse>()
        .await
        .map_err(|e| format!("Failed to parse cost preview: {}", e))
}

async fn fetch_resource_allocations(resource_id: &str) -> Result<Vec<Value>, String> {
    let url = format!("/api/v1/allocations/resource/{}", resource_id);
    let response = authenticated_get(&url)
        .await
        .map_err(|e| format!("Failed to fetch allocations: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch allocations: {}",
            response.status()
        ));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse allocations: {}", e))?;

    Ok(body.as_array().cloned().unwrap_or_default())
}

async fn fetch_capacity_report(
    start_date: &str,
    end_date: &str,
) -> Result<CapacityReportResponse, String> {
    let url = format!(
        "/api/v1/team/capacity-report?start_date={}&end_date={}",
        start_date, end_date
    );

    let response = authenticated_get(&url)
        .await
        .map_err(|e| format!("Failed to fetch capacity report: {}", e))?;

    if !response.status().is_success() {
        let body: Value = response.json().await.unwrap_or_default();
        let msg = body
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .or_else(|| body.get("message").and_then(|v| v.as_str()))
            .unwrap_or("Failed to fetch capacity report.");
        return Err(msg.to_string());
    }

    response
        .json::<CapacityReportResponse>()
        .await
        .map_err(|e| format!("Failed to parse capacity report: {}", e))
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

fn current_month_range() -> (String, String) {
    let now = Date::new_0();
    let year = now.get_full_year();
    let month = now.get_month() as u32 + 1;

    let start_date = format!("{:04}-{:02}-01", year, month);

    let (next_year, next_month) = if month == 12 {
        (year + 1, 1u32)
    } else {
        (year, month + 1)
    };
    let next_month_start = Date::new_with_year_month_day(next_year, next_month as i32 - 1, 1);
    let last_day_current_month = Date::new_with_year_month_day(
        next_month_start.get_full_year(),
        next_month_start.get_month() as i32,
        0,
    );
    let end_date = format!(
        "{:04}-{:02}-{:02}",
        year,
        month,
        last_day_current_month.get_date() as u32
    );

    (start_date, end_date)
}

fn allocation_color(pct: f64) -> &'static str {
    if pct > 100.0 {
        "text-red-800 dark:text-red-300 font-bold"
    } else if pct >= 81.0 {
        "text-yellow-600 dark:text-yellow-400"
    } else {
        "text-green-600 dark:text-green-400"
    }
}

fn budget_health_color(health: &str) -> &'static str {
    match health {
        "healthy" => "bg-green-500",
        "warning" => "bg-yellow-500",
        "critical" => "bg-red-500",
        _ => "bg-gray-400",
    }
}

fn budget_health_text_color(health: &str) -> &'static str {
    match health {
        "healthy" => "text-green-600 dark:text-green-400",
        "warning" => "text-yellow-600 dark:text-yellow-400",
        "critical" => "text-red-600 dark:text-red-400",
        _ => "text-gray-600 dark:text-gray-400",
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

    // Assignment modal state
    let (show_assign_modal, set_show_assign_modal) = create_signal(false);
    let (assign_resource_id, set_assign_resource_id) = create_signal(String::new());
    let (assign_resource_name, set_assign_resource_name) = create_signal(String::new());
    let (assign_project_id, set_assign_project_id) = create_signal(String::new());
    let (assign_start_date, set_assign_start_date) = create_signal(String::new());
    let (assign_end_date, set_assign_end_date) = create_signal(String::new());
    let (assign_pct, set_assign_pct) = create_signal(String::new());
    let (assign_error, set_assign_error) = create_signal(None::<String>);
    let (assign_success, set_assign_success) = create_signal(None::<String>);
    let (assign_submitting, set_assign_submitting) = create_signal(false);
    let (assignable_projects, set_assignable_projects) =
        create_signal(Vec::<AssignableProject>::new());

    let (preview_data, set_preview_data) = create_signal(None::<CostPreviewResponse>);
    let (preview_loading, set_preview_loading) = create_signal(false);
    let (preview_error, set_preview_error) = create_signal(None::<String>);

    let (overallocation_warning, set_overallocation_warning) =
        create_signal(None::<OverallocationWarning>);
    let (show_confirm_overallocation, set_show_confirm_overallocation) = create_signal(false);
    let (confirm_submitting, set_confirm_submitting) = create_signal(false);

    let (capacity_start_date, set_capacity_start_date) = create_signal(String::new());
    let (capacity_end_date, set_capacity_end_date) = create_signal(String::new());
    let (capacity_loading, set_capacity_loading) = create_signal(false);
    let (capacity_error, set_capacity_error) = create_signal(None::<String>);
    let (capacity_report, set_capacity_report) = create_signal(None::<CapacityReportResponse>);

    let preview_timer: Rc<RefCell<Option<Timeout>>> = Rc::new(RefCell::new(None));
    {
        let preview_timer = preview_timer.clone();
        create_effect(move |_| {
            let resource_id = assign_resource_id.get();
            let project_id = assign_project_id.get();
            let start_date = assign_start_date.get();
            let end_date = assign_end_date.get();
            let pct_str = assign_pct.get();

            preview_timer.borrow_mut().take();

            if resource_id.is_empty()
                || project_id.is_empty()
                || start_date.is_empty()
                || end_date.is_empty()
                || pct_str.is_empty()
            {
                set_preview_data.set(None);
                set_preview_error.set(None);
                set_preview_loading.set(false);
                return;
            }

            let pct: f64 = match pct_str.parse() {
                Ok(v) if v > 0.0 && v <= 100.0 => v,
                _ => {
                    set_preview_data.set(None);
                    set_preview_error.set(None);
                    set_preview_loading.set(false);
                    return;
                }
            };

            set_preview_loading.set(true);
            let preview_timer_inner = preview_timer.clone();
            let timeout = Timeout::new(300, move || {
                preview_timer_inner.borrow_mut().take();
                spawn_local(async move {
                    match fetch_cost_preview(&resource_id, &project_id, &start_date, &end_date, pct)
                        .await
                    {
                        Ok(data) => {
                            set_preview_data.set(Some(data));
                            set_preview_error.set(None);
                        }
                        Err(e) => {
                            set_preview_data.set(None);
                            set_preview_error.set(Some(e));
                        }
                    }
                    set_preview_loading.set(false);
                });
            });
            *preview_timer.borrow_mut() = Some(timeout);
        });
    }

    let capacity_timer: Rc<RefCell<Option<Timeout>>> = Rc::new(RefCell::new(None));
    {
        let capacity_timer = capacity_timer.clone();
        create_effect(move |_| {
            let token_present = auth.token.get().is_some();
            let start_date = capacity_start_date.get();
            let end_date = capacity_end_date.get();

            capacity_timer.borrow_mut().take();

            if !token_present || start_date.is_empty() || end_date.is_empty() {
                if start_date.is_empty() || end_date.is_empty() {
                    set_capacity_report.set(None);
                }
                set_capacity_error.set(None);
                set_capacity_loading.set(false);
                return;
            }

            set_capacity_loading.set(true);
            let capacity_timer_inner = capacity_timer.clone();
            let timeout = Timeout::new(300, move || {
                capacity_timer_inner.borrow_mut().take();
                spawn_local(async move {
                    match fetch_capacity_report(&start_date, &end_date).await {
                        Ok(report) => {
                            set_capacity_report.set(Some(report));
                            set_capacity_error.set(None);
                        }
                        Err(e) => {
                            set_capacity_report.set(None);
                            set_capacity_error.set(Some(e));
                        }
                    }
                    set_capacity_loading.set(false);
                });
            });

            *capacity_timer.borrow_mut() = Some(timeout);
        });
    }

    // Timeline modal state
    let (show_timeline_modal, set_show_timeline_modal) = create_signal(false);
    let (timeline_resource_name, set_timeline_resource_name) = create_signal(String::new());
    let (timeline_groups, set_timeline_groups) = create_signal(Vec::<TimelineGroup>::new());
    let (timeline_items, set_timeline_items) = create_signal(Vec::<TimelineItem>::new());

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

    // Can current user create assignments?
    let can_assign = Signal::derive(move || {
        auth.user
            .get()
            .map(|u| {
                u.role == "department_head" || u.role == "project_manager" || u.role == "admin"
            })
            .unwrap_or(false)
    });

    create_effect(move |_| {
        if auth.token.get().is_some() {
            if !is_authorized.get() {
                return;
            }

            if capacity_start_date.get().is_empty() || capacity_end_date.get().is_empty() {
                let (start, end) = current_month_range();
                set_capacity_start_date.set(start);
                set_capacity_end_date.set(end);
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

        let mut filtered: Vec<TeamMember> = members
            .into_iter()
            .filter(|m| match filter.as_str() {
                "active" => m.ctc_status == "Active",
                "missing" => m.ctc_status == "Missing",
                _ => true,
            })
            .collect();

        match sort.as_str() {
            "rate" => {
                filtered.sort_by(|a, b| b.daily_rate.unwrap_or(0).cmp(&a.daily_rate.unwrap_or(0)))
            }
            "allocation" => filtered.sort_by(|a, b| {
                b.current_allocation_percentage
                    .partial_cmp(&a.current_allocation_percentage)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
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
        if count > 0 {
            sum / count
        } else {
            0
        }
    });

    let avg_allocation = Signal::derive(move || {
        let members = team_members.get();
        if members.is_empty() {
            return 0.0;
        }
        let sum: f64 = members
            .iter()
            .map(|m| m.current_allocation_percentage)
            .sum();
        sum / members.len() as f64
    });

    let missing_ctc_count = Signal::derive(move || {
        team_members
            .get()
            .iter()
            .filter(|m| m.ctc_status == "Missing")
            .count()
    });

    // Open assignment modal for a resource
    let open_assign_modal = move |resource_id: String, resource_name: String| {
        set_assign_resource_id.set(resource_id);
        set_assign_resource_name.set(resource_name);
        set_assign_project_id.set(String::new());
        set_assign_start_date.set(String::new());
        set_assign_end_date.set(String::new());
        set_assign_pct.set(String::new());
        set_assign_error.set(None);
        set_assign_success.set(None);
        set_assign_submitting.set(false);
        set_preview_data.set(None);
        set_preview_loading.set(false);
        set_preview_error.set(None);
        set_overallocation_warning.set(None);
        set_show_confirm_overallocation.set(false);
        set_confirm_submitting.set(false);
        set_show_assign_modal.set(true);

        // Fetch assignable projects
        spawn_local(async move {
            match fetch_assignable_projects().await {
                Ok(projects) => set_assignable_projects.set(projects),
                Err(e) => set_assign_error.set(Some(e)),
            }
        });
    };

    // Submit assignment
    let submit_assignment = move |_| {
        let project_id = assign_project_id.get();
        let start_date = assign_start_date.get();
        let end_date = assign_end_date.get();
        let pct_str = assign_pct.get();
        let resource_id = assign_resource_id.get();

        // Client-side validation
        if project_id.is_empty() {
            set_assign_error.set(Some("Please select a project.".to_string()));
            return;
        }
        if start_date.is_empty() || end_date.is_empty() {
            set_assign_error.set(Some("Start date and end date are required.".to_string()));
            return;
        }
        let pct: f64 = match pct_str.parse() {
            Ok(v) => v,
            Err(_) => {
                set_assign_error.set(Some("Allocation percentage must be a number.".to_string()));
                return;
            }
        };
        if pct <= 0.0 || pct > 100.0 {
            set_assign_error.set(Some(
                "Allocation percentage must be > 0 and <= 100.".to_string(),
            ));
            return;
        }

        set_assign_error.set(None);
        set_assign_success.set(None);
        set_assign_submitting.set(true);
        set_overallocation_warning.set(None);
        set_show_confirm_overallocation.set(false);

        let payload = CreateAssignmentRequest {
            project_id,
            resource_id,
            start_date,
            end_date,
            allocation_percentage: pct,
            include_weekend: false,
            confirm_overallocation: false,
        };

        spawn_local(async move {
            let result = authenticated_post_json("/api/v1/allocations", &payload).await;

            match result {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let body: Value = resp.json().await.unwrap_or_default();
                        match body
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("created")
                        {
                            "overallocation_warning" => {
                                let warning = OverallocationWarning {
                                    resource_id: body
                                        .get("resource_id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or_default()
                                        .to_string(),
                                    resource_name: body
                                        .get("resource_name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or_default()
                                        .to_string(),
                                    current_allocation_percentage: body
                                        .get("current_allocation_percentage")
                                        .and_then(value_to_f64)
                                        .unwrap_or(0.0),
                                    requested_allocation_percentage: body
                                        .get("requested_allocation_percentage")
                                        .and_then(value_to_f64)
                                        .unwrap_or(0.0),
                                    projected_allocation_percentage: body
                                        .get("projected_allocation_percentage")
                                        .and_then(value_to_f64)
                                        .unwrap_or(0.0),
                                    warning_message: body
                                        .get("warning_message")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("Total allocation would exceed 100%.")
                                        .to_string(),
                                    requires_confirmation: body
                                        .get("requires_confirmation")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(true),
                                };
                                set_overallocation_warning.set(Some(warning));
                                set_show_confirm_overallocation.set(true);
                                set_assign_error.set(None);
                                set_assign_success.set(None);
                            }
                            _ => {
                                set_assign_success
                                    .set(Some("Assignment created successfully.".to_string()));
                                set_assign_error.set(None);
                                set_preview_data.set(None);
                                set_preview_loading.set(false);
                                set_preview_error.set(None);
                                set_overallocation_warning.set(None);
                                set_show_confirm_overallocation.set(false);

                                if let Ok(members) = fetch_team_members().await {
                                    set_team_members.set(members);
                                }
                            }
                        }
                    } else {
                        let body: Value = resp.json().await.unwrap_or_default();
                        let msg = body
                            .pointer("/error/message")
                            .and_then(|v| v.as_str())
                            .or_else(|| body.get("message").and_then(|v| v.as_str()))
                            .unwrap_or("Failed to create assignment.");
                        set_assign_error.set(Some(msg.to_string()));
                    }
                }
                Err(e) => {
                    set_assign_error.set(Some(e));
                }
            }
            set_assign_submitting.set(false);
        });
    };

    let confirm_overallocation_assignment = move |_| {
        let project_id = assign_project_id.get();
        let start_date = assign_start_date.get();
        let end_date = assign_end_date.get();
        let resource_id = assign_resource_id.get();
        let pct: f64 = match assign_pct.get().parse() {
            Ok(v) => v,
            Err(_) => {
                set_assign_error.set(Some("Allocation percentage must be a number.".to_string()));
                return;
            }
        };

        let payload = CreateAssignmentRequest {
            project_id,
            resource_id,
            start_date,
            end_date,
            allocation_percentage: pct,
            include_weekend: false,
            confirm_overallocation: true,
        };

        set_confirm_submitting.set(true);
        set_assign_error.set(None);

        spawn_local(async move {
            match authenticated_post_json("/api/v1/allocations", &payload).await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let body: Value = resp.json().await.unwrap_or_default();
                        let status = body
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("created");

                        if status == "created" {
                            set_assign_success.set(Some(
                                "Assignment created successfully with over-allocation confirmation."
                                    .to_string(),
                            ));
                            set_show_confirm_overallocation.set(false);
                            set_overallocation_warning.set(None);
                            set_preview_data.set(None);
                            set_preview_loading.set(false);
                            set_preview_error.set(None);

                            if let Ok(members) = fetch_team_members().await {
                                set_team_members.set(members);
                            }
                        } else {
                            set_assign_error.set(Some(
                                "Unexpected response while confirming assignment.".to_string(),
                            ));
                        }
                    } else {
                        let body: Value = resp.json().await.unwrap_or_default();
                        let msg = body
                            .pointer("/error/message")
                            .and_then(|v| v.as_str())
                            .or_else(|| body.get("message").and_then(|v| v.as_str()))
                            .unwrap_or("Failed to confirm over-allocation.");
                        set_assign_error.set(Some(msg.to_string()));
                    }
                }
                Err(e) => set_assign_error.set(Some(e)),
            }

            set_confirm_submitting.set(false);
        });
    };

    // Open timeline modal for a resource
    let open_timeline_modal = move |resource_id: String, resource_name: String, total_pct: f64| {
        set_timeline_resource_name.set(resource_name.clone());
        set_timeline_groups.set(Vec::new());
        set_timeline_items.set(Vec::new());
        set_show_timeline_modal.set(true);

        let rg = ResourceGroup {
            id: resource_id.clone(),
            name: resource_name.clone(),
            allocation_percentage: total_pct,
        };
        set_timeline_groups.set(vec![rg.to_timeline_group()]);

        spawn_local(async move {
            match fetch_resource_allocations(&resource_id).await {
                Ok(allocs) => {
                    let items: Vec<TimelineItem> = allocs
                        .iter()
                        .filter_map(|a| {
                            let item = AllocationItem {
                                id: a.get("id").and_then(|v| v.as_str())?.to_string(),
                                resource_id: a
                                    .get("resource_id")
                                    .and_then(|v| v.as_str())?
                                    .to_string(),
                                project_name: a
                                    .get("project_name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown")
                                    .to_string(),
                                start: a.get("start_date").and_then(|v| v.as_str())?.to_string(),
                                end: a.get("end_date").and_then(|v| v.as_str())?.to_string(),
                                percentage: a
                                    .get("allocation_percentage")
                                    .and_then(value_to_f64)
                                    .unwrap_or(0.0),
                            };
                            Some(item.to_timeline_item())
                        })
                        .collect();
                    set_timeline_items.set(items);
                }
                Err(e) => {
                    web_sys::console::log_1(&format!("Timeline fetch error: {}", e).into());
                }
            }
        });
    };

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
                                                        None => "\u{2014}".to_string(),
                                                    };

                                                    let alloc_class = if is_missing {
                                                        "text-gray-400 font-mono text-right".to_string()
                                                    } else {
                                                        format!(
                                                            "font-mono text-right {}",
                                                            allocation_color(
                                                                m.current_allocation_percentage
                                                            )
                                                        )
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
                                                        "\u{2014}".to_string()
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
                                                    } else if m.is_overallocated {
                                                        "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300 border-red-200"
                                                    } else {
                                                        "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300 border-green-200"
                                                    };

                                                    let display_status = if is_missing {
                                                        "CTC Missing"
                                                    } else if m.is_overallocated {
                                                        "Overallocated"
                                                    } else {
                                                        "Active"
                                                    };

                                                    // Capture values for closures
                                                    let rid_assign = m.resource_id.clone();
                                                    let rname_assign = m.name.clone();
                                                    let rid_timeline = m.resource_id.clone();
                                                    let rname_timeline = m.name.clone();
                                                    let total_pct = m.current_allocation_percentage;
                                                    let has_assignments = !m.active_assignments.is_empty();

                                                    view! {
                                                        <tr class=row_class>
                                                            <td class="px-4 py-3 text-sm font-medium">{m.name.clone()}</td>
                                                            <td class="px-4 py-3 text-sm">{m.role.clone()}</td>
                                                            <td class="px-4 py-3 text-sm font-mono text-right">{rate_text}</td>
                                                            <td class=format!("px-4 py-3 text-sm {}", alloc_class)>{format!("{:.1}%", m.current_allocation_percentage)}</td>
                                                            <td class="px-4 py-3 text-sm" title=projects_title>{projects_display}</td>
                                                            <td class="px-4 py-3 text-sm text-center">
                                                                <span class=format!("px-2 py-1 inline-flex text-xs leading-5 font-semibold rounded-full border {}", status_badge)>
                                                                    {display_status}
                                                                </span>
                                                            </td>
                                                            <td class="px-4 py-3 text-sm text-center">
                                                                <div class="flex items-center justify-center gap-2">
                                                                    {if is_missing {
                                                                        view! {
                                                                            <button
                                                                                disabled=true
                                                                                title="CTC data required to assign. Contact HR to complete employee setup."
                                                                                class="px-3 py-1 text-xs font-medium rounded bg-gray-200 text-gray-400 cursor-not-allowed dark:bg-gray-700 dark:text-gray-500"
                                                                            >
                                                                                "Assign"
                                                                            </button>
                                                                        }.into_view()
                                                                    } else if can_assign.get() {
                                                                        view! {
                                                                            <button
                                                                                class="px-3 py-1 text-xs font-medium rounded bg-blue-600 text-white hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600"
                                                                                on:click=move |_| open_assign_modal(rid_assign.clone(), rname_assign.clone())
                                                                            >
                                                                                "Assign"
                                                                            </button>
                                                                        }.into_view()
                                                                    } else {
                                                                        view! { <span></span> }.into_view()
                                                                    }}
                                                                    {if has_assignments {
                                                                        view! {
                                                                            <button
                                                                                class="px-3 py-1 text-xs font-medium rounded bg-indigo-600 text-white hover:bg-indigo-700 dark:bg-indigo-500 dark:hover:bg-indigo-600"
                                                                                on:click=move |_| open_timeline_modal(rid_timeline.clone(), rname_timeline.clone(), total_pct)
                                                                            >
                                                                                "View Timeline"
                                                                            </button>
                                                                        }.into_view()
                                                                    } else {
                                                                        view! { <span></span> }.into_view()
                                                                    }}
                                                                </div>
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

                            <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-6 space-y-4">
                                <div class="flex flex-col md:flex-row md:items-end md:justify-between gap-3">
                                    <div>
                                        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">
                                            "Department Capacity Report"
                                        </h2>
                                        <p class="text-sm text-gray-500 dark:text-gray-400">
                                            "Utilization by month with overallocated periods highlighted"
                                        </p>
                                    </div>
                                    <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                                        <div>
                                            <label class="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1">
                                                "Start Date"
                                            </label>
                                            <input
                                                type="date"
                                                class="border rounded px-3 py-2 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                                                prop:value=capacity_start_date
                                                on:input=move |ev| set_capacity_start_date.set(event_target_value(&ev))
                                            />
                                        </div>
                                        <div>
                                            <label class="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1">
                                                "End Date"
                                            </label>
                                            <input
                                                type="date"
                                                class="border rounded px-3 py-2 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                                                prop:value=capacity_end_date
                                                on:input=move |ev| set_capacity_end_date.set(event_target_value(&ev))
                                            />
                                        </div>
                                    </div>
                                </div>

                                {move || capacity_error.get().map(|err| view! {
                                    <div class="rounded-md bg-red-50 p-3 dark:bg-red-900/20 text-red-800 dark:text-red-200 text-sm">{err}</div>
                                })}

                                {move || capacity_loading.get().then(|| view! {
                                    <div class="text-sm text-gray-500 dark:text-gray-400">"Loading capacity report..."</div>
                                })}

                                {move || {
                                    match capacity_report.get() {
                                        None => view! { <span></span> }.into_view(),
                                        Some(report) => {
                                            let periods: Vec<String> = report
                                                .employees
                                                .first()
                                                .map(|e| e.periods.iter().map(|p| p.period.clone()).collect())
                                                .unwrap_or_else(Vec::new);

                                            view! {
                                                <div class="space-y-2">
                                                    <div class="text-xs text-gray-500 dark:text-gray-400">
                                                        {format!("Range: {} to {}", report.start_date, report.end_date)}
                                                    </div>
                                                    <div class="overflow-x-auto">
                                                        <table class="min-w-full text-sm border border-gray-200 dark:border-gray-700">
                                                            <thead class="bg-gray-50 dark:bg-gray-700">
                                                                <tr>
                                                                    <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase">"Employee"</th>
                                                                    {periods.iter().map(|period| view! {
                                                                        <th class="px-3 py-2 text-right text-xs font-medium text-gray-500 uppercase">{period.clone()}</th>
                                                                    }).collect::<Vec<_>>()}
                                                                </tr>
                                                            </thead>
                                                            <tbody>
                                                                {report.employees.iter().map(|employee| {
                                                                    let row_name = employee.resource_name.clone();
                                                                    view! {
                                                                        <tr class="border-t border-gray-200 dark:border-gray-700">
                                                                            <td class="px-3 py-2 font-medium text-gray-900 dark:text-gray-100">{row_name}</td>
                                                                            {employee.periods.iter().map(|period| {
                                                                                let pct = period.total_allocation_percentage;
                                                                                let cell_class = if period.is_overallocated {
                                                                                    "bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-300"
                                                                                } else if pct >= 80.0 {
                                                                                    "bg-yellow-50 dark:bg-yellow-900/20 text-yellow-700 dark:text-yellow-300"
                                                                                } else {
                                                                                    "bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-300"
                                                                                };

                                                                                view! {
                                                                                    <td class=format!("px-3 py-2 text-right font-mono {}", cell_class) title=format!("{} allocations", period.allocation_count)>
                                                                                        {format!("{:.1}%", pct)}
                                                                                    </td>
                                                                                }
                                                                            }).collect::<Vec<_>>()}
                                                                        </tr>
                                                                    }
                                                                }).collect::<Vec<_>>()}
                                                            </tbody>
                                                        </table>
                                                    </div>
                                                </div>
                                            }
                                                .into_view()
                                        }
                                    }
                                }}
                            </div>
                        </div>
                    }.into_view()
                }}
            </main>

            // Assignment Modal
            {move || show_assign_modal.get().then(|| view! {
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full max-w-lg mx-4 p-6 space-y-4">
                        <div class="flex items-center justify-between">
                            <h2 class="text-xl font-bold text-gray-900 dark:text-white">
                                "Assign to Project"
                            </h2>
                            <button
                                class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                                on:click=move |_| {
                                    set_show_assign_modal.set(false);
                                    set_show_confirm_overallocation.set(false);
                                    set_overallocation_warning.set(None);
                                }
                            >
                                "\u{2715}"
                            </button>
                        </div>

                        <p class="text-sm text-gray-600 dark:text-gray-400">
                            "Assigning: "
                            <span class="font-semibold text-gray-900 dark:text-white">{move || assign_resource_name.get()}</span>
                        </p>

                        {move || assign_error.get().map(|err| view! {
                            <div class="rounded-md bg-red-50 p-3 dark:bg-red-900/20 text-red-800 dark:text-red-200 text-sm">{err}</div>
                        })}

                        {move || assign_success.get().map(|msg| view! {
                            <div class="rounded-md bg-green-50 p-3 dark:bg-green-900/20 text-green-800 dark:text-green-200 text-sm">{msg}</div>
                        })}

                        <div class="space-y-3">
                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Project"</label>
                                <select
                                    class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                                    prop:value=assign_project_id
                                    on:change=move |ev| {
                                        let project_id = event_target_value(&ev);
                                        set_assign_project_id.set(project_id.clone());
                                        // Auto-fill project date range
                                        let projects = assignable_projects.get();
                                        if let Some(proj) = projects.iter().find(|p| p.id == project_id) {
                                            if assign_start_date.get().is_empty() {
                                                set_assign_start_date.set(proj.start_date.clone());
                                            }
                                            if assign_end_date.get().is_empty() {
                                                set_assign_end_date.set(proj.end_date.clone());
                                            }
                                        }
                                    }
                                >
                                    <option value="">"Select a project..."</option>
                                    <For
                                        each=move || assignable_projects.get()
                                        key=|p| p.id.clone()
                                        children=move |p| {
                                            let label = format!("{} ({} \u{2014} {})", p.name, p.start_date, p.end_date);
                                            let pid = p.id.clone();
                                            view! { <option value=pid>{label}</option> }
                                        }
                                    />
                                </select>
                            </div>

                            <div class="grid grid-cols-2 gap-3">
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Start Date"</label>
                                    <input
                                        type="date"
                                        class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                                        prop:value=assign_start_date
                                        on:input=move |ev| set_assign_start_date.set(event_target_value(&ev))
                                    />
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"End Date"</label>
                                    <input
                                        type="date"
                                        class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                                        prop:value=assign_end_date
                                        on:input=move |ev| set_assign_end_date.set(event_target_value(&ev))
                                    />
                                </div>
                            </div>

                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Allocation %"</label>
                                <input
                                    type="number"
                                    min="1"
                                    max="100"
                                    step="1"
                                    placeholder="e.g. 50"
                                    class="w-full border rounded px-3 py-2 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                                    prop:value=assign_pct
                                    on:input=move |ev| set_assign_pct.set(event_target_value(&ev))
                                />
                            </div>
                        </div>

                        {move || {
                            if preview_loading.get() && preview_data.get().is_none() {
                                return view! {
                                    <div class="mt-4 p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg animate-pulse space-y-3">
                                        <div class="h-4 bg-gray-200 dark:bg-gray-600 rounded w-1/3"></div>
                                        <div class="h-8 bg-gray-200 dark:bg-gray-600 rounded w-1/2"></div>
                                        <div class="h-4 bg-gray-200 dark:bg-gray-600 rounded w-2/3"></div>
                                    </div>
                                }
                                .into_view();
                            }

                            if let Some(err) = preview_error.get() {
                                return view! {
                                    <div class="mt-4 rounded-md bg-red-50 p-3 dark:bg-red-900/20 text-red-800 dark:text-red-200 text-sm">
                                        {format!("Preview error: {}", err)}
                                    </div>
                                }
                                .into_view();
                            }

                            match preview_data.get() {
                                None => view! { <span></span> }.into_view(),
                                Some(data) => {
                                    let total_cost_formatted = format_idr(data.total_cost_idr);
                                    let formula_tooltip = format!(
                                        "{} × {} days × {}% = {}",
                                        format_idr(data.daily_rate_idr),
                                        data.working_days,
                                        data.allocation_percentage,
                                        format_idr(data.total_cost_idr)
                                    );
                                    let loading_now = preview_loading.get();
                                    let monthly = data.monthly_breakdown.clone();
                                    let budget = data.budget_impact.clone();
                                    let warning = data.warning.clone();
                                    let requires_approval = data.requires_approval;

                                    view! {
                                        <div class=format!("mt-4 p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg space-y-4 border border-blue-200 dark:border-blue-800 {}", if loading_now { "opacity-60" } else { "" })>
                                            <h3 class="text-sm font-semibold text-blue-900 dark:text-blue-200">"Cost Impact Preview"</h3>

                                            <div class="relative group">
                                                <div class="text-xs text-gray-500 dark:text-gray-400">"Total Cost"</div>
                                                <div class="text-2xl font-bold text-gray-900 dark:text-white font-mono">{total_cost_formatted}</div>
                                                <div class="text-xs text-gray-500 dark:text-gray-400">
                                                    {format!("{} daily rate × {} working days × {}%", format_idr(data.daily_rate_idr), data.working_days, data.allocation_percentage)}
                                                </div>
                                                <div class="absolute z-50 px-2 py-1 bg-gray-900 text-white text-xs rounded opacity-0 group-hover:opacity-100 transition-opacity bottom-full mb-1 whitespace-nowrap pointer-events-none">
                                                    {formula_tooltip}
                                                </div>
                                            </div>

                                            {if !monthly.is_empty() {
                                                view! {
                                                    <div>
                                                        <div class="text-xs font-medium text-gray-500 dark:text-gray-400 mb-2">"Monthly Breakdown"</div>
                                                        <table class="w-full text-sm">
                                                            <thead>
                                                                <tr class="text-xs text-gray-500 dark:text-gray-400">
                                                                    <th class="text-left py-1">"Month"</th>
                                                                    <th class="text-right py-1">"Working Days"</th>
                                                                    <th class="text-right py-1">"Cost (IDR)"</th>
                                                                </tr>
                                                            </thead>
                                                            <tbody>
                                                                {monthly
                                                                    .iter()
                                                                    .map(|bucket| {
                                                                        let month = bucket.month.clone();
                                                                        let days = bucket.working_days;
                                                                        let cost = format_idr(bucket.cost_idr);
                                                                        view! {
                                                                            <tr class="text-gray-900 dark:text-gray-100">
                                                                                <td class="py-1">{month}</td>
                                                                                <td class="text-right py-1 font-mono">{days}</td>
                                                                                <td class="text-right py-1 font-mono">{cost}</td>
                                                                            </tr>
                                                                        }
                                                                    })
                                                                    .collect::<Vec<_>>()}
                                                            </tbody>
                                                        </table>
                                                    </div>
                                                }
                                                    .into_view()
                                            } else {
                                                view! { <span></span> }.into_view()
                                            }}

                                            {match budget {
                                                Some(bi) => {
                                                    let bar_width = if bi.utilization_percentage > 100.0 {
                                                        100.0
                                                    } else {
                                                        bi.utilization_percentage
                                                    };
                                                    let health_color =
                                                        budget_health_color(&bi.budget_health);
                                                    let text_color =
                                                        budget_health_text_color(&bi.budget_health);
                                                    let remaining =
                                                        format_idr(bi.remaining_after_assignment_idr);
                                                    let budget_total =
                                                        format_idr(bi.department_budget_total_idr);
                                                    let current_committed =
                                                        format_idr(bi.current_committed_idr);
                                                    let projected_committed =
                                                        format_idr(bi.projected_committed_idr);
                                                    view! {
                                                        <div>
                                                            <div class="text-xs font-medium text-gray-500 dark:text-gray-400 mb-2">"Department Budget Impact"</div>
                                                            <div class="w-full bg-gray-200 dark:bg-gray-600 rounded-full h-3 mb-2">
                                                                <div
                                                                    class=format!("{} h-3 rounded-full transition-all", health_color)
                                                                    style=format!("width: {}%", bar_width)
                                                                ></div>
                                                            </div>
                                                            <div class="flex justify-between text-xs">
                                                                <span class=text_color>{format!("{:.1}% utilized", bi.utilization_percentage)}</span>
                                                                <span class="text-gray-500 dark:text-gray-400">{format!("Remaining: {}", remaining)}</span>
                                                            </div>
                                                            <div class="mt-1 space-y-1 text-[11px] text-gray-500 dark:text-gray-400">
                                                                <div>{format!("Current committed: {}", current_committed)}</div>
                                                                <div>{format!("Projected committed: {} / Budget: {}", projected_committed, budget_total)}</div>
                                                            </div>
                                                        </div>
                                                    }
                                                        .into_view()
                                                }
                                                None => {
                                                    view! {
                                                        <div class="text-xs text-gray-400 dark:text-gray-500 italic">
                                                            "Department budget not configured."
                                                        </div>
                                                    }
                                                        .into_view()
                                                }
                                            }}

                                            {match warning {
                                                Some(w) => {
                                                    view! {
                                                        <div class="rounded-md bg-red-100 dark:bg-red-900/30 border border-red-300 dark:border-red-700 p-3 text-sm text-red-800 dark:text-red-200">
                                                            <span class="font-semibold">"⚠ Budget Warning: "</span>
                                                            {w}
                                                            {if requires_approval {
                                                                view! { <div class="mt-1 text-xs font-medium">"Approval required for this assignment."</div> }
                                                                    .into_view()
                                                            } else {
                                                                view! { <span></span> }.into_view()
                                                            }}
                                                        </div>
                                                    }
                                                        .into_view()
                                                }
                                                None => view! { <span></span> }.into_view(),
                                            }}
                                        </div>
                                    }
                                    .into_view()
                                }
                            }
                        }}

                        {move || {
                            preview_data.get().map(|data| {
                                let project_name = {
                                    let pid = assign_project_id.get();
                                    let projects = assignable_projects.get();
                                    projects.iter().find(|p| p.id == pid).map(|p| p.name.clone()).unwrap_or_default()
                                };
                                view! {
                                    <div class="w-full mb-3 p-3 bg-gray-50 dark:bg-gray-700/50 rounded text-xs space-y-1">
                                        <div class="font-semibold text-gray-700 dark:text-gray-300">"Assignment Summary"</div>
                                        <div class="flex justify-between">
                                            <span class="text-gray-500">"Resource:"</span>
                                            <span class="font-medium text-gray-900 dark:text-white">{assign_resource_name.get()}</span>
                                        </div>
                                        <div class="flex justify-between">
                                            <span class="text-gray-500">"Daily Rate:"</span>
                                            <span class="font-mono text-gray-900 dark:text-white">{format_idr(data.daily_rate_idr)}</span>
                                        </div>
                                        <div class="flex justify-between">
                                            <span class="text-gray-500">"Project:"</span>
                                            <span class="font-medium text-gray-900 dark:text-white">{project_name}</span>
                                        </div>
                                        <div class="flex justify-between">
                                            <span class="text-gray-500">"Duration:"</span>
                                            <span class="font-mono text-gray-900 dark:text-white">{format!("{} \u{2014} {} ({} working days)", assign_start_date.get(), assign_end_date.get(), data.working_days)}</span>
                                        </div>
                                        <div class="flex justify-between">
                                            <span class="text-gray-500">"Allocation:"</span>
                                            <span class="font-mono text-gray-900 dark:text-white">{format!("{}%", assign_pct.get())}</span>
                                        </div>
                                        <div class="flex justify-between">
                                            <span class="text-gray-500">"Total Cost:"</span>
                                            <span class="font-mono font-semibold text-gray-900 dark:text-white">{format_idr(data.total_cost_idr)}</span>
                                        </div>
                                        {data.budget_impact.as_ref().map(|bi| {
                                            view! {
                                                <div class="flex justify-between">
                                                    <span class="text-gray-500">"Budget Remaining:"</span>
                                                    <span class=format!("font-mono {}", budget_health_text_color(&bi.budget_health))>
                                                        {format_idr(bi.remaining_after_assignment_idr)}
                                                    </span>
                                                </div>
                                            }
                                        })}
                                    </div>
                                }
                            })
                        }}

                        <div class="flex justify-end gap-3 pt-2">
                            <button
                                class="px-4 py-2 text-sm font-medium rounded border border-gray-300 text-gray-700 bg-white hover:bg-gray-50 dark:bg-gray-700 dark:text-gray-300 dark:border-gray-600 dark:hover:bg-gray-600"
                                on:click=move |_| {
                                    set_show_assign_modal.set(false);
                                    set_show_confirm_overallocation.set(false);
                                    set_overallocation_warning.set(None);
                                }
                            >
                                "Cancel"
                            </button>
                            <button
                                class="px-4 py-2 text-sm font-medium rounded bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
                                prop:disabled=move || assign_submitting.get()
                                on:click=submit_assignment
                            >
                                {move || if assign_submitting.get() { "Submitting..." } else { "Confirm & Create Assignment" }}
                            </button>
                        </div>
                    </div>
                </div>
            })}

            {move || show_confirm_overallocation.get().then(|| {
                let warning = overallocation_warning.get();
                view! {
                    <div class="fixed inset-0 z-[60] flex items-center justify-center bg-black/60">
                        <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full max-w-md mx-4 p-6 space-y-4">
                            <h3 class="text-lg font-semibold text-gray-900 dark:text-white">"Confirm Over-Allocation"</h3>

                            {warning.as_ref().map(|w| view! {
                                <div class="rounded-md bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-300 dark:border-yellow-700 p-3 text-sm text-yellow-900 dark:text-yellow-200 space-y-1">
                                    <div class="font-medium">{w.warning_message.clone()}</div>
                                    <div>{format!("Current: {:.1}%", w.current_allocation_percentage)}</div>
                                    <div>{format!("Requested: {:.1}%", w.requested_allocation_percentage)}</div>
                                    <div class="font-semibold text-red-700 dark:text-red-300">{format!("Projected: {:.1}%", w.projected_allocation_percentage)}</div>
                                </div>
                            })}

                            <div class="flex justify-end gap-3 pt-2">
                                <button
                                    class="px-4 py-2 text-sm font-medium rounded border border-gray-300 text-gray-700 bg-white hover:bg-gray-50 dark:bg-gray-700 dark:text-gray-300 dark:border-gray-600 dark:hover:bg-gray-600"
                                    on:click=move |_| {
                                        set_show_confirm_overallocation.set(false);
                                        set_confirm_submitting.set(false);
                                    }
                                >
                                    "Cancel"
                                </button>
                                <button
                                    class="px-4 py-2 text-sm font-medium rounded bg-red-600 text-white hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed"
                                    prop:disabled=move || confirm_submitting.get()
                                    on:click=confirm_overallocation_assignment
                                >
                                    {move || if confirm_submitting.get() { "Confirming..." } else { "Confirm Over-Allocation" }}
                                </button>
                            </div>
                        </div>
                    </div>
                }
            })}

            // Timeline Modal
            {move || show_timeline_modal.get().then(|| view! {
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full max-w-4xl mx-4 p-6 space-y-4">
                        <div class="flex items-center justify-between">
                            <h2 class="text-xl font-bold text-gray-900 dark:text-white">
                                {move || format!("Timeline: {}", timeline_resource_name.get())}
                            </h2>
                            <button
                                class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                                on:click=move |_| {
                                    set_show_timeline_modal.set(false);
                                    set_timeline_groups.set(Vec::new());
                                    set_timeline_items.set(Vec::new());
                                }
                            >
                                "\u{2715}"
                            </button>
                        </div>

                        <div class="min-h-[400px]">
                            <TimelineChart
                                groups=Signal::derive(move || timeline_groups.get())
                                items=Signal::derive(move || timeline_items.get())
                                days_before=30
                                days_after=90
                            />
                        </div>
                    </div>
                </div>
            })}

            <Footer/>
        </div>
    }
}
