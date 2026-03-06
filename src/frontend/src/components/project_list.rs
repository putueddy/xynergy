use chrono::NaiveDate;
use leptos::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Project data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub client: Option<String>,
    pub description: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: String,
    pub project_manager_id: Option<Uuid>,
    pub total_budget_idr: i64,
    pub budget_hr_idr: i64,
    pub budget_software_idr: i64,
    pub budget_hardware_idr: i64,
    pub budget_overhead_idr: i64,
}

/// Project list component
#[component]
pub fn ProjectList(
    projects: Signal<Vec<Project>>,
    on_edit: Callback<Uuid>,
    on_delete: Callback<Uuid>,
    on_view_budget: Callback<Uuid>,
    on_view_expenses: Callback<Uuid>,
    on_view_resource_costs: Callback<Uuid>,
    on_view_revenue: Callback<Uuid>,
    on_view_pnl: Callback<Uuid>,
) -> impl IntoView {
    view! {
        <div class="overflow-hidden overflow-x-auto">
            <table class="min-w-full divide-y divide-huly-divider">
                <thead class="bg-huly-surface-2">
                    <tr>
                        <th class="th-cell-compact">
                            "Name"
                        </th>
                        <th class="th-cell-compact">
                            "Client"
                        </th>
                        <th class="th-cell-compact">
                            "Status"
                        </th>
                        <th class="th-cell-compact">
                            "Start Date"
                        </th>
                        <th class="th-cell-compact">
                            "End Date"
                        </th>
                        <th class="th-cell-compact text-right">
                            "Actions"
                        </th>
                    </tr>
                </thead>
                <tbody class="bg-huly-surface divide-y divide-huly-divider">
                    {move || {
                        projects.get().into_iter().map(|project| {
                            let project_id = project.id;
                            let status_color = match project.status.as_str() {
                                "planning" => "badge-warning",
                                "Active" => "badge-positive",
                                "completed" => "badge-primary",
                                "cancelled" => "badge-negative",
                                _ => "badge-neutral",
                            };
                            view! {
                                <tr class="table-row-hover">
                                    <td class="td-cell-compact whitespace-nowrap">
                                        <div class="text-sm font-medium text-huly-caption">
                                            {project.name.clone()}
                                        </div>
                                        {project.description.clone().map(|desc| {
                                            view! {
                                                <div class="text-sm text-huly-muted truncate max-w-xs">
                                                    {desc}
                                                </div>
                                            }
                                        })}
                                    </td>
                                    <td class="td-cell-compact whitespace-nowrap text-sm text-huly-muted">
                                        {project.client.clone().unwrap_or_default()}
                                    </td>
                                    <td class="td-cell-compact whitespace-nowrap">
                                        <span class={format!("px-2 inline-flex text-xs leading-5 font-semibold rounded-full {}", status_color)}>
                                            {project.status.clone()}
                                        </span>
                                    </td>
                                    <td class="td-cell-compact whitespace-nowrap text-sm text-huly-muted">
                                        {project.start_date.to_string()}
                                    </td>
                                    <td class="td-cell-compact whitespace-nowrap text-sm text-huly-muted">
                                        {project.end_date.to_string()}
                                    </td>
                                    <td class="td-cell-compact whitespace-nowrap text-right text-sm font-medium">
                                        <button
                                            class="link mr-3"
                                            on:click=move |_| on_view_expenses.call(project_id)
                                        >
                                            "Expenses"
                                        </button>
                                        <button
                                            class="text-positive-default hover:text-positive-default/80 mr-3"
                                            on:click=move |_| on_view_resource_costs.call(project_id)
                                        >
                                            "Costs"
                                        </button>
                                        <button
                                            class="text-primary-400 hover:text-primary-400/80 mr-3"
                                            on:click=move |_| on_view_revenue.call(project_id)
                                        >
                                            "Revenue"
                                        </button>
                                        <button
                                            class="link-danger mr-3"
                                            on:click=move |_| on_view_pnl.call(project_id)
                                        >
                                            "P&L"
                                        </button>
                                        <button
                                            class="text-accent-orange hover:text-accent-orange/80 mr-3"
                                            on:click=move |_| on_view_budget.call(project_id)
                                        >
                                            "Budget"
                                        </button>
                                        <button
                                            class="link mr-3"
                                            on:click=move |_| on_edit.call(project_id)
                                        >
                                            "Edit"
                                        </button>
                                        <button
                                            class="link-danger"
                                            on:click=move |_| on_delete.call(project_id)
                                        >
                                            "Delete"
                                        </button>
                                    </td>
                                </tr>
                            }
                        }).collect_view()
                    }}
                </tbody>
            </table>
        </div>
    }
}
