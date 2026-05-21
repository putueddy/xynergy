# Story 6.1: Role-Based Dashboard

Status: done

<!-- Ultimate context engine analysis completed - comprehensive developer guide created. -->

## Story

As a **System User**,
I want **to see a personalized dashboard based on my role**,
so that **I immediately see information relevant to my responsibilities**.

## Acceptance Criteria

1. **Given** I am an HR Staff member **when** I log in **then** I see: CTC Completeness Status, Recent CTC Changes, Pending Updates, Compliance Alerts.
2. **Given** I am a Department Head **when** I log in **then** I see: Team Utilization Rates, Budget Status, Overallocations, Upcoming Assignments.
3. **Given** I am a Project Manager **when** I log in **then** I see: Project Health Cards (Budget, P&L, Margin), Active Projects, Margin Alerts.
4. **Given** I am a Finance Team member **when** I log in **then** I see: Cash Position, CTC Validation Status, Audit Alerts, Export Requests Pending.

## Scope Boundary

- **In scope**: a role-aware `/dashboard` first screen after login, backend-authoritative dashboard summary contract, per-role dashboard cards/lists for HR, Department Head, Project Manager, and Finance, access-safe role scoping, loading/error/empty states, manual refresh, basic 30-second polling foundation if it can be added cleanly, and backend integration tests.
- **In scope for Admin**: admin users must not regress. They may receive an admin/combined operational dashboard or a clear admin dashboard summary, but they must not be redirected into a broken or forbidden dashboard state.
- **Not in scope**: full Story 6.2 changed-value highlight behavior, full Story 6.3 project health sorting/detail UX, full Story 6.4 utilization trend dashboard, full Story 6.5 CTC completeness trend by month, new charting libraries, SSE/WebSocket updates, executive BI, export approval inbox, or new domain data-entry workflows.
- **Dependencies from prior stories**: reuse CTC completeness/reporting, team capacity/budget, P&L, cash-flow, CTC validation, and compliance audit foundations. Do not duplicate those services or invent parallel tables. [Source: `_bmad-output/implementation-artifacts/3-5-department-budget-utilization.md`; `_bmad-output/implementation-artifacts/4-5-pl-dashboard.md`; `_bmad-output/implementation-artifacts/5-2-cash-flow-dashboard.md`; `_bmad-output/implementation-artifacts/5-3-ctc-validation-reports.md`; `_bmad-output/implementation-artifacts/5-4-compliance-audit-reports.md`]

## Tasks / Subtasks

