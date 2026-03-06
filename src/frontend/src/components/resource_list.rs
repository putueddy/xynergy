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
        <div class="overflow-hidden overflow-x-auto">
            <table class="min-w-full divide-y divide-huly-divider">
                <thead class="bg-huly-surface-2">
                    <tr>
                        <th class="th-cell-compact">
                            "Name"
                        </th>
                        <th class="th-cell-compact">
                            "Type"
                        </th>
                        <th class="th-cell-compact">
                            "Capacity"
                        </th>
                        <th class="th-cell-compact">
                            "Department"
                        </th>
                        <th class="th-cell-compact">
                            "Start Date"
                        </th>
                        <th class="th-cell-compact text-right">
                            "Actions"
                        </th>
                    </tr>
                </thead>
                <tbody class="bg-huly-surface divide-y divide-huly-divider">
                    {move || {
                        resources.get().into_iter().map(|resource| {
                            let resource_id = resource.id;
                            view! {
                                <tr class="table-row-hover">
                                    <td class="td-cell-compact whitespace-nowrap">
                                        <div class="text-sm font-medium text-huly-caption">
                                            {resource.name.clone()}
                                        </div>
                                    </td>
                                    <td class="td-cell-compact whitespace-nowrap">
                                        <span class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full badge-primary">
                                            {resource.resource_type.clone()}
                                        </span>
                                    </td>
                                    <td class="td-cell-compact whitespace-nowrap text-sm text-huly-muted">
                                        {resource.capacity.map(|c| c.to_string()).unwrap_or_else(|| "-".to_string())}
                                    </td>
                                    <td class="td-cell-compact whitespace-nowrap text-sm text-huly-muted">
                                        {resource.department_name.clone().or_else(|| resource.department_id.as_ref().map(|id| id.to_string())).unwrap_or_else(|| "-".to_string())}
                                    </td>
                                    <td class="td-cell-compact whitespace-nowrap text-sm text-huly-muted">
                                        {resource.employment_start_date.clone().unwrap_or_else(|| "-".to_string())}
                                    </td>
                                    <td class="td-cell-compact whitespace-nowrap text-right text-sm font-medium">
                                        <button
                                            class="link mr-3"
                                            on:click=move |_| on_edit.call(resource_id)
                                        >
                                            "Edit"
                                        </button>
                                        <button
                                            class="link-danger"
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
