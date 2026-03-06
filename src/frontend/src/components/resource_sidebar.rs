use leptos::*;

/// Resource allocation info for sidebar
#[derive(Debug, Clone)]
pub struct ResourceAllocationInfo {
    pub resource_id: String,
    pub resource_name: String,
    pub total_allocations: usize,
    pub current_allocation_percentage: f64,
}

/// Resource sidebar component
#[component]
pub fn ResourceSidebar(
    resources: Signal<Vec<ResourceAllocationInfo>>,
    selected_resource: Signal<Option<String>>,
    on_select: Callback<String>,
) -> impl IntoView {
    view! {
        <div class="h-full flex flex-col bg-huly-surface border-r border-huly-divider">
            <div class="p-3 border-b border-huly-divider">
                <h3 class="text-sm font-semibold text-huly-caption">
                    "Resources"
                </h3>
                <p class="text-xs text-huly-muted">
                    {move || format!("{} resources", resources.get().len())}
                </p>
            </div>

            <div class="flex-1 overflow-y-auto">
                <div class="divide-y divide-huly-divider">
                    {move || {
                        resources.get().into_iter().map(|resource| {
                            let is_selected = selected_resource.get()
                                .map(|id| id == resource.resource_id)
                                .unwrap_or(false);

                            let resource_id = resource.resource_id.clone();
                            let allocation_color = if resource.current_allocation_percentage >= 100.0 {
                                "bg-red-500"
                            } else if resource.current_allocation_percentage >= 80.0 {
                                "bg-yellow-500"
                            } else {
                                "bg-green-500"
                            };

                            view! {
                                <div
                                    class={format!(
                                        "p-3 cursor-pointer table-row-hover transition-colors {}",
                                        if is_selected { "bg-primary-600/10 border-l-4 border-primary-600" } else { "border-l-4 border-transparent" }
                                    )}
                                    on:click=move |_| on_select.call(resource_id.clone())
                                >
                                    <div class="flex items-center justify-between">
                                        <div class="flex-1 min-w-0">
                                            <p class="text-sm font-medium text-huly-caption truncate">
                                                {resource.resource_name.clone()}
                                            </p>
                                            <p class="text-xs text-huly-muted mt-1">
                                                {format!("{} allocations", resource.total_allocations)}
                                            </p>
                                        </div>

                                        <div class="ml-3 flex items-center">
                                            <div class="w-16 bg-huly-surface-2 rounded-full h-2">
                                                <div
                                                    class={format!("{} h-2 rounded-full", allocation_color)}
                                                    style={format!("width: {}%", resource.current_allocation_percentage.min(100.0))}
                                                ></div>
                                            </div>
                                            <span class="ml-2 text-xs font-medium text-huly-secondary w-10 text-right">
                                                {format!("{:.0}%", resource.current_allocation_percentage)}
                                            </span>
                                        </div>
                                    </div>
                                </div>
                            }
                        }).collect_view()
                    }}
                </div>
            </div>
        </div>
    }
}
