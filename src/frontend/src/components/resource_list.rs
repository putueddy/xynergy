use leptos::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Resource data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: Uuid,
    pub name: String,
    pub resource_type: String,
    pub capacity: Option<f64>,
    pub department_id: Option<Uuid>,
    pub department_name: Option<String>,
    pub employment_start_date: Option<String>,
    pub skills: Option<serde_json::Value>,
}

/// Resource list component
#[component]
pub fn ResourceList(
    resources: Signal<Vec<Resource>>,
    on_edit: Callback<Uuid>,
    on_delete: Callback<Uuid>,
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
                            "Type"
                        </th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                            "Capacity"
                        </th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                            "Department"
                        </th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                            "Start Date"
                        </th>
                        <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                            "Actions"
                        </th>
                    </tr>
                </thead>
                <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                    {move || {
                        resources.get().into_iter().map(|resource| {
                            let resource_id = resource.id;
                            view! {
                                <tr class="hover:bg-gray-50 dark:hover:bg-gray-700">
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <div class="text-sm font-medium text-gray-900 dark:text-white">
                                            {resource.name.clone()}
                                        </div>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap">
                                        <span class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200">
                                            {resource.resource_type.clone()}
                                        </span>
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                        {resource.capacity.map(|c| c.to_string()).unwrap_or_else(|| "-".to_string())}
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                        {resource.department_name.clone().or_else(|| resource.department_id.as_ref().map(|id| id.to_string())).unwrap_or_else(|| "-".to_string())}
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                        {resource.employment_start_date.clone().unwrap_or_else(|| "-".to_string())}
                                    </td>
                                    <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                        <button
                                            class="text-blue-600 hover:text-blue-900 dark:text-blue-400 dark:hover:text-blue-300 mr-4"
                                            on:click=move |_| on_edit.call(resource_id)
                                        >
                                            "Edit"
                                        </button>
                                        <button
                                            class="text-red-600 hover:text-red-900 dark:text-red-400 dark:hover:text-red-300"
                                            on:click=move |_| on_delete.call(resource_id)
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
