use crate::auth::{logout_user, use_auth};
use leptos::*;
use leptos_router::*;

/// Sidebar navigation item
#[component]
fn SidebarNavItem(
    #[prop(into)] href: String,
    #[prop(into)] label: String,
    #[prop(into)] icon_path: String,
) -> impl IntoView {
    let location = use_location();
    let href_clone = href.clone();

    let is_active = Signal::derive(move || {
        let current = location.pathname.get();
        if href_clone == "/dashboard" {
            current == "/dashboard"
        } else {
            current.starts_with(&href_clone) && !href_clone.is_empty()
        }
    });

    view! {
        <a
            href=href
            class="group flex items-center gap-3 px-3 py-1.5 rounded-md text-sm font-medium transition-colors duration-150"
            class:bg-huly-nav-selected=is_active
            class:text-huly-caption=is_active
            class:text-huly-secondary=move || !is_active.get()
            class:hover:bg-huly-nav-hover=move || !is_active.get()
            class:hover:text-huly-content=move || !is_active.get()
        >
            <svg
                class="w-4 h-4 flex-shrink-0 transition-colors duration-150"
                class:text-huly-caption=is_active
                class:text-huly-nav-icon=move || !is_active.get()
                class:group-hover:text-huly-content=move || !is_active.get()
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                stroke-width="1.5"
            >
                <path stroke-linecap="round" stroke-linejoin="round" d=icon_path />
            </svg>
            <span>{label}</span>
        </a>
    }
}

/// Section label in sidebar
#[component]
fn SidebarSection(#[prop(into)] label: String) -> impl IntoView {
    view! {
        <div class="px-3 pt-5 pb-1.5">
            <span class="text-[0.6875rem] font-semibold uppercase tracking-wider text-huly-muted">
                {label}
            </span>
        </div>
    }
}