- [x] **Task 1: Define a backend role-dashboard contract** (AC: #1, #2, #3, #4)
  - [x] Add `src/backend/src/routes/dashboard.rs` with `dashboard_routes() -> Router<PgPool>` and route `GET /api/v1/dashboard`.
  - [x] Export `dashboard_routes` from `src/backend/src/routes/mod.rs` and merge it in `api_routes()` in `src/backend/src/lib.rs`.
  - [x] Add `src/backend/src/services/dashboard_service.rs` if route logic would otherwise grow beyond auth/query orchestration; export it from `src/backend/src/services/mod.rs`.
  - [x] Extract JWT claims with `user_claims_from_headers(&headers)?`; return `401` for missing/invalid token.
  - [x] Shape the response so the frontend can render without guessing permissions. Recommended contract:
    ```rust
    #[derive(Debug, Serialize)]
    pub struct RoleDashboardResponse {
        pub role: String,
        pub generated_at: chrono::DateTime<chrono::Utc>,
        pub hr: Option<HrDashboard>,
        pub department_head: Option<DepartmentHeadDashboard>,
        pub project_manager: Option<ProjectManagerDashboard>,
        pub finance: Option<FinanceDashboard>,
        pub admin: Option<AdminDashboard>,
    }
    ```
  - [x] For a non-admin role, populate only that role's section and set other sections to `None`.
  - [x] For admin, either populate `admin` with operational totals or a bounded combined view. Document the chosen behavior in tests and keep it access-safe.
  - [x] Keep all response fields explicit and typed. Avoid `serde_json::Value` for primary dashboard cards unless it wraps a clearly documented metadata field.
  - [x] Include `generated_at` so the UI can show last refresh time without client-side clock inference.

- [x] **Task 2: Build HR dashboard data from existing CTC services** (AC: #1)
  - [x] CTC Completeness Status: reuse `get_completeness_summary(&pool, department_id)` from `ctc_completeness.rs`.
  - [x] Pending Updates: reuse `get_missing_employees(&pool, department_id)` for HR; return count plus a short bounded list (for example first 5 employees) so the dashboard does not become a full Story 6.5 page.
  - [x] Recent CTC Changes: query `ctc_revisions` joined to `resources` and `users` for recent revisions ordered newest first, limited to 5-10 rows.
  - [x] Recent CTC Changes must expose only safe summary fields: employee name/id, changed-by display/id, revision number, created date, and reason. Do not expose component values, daily rates, encrypted payloads, key versions, algorithms, or ciphertext.
  - [x] Compliance Alerts: reuse `validate_bpjs_compliance(&pool, start_date, end_date)` or a lightweight service wrapper. Use a current-month or last-30-days default and include total discrepancies / compliance rate / top risk rows only.
  - [x] Follow existing HR/Finance access semantics: HR can view CTC completeness and compliance report, but only HR sees pending missing-CTC employee list from `/ctc/completeness/missing`. [Source: `src/backend/src/routes/ctc.rs`]

- [x] **Task 3: Build Department Head dashboard data with department scoping** (AC: #2)
  - [x] Reuse the same scoping semantics as `team.rs`: Department Head is scoped to their own department via session/RLS; HR/Admin optional overrides are not part of this Department Head dashboard path.
  - [x] Avoid duplicating private route helpers inconsistently. If needed, extract shared department-scope helpers from `src/backend/src/routes/team.rs` into a service module, then update both `team.rs` and dashboard code to use the shared helper.
  - [x] Team Utilization Rates: reuse `get_capacity_report_in_transaction(...)` for the current month or next 30 days. Summarize average utilization, overallocated count, and top at-risk members.
  - [x] Budget Status: reuse `compute_department_budget_utilization(...)` for current `YYYY-MM`; preserve budget health rules (`healthy`, `warning`, `critical`) from Story 3.5.
  - [x] Overallocations: reuse `TeamMemberResponse.is_overallocated` / `current_allocation_percentage` from `team_service.rs`.
  - [x] Upcoming Assignments: query existing `allocations` joined to `projects` and `resources`, scoped to department resources, ordered by nearest `start_date`, limited to 5-10 rows.
  - [x] Do not bypass RLS or relationship-based access checks to make the dashboard easier.

- [x] **Task 4: Build Project Manager dashboard data from existing project/P&L services** (AC: #3)
  - [x] Active Projects: query active projects where `projects.project_manager_id = user_id`, ordered by end date or margin risk, limited to a bounded list.
  - [x] Project Health Cards: include project name/id, budget status, total budget/spent/remaining where available, P&L summary, current margin, forecast/alert fields if already available.
  - [x] Reuse `get_project_pl_dashboard(pool, project_id, current_year)` from `project_pl_service.rs` for P&L/margin. Do not recalculate P&L formulas in the dashboard route.
  - [x] Reuse project budget/resource-cost services or existing route logic where available; if private helper extraction is needed, keep it small and shared.
  - [x] Margin Alerts: surface `ProjectPlDashboardResult.margin_alert` exactly rather than inventing a second threshold rule.
  - [x] Enforce relationship-based access with `projects.project_manager_id`; Project Managers must not see non-owned project health cards. Admin handling must be explicit if admin receives combined data. [Source: `src/backend/src/services/rbac.rs`]
  - [x] Avoid N+1 growth: cap project count, and use `tokio::try_join!` or batched queries where practical.

- [x] **Task 5: Build Finance dashboard data from existing finance/audit services** (AC: #4)
  - [x] Cash Position: reuse `cash_flow_service::get_cash_flow_dashboard(...)` for current calendar year or current month; expose total cash in, total cash out, net cash flow, and ending cumulative position.
  - [x] CTC Validation Status: reuse `generate_validation_report(...)` for a default recent/current-month range when payroll staging data exists. If no comparable payroll data exists, return a clear `no_data` status instead of surfacing a hard dashboard failure.
  - [x] Audit Alerts: summarize recent `ACCESS_DENIED`, failed login, blocked login, hash-chain verification failures if available, and recent report/export audit events from `audit_logs`.
  - [x] Export Requests Pending: query `audit_export_requests` where `status = 'pending_approval'`, include count and latest request metadata only.
  - [x] Use the same Finance/Admin role gate as `cash_flow.rs`, `ctc_validation.rs`, and `audit_log.rs`. Non-finance roles must not receive finance dashboard fields.
  - [x] Do not return export bytes or sensitive report contents from the dashboard.

- [x] **Task 6: Replace the current frontend dashboard with role-specific rendering** (AC: #1, #2, #3, #4)
  - [x] Refactor `src/frontend/src/pages/dashboard.rs` to fetch only `/api/v1/dashboard` for dashboard content.
  - [x] Remove the current generic endpoint fan-out (`/resources`, `/allocations`, `/projects`, `/audit-logs`) from the dashboard page; that pattern causes avoidable forbidden/error states for some roles.
  - [x] Keep existing unauthenticated redirect to `/login` and sign-out behavior.
  - [x] Render HR, Department Head, Project Manager, Finance, and Admin dashboard sections from the typed response. A role should see only relevant panels.
  - [x] Use the app's established Huly/Tailwind `page-container`, `page-header`, `panel`, `stat-card`, `alert-*`, table/list, and empty-state patterns. Do not introduce a separate design system.
  - [x] Add manual refresh and display `generated_at` as "Last updated" or equivalent.
  - [x] If adding 30-second polling now, use a cleanup-safe interval pattern; do not call `Interval::forget()` from inside a component unless intentionally leaking the timer is acceptable. Prefer keeping the `Interval` handle in component state or using an effect cleanup pattern. (Deferred to Story 6.2 — only manual refresh shipped in 6.1.)
  - [x] Keep dashboard cards compact, scannable, and task-oriented. This is an operational app, not a marketing page.
  - [x] Add links/actions from dashboard cards to existing full pages where available:
    - HR CTC completeness -> `/ctc/completeness`
    - Department team/budget -> `/team`
    - Project health/P&L -> `/projects`
    - Finance cash/validation/audit -> `/finance/cash-flow`, `/finance/ctc-validation`, `/finance/audit-reports`

- [x] **Task 7: Make login land on the role-based dashboard** (AC: #1, #2, #3, #4)
  - [x] Update `role_dashboard_path()` in `src/frontend/src/pages/login.rs` so successful login and already-authenticated redirects go to `/dashboard` for the supported roles once this dashboard is implemented.
  - [x] Preserve the generic fallback to `/dashboard`.
  - [x] Confirm existing sidebar Dashboard navigation remains visible for every authenticated role. [Source: `src/frontend/src/components/app_sidebar.rs`]

- [x] **Task 8: Add integration and regression coverage** (AC: #1, #2, #3, #4)
  - [x] Create `src/backend/tests/dashboard_tests.rs`.
  - [x] Required backend tests:
    - [x] unauthenticated dashboard request returns `401`.
    - [x] HR gets `hr` section only, including completeness status, missing-CTC count/list, recent CTC change summaries, and compliance alert summary.
    - [x] HR recent changes do not include encrypted values, key metadata, or salary component payloads.
    - [x] Department Head gets only own department data; another department's resources/allocations/budget are excluded.
    - [x] Department Head response includes utilization summary, budget health, overallocated count, and upcoming assignments.
    - [x] Project Manager gets only owned active projects; non-owned projects are excluded.
    - [x] Project Manager response surfaces P&L margin alert from existing P&L service when applicable.
    - [x] Finance gets cash position, CTC validation status/no-data state, audit alert summary, and pending export request count.
    - [x] Finance response does not include HR pending employee lists or Department Head/PM scoped data.
    - [x] Admin dashboard access returns `200` and documented admin/combined fields.
  - [x] Re-run high-risk backend suites after dashboard tests:
    - [x] `cargo test --package xynergy-backend --test dashboard_tests`
    - [x] `cargo test --package xynergy-backend --test team_tests`
    - [x] `cargo test --package xynergy-backend --test project_pl_tests`
    - [x] `cargo test --package xynergy-backend --test cash_flow_dashboard_tests`
    - [x] `cargo test --package xynergy-backend --test ctc_validation_report_tests`
    - [x] `cargo test --package xynergy-backend --test compliance_audit_report_tests`
  - [x] Frontend compile/regression:
    - [x] `cargo check -p xynergy-frontend --target wasm32-unknown-unknown`
    - [x] `npm --prefix src/frontend run build`

### Review Findings

- [x] [Review][Patch] Dashboard bypasses RLS/session-backed scoping and trusts JWT department claims [src/backend/src/routes/dashboard.rs:22]
- [x] [Review][Patch] Project Manager health cards omit required budget status and budget amounts [src/backend/src/services/dashboard_service.rs:157]
- [x] [Review][Patch] Project Manager active-project query includes planning or ended projects [src/backend/src/services/dashboard_service.rs:663]
- [x] [Review][Patch] Finance cash position counts future-dated entries through year-end while labeled current YTD [src/backend/src/services/dashboard_service.rs:747]
- [x] [Review][Patch] Optional dashboard card failures can still blank the entire role dashboard [src/backend/src/services/dashboard_service.rs:355]
- [x] [Review][Patch] Finance validation status exposes raw internal error strings in dashboard JSON [src/backend/src/services/dashboard_service.rs:827]
- [x] [Review][Patch] Department Head upcoming assignments can be crowded out by current or past-started allocations [src/backend/src/services/dashboard_service.rs:596]

## Dev Notes

### Developer Context

- Epic 6 is the Dashboard & Reporting epic. Story 6.1 is the first story and establishes the personalized landing surface; Stories 6.2-6.5 expand refresh behavior and deeper dashboards after this foundation. [Source: `_bmad-output/planning-artifacts/epics.md#Epic-6-Dashboard--Reporting`]
- Story 6.1's exact role expectations come from the epics file: HR sees CTC completeness/recent changes/pending updates/compliance alerts; Department Head sees utilization/budget/overallocations/upcoming assignments; Project Manager sees project health/active projects/margin alerts; Finance sees cash/validation/audit/export-request status. [Source: `_bmad-output/planning-artifacts/epics.md#Story-6.1-Role-Based-Dashboard`]
- PRD FR45-FR51 define personalized dashboards, CTC completeness, team utilization, project health, finance validation reports, polling updates, and manual refresh. Story 6.1 should implement FR45 and lay the contract for FR50/FR51 without swallowing the deeper stories. [Source: `_bmad-output/planning-artifacts/prd.md#7-Dashboard--Reporting`]
- The UX specification positions "Back to my dashboard" as a habit-forming returning-user state and emphasizes desktop SPA dashboards with dense data tables. Keep this dashboard practical and scan-friendly. [Source: `_bmad-output/planning-artifacts/ux-design-specification.md#Core-User-Experience`]
- The current `src/frontend/src/pages/dashboard.rs` is a generic welcome/count page. It fetches resources, allocations, projects, and recent audit logs directly. That is not role-based and can produce errors because `/api/v1/audit-logs` is finance/admin-only. Replace this fan-out with the new dashboard endpoint. [Source: `src/frontend/src/pages/dashboard.rs`; `src/backend/src/routes/audit_log.rs`]
- `src/frontend/src/pages/login.rs` currently redirects roles away from `/dashboard` (`hr` -> `/resources`, `department_head` -> `/allocations`, `project_manager` -> `/projects`, `admin` -> `/settings/users`). This conflicts with Story 6.1's login AC and must be updated as part of this story. [Source: `src/frontend/src/pages/login.rs`]
- `src/frontend/src/components/app_sidebar.rs` already shows Dashboard to all authenticated users and gates feature sections by role. Preserve that behavior.
- Admin exists throughout the codebase even though the Story 6.1 ACs name only HR, Department Head, Project Manager, and Finance. Handle admin deliberately so `/dashboard` is never a dead end for administrators. [Source: `src/backend/src/routes/user.rs`; `src/frontend/src/components/app_sidebar.rs`]

### Current File State To Preserve

- `src/frontend/src/auth.rs`: `authenticated_get()` handles absolute URL resolution in WASM, Authorization headers, token refresh, and forced relogin. Use this helper for the dashboard fetch, not raw `reqwest`.
- `src/frontend/src/pages/dashboard.rs`: preserve unauthenticated redirect and sign-out flow, but replace generic counters and endpoint fan-out.
- `src/frontend/src/pages/login.rs`: preserve auth signal updates and generic invalid-credential handling; only change destination routing.
- `src/backend/src/lib.rs`: API routes are merged under `/api/v1`; add dashboard route composition here.
- `src/backend/src/routes/mod.rs`: one route module export per route file is the local pattern.
- `src/backend/src/routes/team.rs`: contains department scoping and RLS transaction flow. Do not fork a subtly different scoping model in dashboard code.
- `src/backend/src/services/team_service.rs`: already decrypts daily rates and produces `TeamMemberResponse` / `CapacityReportResponse`; use it for Department Head dashboard summaries.
- `src/backend/src/services/budget_service.rs`: canonical department budget summary/breakdown and health logic; use this for budget status.
- `src/backend/src/services/project_pl_service.rs`: canonical project P&L and margin alert logic; use this for PM dashboard margin cards.
- `src/backend/src/services/cash_flow_service.rs`: canonical cash flow dashboard math; use this for Finance cash position.
- `src/backend/src/services/compliance_audit_report.rs` and `src/backend/src/routes/audit_log.rs`: canonical compliance audit report/export foundations; use these for audit alert and pending export summary only where appropriate.

### Technical Requirements

- Backend must be the authority for role scope. The frontend should render what `/api/v1/dashboard` returns; it should not fetch forbidden endpoints and hide failures by role string.
- Use relationship-based access controls where applicable:
  - Department Head: department via `departments.head_id` / current user department and existing RLS/session patterns.
  - Project Manager: owned projects via `projects.project_manager_id`.
  - Finance: finance/admin gate for finance summaries.
  - HR: HR-only CTC missing employee list and HR/Finance compliance report semantics.
- Every dashboard list must be bounded. Use small limits (`5`, `10`, or a documented constant) for recent changes, upcoming assignments, active project cards, and audit alerts.
- Use `chrono::Utc::now()` / `chrono::Local` consistently only where the repo already does. Backend `generated_at` should be UTC.
- Use `chrono::NaiveDate` for date windows and `YYYY-MM` strings for budget periods, matching existing services.
- Keep money values as integer IDR (`i64`) end to end. Do not introduce `f64` for currency.
- Do not expose encrypted CTC data, key metadata, salary component details, or raw audit payload JSON in dashboard cards.
- Access-denied attempts on sensitive dashboard data should be audited where existing sensitive routes already do this; avoid adding noisy audit rows for ordinary successful dashboard reads unless the domain route already audits that read.
- Empty/no-data states are valid dashboard states. Example: finance CTC validation may have no payroll staging data; return `no_data`/zero summary rather than failing the whole dashboard.
- A failure in one optional card should not blank an otherwise valid role dashboard unless that failure is security-critical. Prefer a per-section warning field for no-data/partial data when safe.

### Architecture Compliance

- Keep Axum handlers thin: route extracts headers/query/state, service assembles role data.
- Mount new route under `/api/v1/dashboard`.
- Keep DTO names explicit, `#[derive(Debug, Serialize)]` on responses and `#[derive(Debug, Deserialize)]` on query structs.
- Use parameterized SQL with `.bind()` or `sqlx::QueryBuilder` + `push_bind()`. Never concatenate user-controlled filter values into SQL.
- If private helpers must become shared, extract them intentionally rather than copying logic.
- Preserve existing route behavior for `/team`, `/ctc/*`, `/projects/*`, `/cash-flow/*`, and `/audit-logs/*`; Story 6.1 should compose them, not rewrite them.

### Library / Framework Requirements

- Local manifests are the source of truth for versions: Rust 2021, Axum 0.7, sqlx 0.7, Leptos 0.8, PostgreSQL 15+, Tailwind CSS 4.1.x. The older project-context note that says Leptos 0.6 is stale for this repo; use `src/frontend/Cargo.toml` as the override. [Source: `Cargo.toml`; `src/backend/Cargo.toml`; `src/frontend/Cargo.toml`; `src/frontend/package.json`]
- Official Leptos docs describe `Resource`/`LocalResource` as reactive wrappers for async tasks and support `.refetch()` for manual reloads. This fits dashboard refresh. [Source: https://book.leptos.dev/async/10_resources.html]
- Official Axum docs for `Query<T>` parse query strings into `serde::Deserialize` types and reject parse failures as `400`; use typed query DTOs where filters are added. [Source: https://docs.rs/axum/latest/axum/extract/struct.Query.html]
- Official `gloo_timers::callback::Interval` source documents RAII cancellation on drop and warns that `forget()` leaks the interval. If polling is implemented in Story 6.1, keep cleanup explicit. [Source: https://docs.rs/gloo-timers/latest/src/gloo_timers/callback.rs.html]

### File Structure Requirements

- Backend likely touch points:
  - `src/backend/src/routes/dashboard.rs` (new)
  - `src/backend/src/routes/mod.rs`
  - `src/backend/src/lib.rs`
  - `src/backend/src/services/dashboard_service.rs` (new, recommended)
  - `src/backend/src/services/mod.rs`
  - `src/backend/src/routes/team.rs` only if shared scoping helpers are extracted/reused
  - `src/backend/tests/dashboard_tests.rs` (new)
- Frontend likely touch points:
  - `src/frontend/src/pages/dashboard.rs`
  - `src/frontend/src/pages/login.rs`
  - `src/frontend/public/output.css` if Tailwind output must be regenerated for new classes
- Existing files to treat mainly as references:
  - `src/frontend/src/pages/ctc_completeness.rs`
  - `src/frontend/src/pages/team.rs`
  - `src/frontend/src/pages/projects.rs`
  - `src/frontend/src/pages/cash_flow.rs`
  - `src/frontend/src/pages/ctc_validation.rs`
  - `src/frontend/src/pages/audit_reports.rs`

### Testing Requirements

- Follow existing backend integration style with `#[sqlx::test(migrations = "../../migrations")]`.
- Set required test env vars in dashboard tests: `JWT_SECRET`, `CTC_ACTIVE_KEY_VERSION`, and `CTC_ENCRYPTION_KEY_V1` when CTC encryption/decryption is involved.
- Use login endpoint to obtain tokens in tests, matching existing suites.
- Test both positive role summaries and negative data isolation. Data isolation is more important than card counts.
- Add explicit assertions that sensitive fields are absent from dashboard JSON, especially HR recent CTC changes and Finance audit/export summaries.
- Frontend automated E2E is not established. At minimum, compile the WASM frontend and manually inspect role rendering if a browser check is available during implementation.

### Previous Story Intelligence

- Story 5.4 added substantial audit-report/export behavior and patched many access, pagination, snapshot, and sensitive-payload issues. The dashboard should summarize audit/export state, not reopen raw report/export bytes or duplicate report generation semantics.
- Story 5.3 established finance validation report patterns, including sanitized errors and no-data handling. Finance dashboard should reuse that posture rather than showing raw API/proxy bodies.
- Story 5.2 added cash-flow dashboard polling/manual refresh patterns. Reuse its approach carefully, but avoid leaking intervals.
- Story 4.5 and 4.6 established native SVG/table parity for deeper dashboard charts. Story 6.1 does not need new charts; if simple visuals are added, keep table/text parity.
- Story 3.5 review history emphasized department scoping consistency via `resolve_department_id()` and weighted capacity formulas. Do not bypass those helpers for dashboard summaries.

### Git Intelligence Summary

- Recent commits are vertical slices: route/service/frontend/tests/story artifact together. Story 6.1 should follow the same slice shape. [Source: `git log --oneline -5`]
- Most recent commit `d89082b` touched `src/frontend/src/pages/dashboard.rs` as part of audit report hardening, so read current dashboard behavior before editing and avoid reintroducing raw audit-log assumptions. [Source: `git show --stat --oneline d89082b`]
- Current working tree was clean before story creation. Avoid unrelated refactors.

### Implementation Pitfalls To Avoid

- Do not keep the old generic dashboard endpoint fan-out and merely hide cards conditionally. That leaves role-specific forbidden/error behavior in place.
- Do not make the frontend decide access by role string and then call domain endpoints directly for each card. Use the backend dashboard contract.
- Do not add a second copy of budget/P&L/cash-flow math.
- Do not expose encrypted or salary component data in recent CTC changes.
- Do not let Admin fall through to a blank dashboard.
- Do not use `Interval::forget()` casually; leaked dashboard polling will continue after route changes.
- Do not mark checklist items complete until code and tests actually prove them.

## Dev Agent Record

### Agent Model Used

Claude Opus 4.7 (claude-opus-4-7)

### Debug Log References

- Initial backend `cargo check`: clean (warning about sqlx-postgres future-incompat is pre-existing in the workspace).
- Frontend WASM `cargo check` first pass failed with five `borrow of moved value` errors after `Option::map` consumed `data.*` fields before the catch-all empty-state check; resolved by pre-computing the `no_widgets` boolean before the `view!` block.
- `cargo test --package xynergy-backend --test dashboard_tests`: 10/10 passed.
- Regression suites re-run after dashboard tests:
  - `team_tests`: 10/10 passed.
  - `project_pl_tests`: 10/10 passed.
  - `cash_flow_dashboard_tests`: 28/28 passed.
  - `ctc_validation_report_tests`: 36/36 passed.
  - `compliance_audit_report_tests`: 48/48 passed.
- `npm --prefix src/frontend run build`: regenerated `public/output.css` in 44ms (Tailwind v4.1.18).
- Code review patch pass:
  - `cargo test --package xynergy-backend --test dashboard_tests`: 27/27 passed.
  - `cargo test --package xynergy-backend --test team_tests --test project_pl_tests --test cash_flow_dashboard_tests --test ctc_validation_report_tests --test compliance_audit_report_tests`: all selected suites passed.
  - `cargo check -p xynergy-frontend --target wasm32-unknown-unknown`: passed with existing dead-code warnings.
  - `npm --prefix src/frontend run build`: passed (Tailwind v4.1.18).

### Completion Notes List

- Added a new backend service `services::dashboard_service` that composes existing CTC completeness, team capacity, budget, project P&L, cash-flow, CTC validation, audit, and export services into bounded per-role summaries. No domain formulas were duplicated.
- The new route `GET /api/v1/dashboard` is thin: it extracts JWT claims, then delegates to `build_dashboard(&pool, &headers, &claims)`. Unauthenticated requests return `401`; unsupported roles (e.g. `team_member`) return `403`.
- Response contract follows the recommended `RoleDashboardResponse` shape. Only the requester's role section is populated; others serialize as absent (`#[serde(skip_serializing_if = "Option::is_none")]`).
- Department head scoping now uses an RLS-initialized transaction and the current database `users.department_id`, not the JWT department claim; the dashboard reuses `get_team_members_in_transaction` / `get_capacity_report_in_transaction` / `compute_department_budget_utilization`.
- Project Manager dashboard is gated by `projects.project_manager_id = user_id` directly in SQL; non-owned, planning, completed, cancelled, closed, and already-ended projects cannot appear in `active_projects` or `margin_alerts`. Margin alerts are surfaced verbatim from `ProjectPlDashboardResult.margin_alert`.
- Project Manager project health cards now include budget total/spent/remaining/status alongside P&L and margin fields.
- Finance dashboard computes current YTD cash position through today, so future-dated entries are excluded from the current-position card. CTC validation returns `no_data` for missing payroll staging and a sanitized `error` message for internal failures.
- Optional dashboard cards now degrade to bounded empty/default summaries with role-section warnings where safe instead of blanking the entire dashboard.
- HR recent changes only expose `resource_id`, `resource_name`, `revision_number`, `changed_by` id/name, `created_at`, and `reason`. Encrypted payloads, ciphertext, key metadata, and salary components are never read into the response (verified by `hr_recent_changes_do_not_expose_encrypted_fields`).
- Admin dashboard is intentionally an operational totals view (users, departments, active projects, active CTC records, pending exports, access-denied count in last 24h) so admin never lands on a blank `/dashboard`.
- Frontend `pages/dashboard.rs` now fetches only `/api/v1/dashboard` and renders typed role panels using the existing Huly/Tailwind components (`page-container`, `panel`, `stat-card`, `activity-item`, `empty-state`, `alert-error`). Manual refresh is wired via a `signal`-driven `Effect`; the page displays `generated_at` as "Last updated" without inferring time on the client. 30-second polling is intentionally deferred to Story 6.2.
- `pages/login.rs::role_dashboard_path()` now routes every supported role (`admin`, `hr`, `department_head`, `project_manager`, `finance`) plus the generic fallback to `/dashboard`, so the new role-based screen is the landing surface after login.

### File List

- `src/backend/src/lib.rs` (modified)
- `src/backend/src/routes/dashboard.rs` (new)
- `src/backend/src/routes/mod.rs` (modified)
- `src/backend/src/services/compliance_report.rs` (modified)
- `src/backend/src/services/ctc_completeness.rs` (modified)
- `src/backend/src/services/dashboard_service.rs` (new)
- `src/backend/src/services/mod.rs` (modified)
- `src/backend/tests/dashboard_tests.rs` (new)
- `src/frontend/src/pages/dashboard.rs` (rewritten)
- `src/frontend/src/pages/login.rs` (modified)
- `src/frontend/public/output.css` (regenerated by Tailwind build)
- `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md` (story status / task checkboxes / Dev Agent Record updates)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (status transition + history comment)

## Change Log

| Date       | Author | Change                                                                                  |
|------------|--------|-----------------------------------------------------------------------------------------|
| 2026-05-21 | Putu   | Implemented Story 6.1 role-based dashboard backend contract, frontend, and regression. |
