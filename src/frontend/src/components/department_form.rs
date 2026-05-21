use leptos::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

/// Department form data structure
#[derive(Debug, Clone, Default)]
pub struct DepartmentFormData {
    pub name: String,
    pub head_id: String,
}

/// Department edit data structure
#[derive(Debug, Clone)]
pub struct DepartmentEditData {
    pub id: String,
    pub name: String,
    pub head_id: String,
}

/// Head candidate option for dropdown
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HeadCandidate {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

/// Department form component
#[component]
pub fn DepartmentForm(
    #[prop(optional)] head_candidates: Vec<HeadCandidate>,
    editing_department: Option<DepartmentEditData>,
    #[prop(into)] on_submit: Callback<DepartmentFormData>,
    #[prop(into)] on_cancel: Callback<()>,
    #[prop(default = false)] is_submitting: bool,
) -> impl IntoView {
    let is_edit = editing_department.is_some();

    // Form fields
    let (name, set_name) = signal(
        editing_department
            .as_ref()
            .map(|d| d.name.clone())
            .unwrap_or_default(),
    );
    let (head_id, set_head_id) = signal(
        editing_department
            .as_ref()
            .map(|d| d.head_id.clone())
            .unwrap_or_default(),
    );

    // Update form fields when editing_department changes
    Effect::new(move |_| {
        if let Some(dept) = &editing_department {
            set_name.set(dept.name.clone());
            set_head_id.set(dept.head_id.clone());
        } else {
            set_name.set(String::new());
            set_head_id.set(String::new());
        }
    });

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();

        let form_data = DepartmentFormData {
            name: name.get(),
            head_id: head_id.get(),
        };

        on_submit.run(form_data);
    };

    view! {
        <form on:submit=handle_submit class="space-y-3">
            <div class="grid grid-cols-1 gap-4">
                // Department Name
                <div>
                    <label for="name" class="label">
                        "Department Name *"
                    </label>
                    <input
                        type="text"
                        id="name"
                        class="input"
                        placeholder="Engineering"
                        prop:value=name
                        on:input=move |ev| set_name.set(event_target_value(&ev))
                        required
                    />
                </div>

                // Department Head
                <div>
                    <label for="head" class="label">
                        "Department Head"
                    </label>
                    <select
                        id="head"
                        class="input"
                        prop:value=head_id
                        on:change=move |ev| set_head_id.set(event_target_value(&ev))
                    >
                        <option value="">"-- Select Department Head --"</option>
                        {head_candidates.iter().map(|candidate| {
                            view! {
                                <option value={candidate.id.to_string()}>
                                    {format!("{} ({})", candidate.name, candidate.email)}
                                </option>
                            }
                        }).collect_view()}
                    </select>
                    <p class="mt-1 text-xs text-huly-muted">
                        "Only admin and project manager roles can be department heads"
                    </p>
                </div>
            </div>

            // Form buttons
            <div class="flex justify-end gap-2 pt-3">
                <button
                    type="button"
                    class="btn-secondary btn-press"
                    on:click=move |_| on_cancel.run(())
                    disabled=is_submitting
                >
                    "Cancel"
                </button>
                <button
                    type="submit"
                    class="btn-primary btn-press"
                    disabled=is_submitting
                >
                    {if is_submitting {
                        if is_edit { "Saving..." } else { "Creating..." }
                    } else {
                        if is_edit { "Save Changes" } else { "Create Department" }
                    }}
                </button>
            </div>
        </form>
    }
}
