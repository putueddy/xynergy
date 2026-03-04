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
) -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 shadow rounded-lg overflow-hidden">
            <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                <thead class="bg-gray-50 dark:bg-gray-700">
                    <tr>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                            "Name"
                        </th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                            "Client"
                        </th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                            "Status"
                        </th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                            "Start Date"
                        </th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                            "End Date"
                        </th>
                        <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                            "Actions"
                        </th>
                    </tr>
                </thead>
                <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                    {move || {
                        projects.get().into_iter().map(|project| {
                            let project_id = project.id;
                            let status_color = match project.status.as_str() {
                                "planning" => "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200",
                                "Active" => "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200",
                                "completed" => "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200",
                                "cancelled" => "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200",
                                _ => "bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200",
                            };
                            view! {
                                <tr class="hover:bg-gray-50 dark:hover:bg-gray-700">
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <div class="text-sm font-medium text-gray-900 dark:text-white">
                                            {project.name.clone()}
                                        </div>
                                        {project.description.clone().map(|desc| {
                                            view! {
                                                <div class="text-sm text-gray-500 dark:text-gray-400 truncate max-w-xs">
                                                    {desc}
                                                </div>
                                            }
                                        })}
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                        {project.client.clone().unwrap_or_default()}
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <span class={format!("px-2 inline-flex text-xs leading-5 font-semibold rounded-full {}", status_color)}>
                                            {project.status.clone()}
                                        </span>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                        {project.start_date.to_string()}
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                        {project.end_date.to_string()}
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                        <button
                                            class="text-indigo-600 hover:text-indigo-900 dark:text-indigo-400 dark:hover:text-indigo-300 mr-4"
                                            on:click=move |_| on_view_expenses.call(project_id)
                                        >
                                            "Expenses"
                                        </button>
                                        <button
                                            class="text-purple-600 hover:text-purple-900 dark:text-purple-400 dark:hover:text-purple-300 mr-4"
                                            on:click=move |_| on_view_budget.call(project_id)
                                        >
                                            "Budget"
                                        </button>
                                        <button
                                            class="text-blue-600 hover:text-blue-900 dark:text-blue-400 dark:hover:text-blue-300 mr-4"
                                            on:click=move |_| on_edit.call(project_id)
                                        >
                                            "Edit"
                                        </button>
                                        <button
                                            class="text-red-600 hover:text-red-900 dark:text-red-400 dark:hover:text-red-300"
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