/// Main application sidebar — Huly-inspired vertical nav
#[component]
pub fn AppSidebar() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();

    let user_role = Signal::derive(move || auth.user.get().map(|u| u.role).unwrap_or_default());

    let user_name = Signal::derive(move || {
        auth.user
            .get()
            .map(|u| format!("{} {}", u.first_name, u.last_name))
            .unwrap_or_else(|| "User".to_string())
    });

    let user_email = Signal::derive(move || auth.user.get().map(|u| u.email).unwrap_or_default());

    let is_hr = Signal::derive(move || user_role.get() == "hr");
    let is_dept_head = Signal::derive(move || user_role.get() == "department_head");
    let is_admin = Signal::derive(move || user_role.get() == "admin");

    let show_team = Signal::derive(move || is_hr.get() || is_dept_head.get() || is_admin.get());

    let show_ctc_status = Signal::derive(move || is_hr.get() || is_dept_head.get());

    let show_hr_tools = Signal::derive(move || is_hr.get());

    let handle_logout = move |_| {
        logout_user(&auth);
        navigate("/", Default::default());
    };

    view! {
        <aside class="flex flex-col w-60 h-screen bg-huly-nav border-r border-huly-divider flex-shrink-0 select-none">
            // ─── Brand ───
            <div class="flex items-center gap-2.5 px-4 h-14 flex-shrink-0 border-b border-huly-divider">
                <div class="w-7 h-7 rounded-md bg-primary-600 flex items-center justify-center">
                    <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M13 10V3L4 14h7v7l9-11h-7z" />
                    </svg>
                </div>
                <span class="text-sm font-semibold text-huly-caption tracking-tight">"Xynergy"</span>
            </div>

            // ─── Navigation ───
            <nav class="flex-1 overflow-y-auto px-2 py-1">
                <SidebarSection label="Navigation" />
                <div class="space-y-0.5">
                    <SidebarNavItem
                        href="/dashboard"
                        label="Dashboard"
                        icon_path="M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6zM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25zM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6zM13.5 15.75a2.25 2.25 0 012.25-2.25H18a2.25 2.25 0 012.25 2.25V18A2.25 2.25 0 0118 20.25h-2.25A2.25 2.25 0 0113.5 18v-2.25z"
                    />
                    <SidebarNavItem
                        href="/resources"
                        label="Resources"
                        icon_path="M15 19.128a9.38 9.38 0 002.625.372 9.337 9.337 0 004.121-.952 4.125 4.125 0 00-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 018.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0111.964-3.07M12 6.375a3.375 3.375 0 11-6.75 0 3.375 3.375 0 016.75 0zm8.25 2.25a2.625 2.625 0 11-5.25 0 2.625 2.625 0 015.25 0z"
                    />
                    <SidebarNavItem
                        href="/projects"
                        label="Projects"
                        icon_path="M9 12h3.75M9 15h3.75M9 18h3.75m3 .75H18a2.25 2.25 0 002.25-2.25V6.108c0-1.135-.845-2.098-1.976-2.192a48.424 48.424 0 00-1.123-.08m-5.801 0c-.065.21-.1.433-.1.664 0 .414.336.75.75.75h4.5a.75.75 0 00.75-.75 2.25 2.25 0 00-.1-.664m-5.8 0A2.251 2.251 0 0113.5 2.25H15a2.25 2.25 0 012.15 1.586m-5.8 0c-.376.023-.75.05-1.124.08C9.095 4.01 8.25 4.973 8.25 6.108V8.25m0 0H4.875c-.621 0-1.125.504-1.125 1.125v11.25c0 .621.504 1.125 1.125 1.125h9.75c.621 0 1.125-.504 1.125-1.125V9.375c0-.621-.504-1.125-1.125-1.125H8.25z"
                    />
                    <SidebarNavItem
                        href="/allocations"
                        label="Allocations"
                        icon_path="M6.75 3v2.25M17.25 3v2.25M3 18.75V7.5a2.25 2.25 0 012.25-2.25h13.5A2.25 2.25 0 0121 7.5v11.25m-18 0A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75m-18 0v-7.5A2.25 2.25 0 015.25 9h13.5A2.25 2.25 0 0121 11.25v7.5"
                    />
                </div>

                // ─── Management Section (role-gated) ───
                <Show when=move || show_team.get()>
                    <SidebarSection label="Management" />
                    <div class="space-y-0.5">
                        <SidebarNavItem
                            href="/team"
                            label="My Team"
                            icon_path="M18 18.72a9.094 9.094 0 003.741-.479 3 3 0 00-4.682-2.72m.94 3.198l.001.031c0 .225-.012.447-.037.666A11.944 11.944 0 0112 21c-2.17 0-4.207-.576-5.963-1.584A6.062 6.062 0 016 18.719m12 0a5.971 5.971 0 00-.941-3.197m0 0A5.995 5.995 0 0012 12.75a5.995 5.995 0 00-5.058 2.772m0 0a3 3 0 00-4.681 2.72 8.986 8.986 0 003.74.477m.94-3.197a5.971 5.971 0 00-.94 3.197M15 6.75a3 3 0 11-6 0 3 3 0 016 0zm6 3a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0zM6.75 9.75a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0z"
                        />
                        <Show when=move || show_ctc_status.get()>
                            <SidebarNavItem
                                href="/ctc/completeness"
                                label="CTC Status"
                                icon_path="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                            />
                        </Show>
                        <Show when=move || show_hr_tools.get()>
                            <SidebarNavItem
                                href="/ctc"
                                label="CTC"
                                icon_path="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z"
                            />
                            <SidebarNavItem
                                href="/thr"
                                label="THR"
                                icon_path="M2.25 18.75a60.07 60.07 0 0115.797 2.101c.727.198 1.453-.342 1.453-1.096V18.75M3.75 4.5v.75A.75.75 0 013 6h-.75m0 0v-.375c0-.621.504-1.125 1.125-1.125H20.25M2.25 6v9m18-10.5v.75c0 .414.336.75.75.75h.75m-1.5-1.5h.375c.621 0 1.125.504 1.125 1.125v9.75c0 .621-.504 1.125-1.125 1.125h-.375m1.5-1.5H21a.75.75 0 00-.75.75v.75m0 0H3.75m0 0h-.375a1.125 1.125 0 01-1.125-1.125V15m1.5 1.5v-.75A.75.75 0 003 15h-.75M15 10.5a3 3 0 11-6 0 3 3 0 016 0zm3 0h.008v.008H18V10.5zm-12 0h.008v.008H6V10.5z"
                            />
                        </Show>
                    </div>
                </Show>

                // ─── Settings ───
                <div class="mt-auto pt-2 border-t border-huly-divider mx-2 mt-4">
                    <SidebarNavItem
                        href="/settings"
                        label="Settings"
                        icon_path="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 011.37.49l1.296 2.247a1.125 1.125 0 01-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 010 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 01-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 01-.22.128c-.331.183-.581.495-.644.869l-.213 1.28c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 01-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 01-1.369-.49l-1.297-2.247a1.125 1.125 0 01.26-1.431l1.004-.827c.292-.24.437-.613.43-.992a6.932 6.932 0 010-.255c.007-.378-.138-.75-.43-.99l-1.004-.828a1.125 1.125 0 01-.26-1.43l1.297-2.247a1.125 1.125 0 011.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281z"
                    />
                </div>
            </nav>

            // ─── User Section ───
            <div class="flex-shrink-0 border-t border-huly-divider p-3">
                <div class="flex items-center gap-3">
                    <div class="w-8 h-8 rounded-full bg-primary-600 flex items-center justify-center text-white text-xs font-semibold flex-shrink-0">
                        {move || {
                            auth.user.get()
                                .map(|u| {
                                    let first = u.first_name.chars().next().unwrap_or('U');
                                    let last = u.last_name.chars().next().unwrap_or(' ');
                                    format!("{}{}", first, last).to_uppercase()
                                })
                                .unwrap_or_else(|| "U".to_string())
                        }}
                    </div>
                    <div class="flex-1 min-w-0">
                        <div class="text-xs font-medium text-huly-content truncate">
                            {move || user_name.get()}
                        </div>
                        <div class="text-[0.6875rem] text-huly-muted truncate">
                            {move || user_email.get()}
                        </div>
                    </div>
                    <button
                        on:click=handle_logout
                        class="p-1.5 rounded-md text-huly-muted hover:text-huly-content hover:bg-huly-btn-hover transition-colors duration-150"
                        title="Logout"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.5">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 9V5.25A2.25 2.25 0 0013.5 3h-6a2.25 2.25 0 00-2.25 2.25v13.5A2.25 2.25 0 007.5 21h6a2.25 2.25 0 002.25-2.25V15m3 0l3-3m0 0l-3-3m3 3H9" />
                        </svg>
                    </button>
                </div>
            </div>
        </aside>
    }
}
