# Story 6.4: Team Utilization Dashboard

Status: done

<!-- Ultimate context engine analysis completed - comprehensive developer guide created. -->

## Story

As a **Department Head**,
I want **to view team utilization rates and budget status**,
so that **I can optimize resource allocation**.

## Acceptance Criteria

1. **Given** I navigate to Team Dashboard **when** the page loads **then** I see: Team members, Current Utilization %, Current Projects, Available Capacity.
2. **Given** I view the utilization chart **when** I select a time range **then** I see utilization trends over time for each team member.
3. **Given** I identify underutilized resources **when** I see current utilization < 50% **then** I can click to assign them to new projects.
4. **Given** I view budget status **when** I look at department summary **then** I see: Total Budget, Committed, Spent, Available with visual gauge.

## Scope Boundary

- **In scope**: deepen the Department Head section of the existing role-based `/dashboard`; include a team-member utilization table, current project summary, available capacity, underutilized members below 50% utilization, selected time-range utilization trends, and a full department budget summary with gauge.
- **In scope for assignment action**: provide a dashboard action that reuses the existing `/team` assignment workflow, preferably through a narrow deep-link such as `/team?assign_resource_id=<uuid>`. The existing assignment modal, assignable-project fetch, cost preview, CTC guard, overallocation warning, and budget-impact preview must remain the source of truth.
- **In scope for dashboard query shape**: if the utilization trend range needs backend input, extend `GET /api/v1/dashboard` with typed optional query parameters such as `team_start_date` and `team_end_date`. Keep one dashboard fetch path for initial load, manual refresh, and 30-second polling.
- **Not in scope**: new charting libraries, SSE/WebSockets, a parallel assignment modal inside `dashboard.rs`, new allocation formulas, new budget formulas, Story 6.5 CTC completeness dashboard, PM project-health changes, Finance dashboard changes, database schema changes, or replacing the full `/team` page.
- **Hard dependencies**: reuse `GET /api/v1/dashboard`, `get_team_members_in_transaction(...)`, `get_capacity_report_in_transaction(...)`, `compute_department_budget_utilization(...)`, the existing `/team` assignment modal flow, and Story 6.2 polling/change-highlight behavior. Do not reintroduce dashboard fan-out to several domain endpoints for ordinary role cards.

## Tasks / Subtasks

- [x] **Task 1: Extend the Department Head dashboard backend contract** (AC: #1, #2, #3, #4)
  - [x] Update `src/backend/src/routes/dashboard.rs` only as needed to accept typed optional trend-range query params, e.g. `DashboardQuery { team_start_date: Option<NaiveDate>, team_end_date: Option<NaiveDate> }`.
  - [x] Validate date params: reject `start > end`, cap excessive ranges to a documented maximum (recommended 6 or 12 months), and default to the existing operational window when no query params are provided.
  - [x] Update `src/backend/src/services/dashboard_service.rs::DepartmentHeadDashboard` with explicit typed fields for the deeper dashboard. Recommended shape:
    - `team_members: Vec<TeamUtilizationMember>`
    - `utilization_trends: Vec<TeamUtilizationTrend>`
    - `underutilized_members: Vec<TeamUtilizationMember>` or stable ids derived from `team_members`
    - keep existing `utilization`, `budget`, `overallocations`, `upcoming_assignments`, and `warnings`
  - [x] `TeamUtilizationMember` should expose safe operational fields only: `resource_id`, `resource_name`, `role`, `current_utilization_pct`, `available_capacity_pct`, `is_underutilized`, `is_overallocated`, `ctc_status`, and a bounded `current_projects` list with project name, allocation %, start date, and end date.
  - [x] Compute `available_capacity_pct` as `max(0, 100 - current_allocation_percentage)` for display; do not use it as an authorization or allocation-validity check.
  - [x] Treat `< 50% current utilization` as underutilized for this story. Do not flag missing-CTC resources as directly assignable; they may be visible but their action must route to the disabled/guarded state already used by `/team`.
  - [x] Derive utilization trends from `get_capacity_report_in_transaction(...)` so the weighted working-day formula stays canonical.
  - [x] Reuse `compute_department_budget_utilization(...)` for `Total Budget`, `Committed`, `Spent`, `Available`, `budget_health`, and threshold data. Do not calculate committed cost or budget health in dashboard frontend code.
  - [x] Keep Department Head scoping through the existing RLS transaction/current DB department path. Never trust only the JWT role string or a user-supplied department id.

- [x] **Task 2: Preserve backend performance, isolation, and failure behavior** (AC: #1, #2, #4)
  - [x] Keep lists bounded and deterministic. Team member rows may include the department team, but nested project lists should be bounded and stable; avoid unbounded nested assignment fan-out.
  - [x] Use existing joins/service outputs rather than per-member extra queries. The dashboard should still meet the <500ms data retrieval target for a 50-person department.
  - [x] Preserve savepoint-based optional degradation already used for budget and upcoming assignments. If trend data fails for a non-security reason, return the rest of the Department Head dashboard with a bounded warning.
  - [x] Do not expose CTC salary components, daily-rate ciphertext, encryption key metadata, raw audit payloads, or raw cost-preview internals. Department Head may see blended/daily rates on `/team`, but this story does not require new salary component exposure on `/dashboard`.
  - [x] Keep Admin and non-Department-Head dashboards stable. Optional team range query params must not break HR, PM, Finance, or Admin responses.

- [x] **Task 3: Render the Team Utilization dashboard in `DepartmentHeadPanel`** (AC: #1, #2, #3, #4)
  - [x] Update frontend DTOs in `src/frontend/src/pages/dashboard.rs` to deserialize the new Department Head fields with `#[serde(default)]` where compatibility requires it.
  - [x] Replace the current shallow Department Head panel with a dense operational dashboard:
    - summary stats for average utilization, available capacity, underutilized count, overallocated count, and budget health
    - team utilization table showing team members, current utilization %, current projects, available capacity, and status
    - utilization trend view for the selected range
    - department budget summary cards for Total Budget, Committed, Spent, Available
    - a visual budget gauge using existing `progress-track`, status colors, and `budget_health`
  - [x] Add a compact range control for utilization trends. Recommended presets: Current Month, Next 30 Days, 3 Months, 6 Months. Use a segmented control or select consistent with current dashboard density.
  - [x] Implement trend visualization with existing HTML/CSS and tables/inline bars; do not add a charting dependency. Provide table-readable values so the chart has an accessible equivalent.
  - [x] Keep the UI task-oriented and compact. Do not add explanatory in-app text about how dashboards work, polling, or implementation mechanics.
  - [x] Use stable dimensions for bars, gauges, buttons, and status badges so polling highlights and range changes do not shift layout.
  - [x] Use existing Huly/Tailwind classes (`panel`, `toolbar`, `stat-card`, `badge-positive`, `badge-warning`, `badge-negative`, `badge-neutral`, `progress-track`, `dashboard-change-flash`) before adding new utilities.

- [x] **Task 4: Reuse the existing assignment workflow for underutilized resources** (AC: #3)
  - [x] Add an "Assign" action only for assignable underutilized resources. Missing-CTC resources must show the same disabled/guarded posture as the `/team` table.
  - [x] Prefer dashboard navigation to `/team?assign_resource_id=<uuid>` rather than duplicating assignment modal logic inside `dashboard.rs`.
  - [x] Update `src/frontend/src/pages/team.rs` to parse the optional `assign_resource_id` query param after auth and team-member data are available.
  - [x] When the query id matches a loaded team member and the user can assign, open the existing assignment modal by reusing `open_assign_modal(...)` behavior.
  - [x] If the resource id is missing, not in the user's scoped team, missing CTC, or not assignable, show the existing bounded error/disabled state without leaking cross-department resource details.
  - [x] Clean the query param after opening/closing/submitting the modal using router navigation with replace semantics so the modal does not reopen on refresh or back/forward navigation.
  - [x] Keep existing assignment protections: assignable projects endpoint, cost preview, CTC required guard, overallocation warning confirmation, budget impact preview, and post-submit team refresh.

- [x] **Task 5: Extend dashboard changed-value tracking for new visible values** (AC: #1, #2, #4)
  - [x] Update `dashboard_value_map(...)` with deterministic keys for every new visible Department Head value, for example:
    - `department_head.team.<resource_id>.current_utilization_pct`
    - `department_head.team.<resource_id>.available_capacity_pct`
    - `department_head.team.<resource_id>.current_projects`
    - `department_head.team.<resource_id>.is_underutilized`
    - `department_head.trend.<resource_id>.<period>`
    - `department_head.budget.spent_actual_idr`
    - `department_head.budget.alert_threshold_pct`
  - [x] Continue ignoring `generated_at` for change detection.
  - [x] Treat newly visible team members, assignments, and trend periods as changed.
  - [x] Do not put hidden sensitive data into the comparison map. Only include values rendered on the dashboard.
  - [x] Bind `dashboard-change-flash` to new visible values without resizing text or changing row height.

- [x] **Task 6: Verify backend dashboard and team regressions** (AC: #1, #2, #3, #4)
  - [x] Extend `src/backend/tests/dashboard_tests.rs` with Department Head Story 6.4 coverage:
    - dashboard includes team members with current utilization, current projects, and available capacity
    - current utilization < 50% surfaces underutilized resource state
    - trend date range changes trend periods while preserving department scoping
    - budget payload includes total, committed, spent, remaining, health, and configured state
    - another department's resources/allocations are excluded even if ids are guessed
    - invalid trend date range returns validation error
  - [x] Run `cargo test --package xynergy-backend --test dashboard_tests`.
  - [x] Run `cargo test --package xynergy-backend --test team_tests`.
  - [x] Run `cargo test --package xynergy-backend --test budget_tests`.
  - [x] Run `cargo test --package xynergy-backend --test overallocation_tests` if assignment/deep-link behavior touches overallocations or capacity assumptions.

- [x] **Task 7: Verify frontend behavior and compile** (AC: #1, #2, #3, #4)
  - [x] Add or update pure frontend tests in `src/frontend/src/pages/dashboard.rs` for new Department Head change keys and any range/helper functions that can run natively.
  - [x] Add focused tests for underutilized threshold helper if introduced: 49.9% is underutilized, 50.0% is not.
  - [x] Run `cargo test -p xynergy-frontend --lib` if native frontend tests remain usable.
  - [x] Run `cargo check -p xynergy-frontend --target wasm32-unknown-unknown`.
  - [x] Run `npm --prefix src/frontend run build` if Tailwind classes/output change.
  - [x] Manually verify as a Department Head in a browser:
    - `/dashboard` shows team members, current utilization, current projects, available capacity, trend range selector, and budget gauge
    - changing the trend range refreshes data without breaking 30-second polling or manual Refresh
    - underutilized resource action opens the existing `/team` assignment modal for that resource
    - missing-CTC underutilized resource cannot be assigned and uses the existing disabled/guarded behavior
    - budget gauge shows Total, Committed, Spent, Available and status color/text
    - dashboard polling and manual Refresh still apply changed-value flashes to new Department Head values

### Review Findings

- [x] [Review][Patch] Non-DH dashboard requests could be rejected by Department Head-only range params — fixed by parsing and validating team range params only after role resolution for `department_head`; non-DH roles now ignore even malformed team range params.
- [x] [Review][Patch] Start-only future trend ranges defaulted `team_end_date` from today instead of the supplied start date — fixed by anchoring the default end date to the resolved start date.
- [x] [Review][Patch] Range cap used exclusive day math — fixed by enforcing the 366-day cap inclusively.
- [x] [Review][Patch] Trend range changes affected existing 30-day utilization summary metrics — fixed by keeping summary capacity on the operational 30-day window while applying selected ranges only to the trend bundle.
- [x] [Review][Patch] Trend-capacity failure hard-failed the Department Head dashboard — fixed with savepoint-backed optional degradation and a bounded warning for non-security failures.
- [x] [Review][Patch] Bounded current project lists were not deterministically ordered before truncation — fixed by sorting assignments before applying the per-member limit.
- [x] [Review][Patch] Assignment deep-links could be consumed before auth/team data loaded — fixed by waiting for auth validation and team load completion before acting.
- [x] [Review][Patch] Assignment deep-link cleanup dropped unrelated query/hash state — fixed by removing only `assign_resource_id` with replace navigation.
- [x] [Review][Patch] Guarded assignment deep-link failures were silently discarded — fixed with bounded error messages for malformed, unavailable, CTC-missing, or non-assignable resources.
- [x] [Review][Patch] Dashboard change tracking missed visible assignability/CTC and overallocated-member state — fixed by adding `ctc_status` and overallocated-member keys to `dashboard_value_map` and row flash bindings.
- [x] [Review][Patch] Frontend range preset date math could drift across DST transitions — fixed by using calendar-day date mutation instead of millisecond offsets.
- [x] [Review][Patch] Review artifacts still claimed a clean PASS after patching — updated manual, gate, trace, and validation artifacts to `review patched / reverify required` for the patched AC#2/AC#3 paths.
- [x] [Review][Patch] Manual verification screenshots were referenced but ignored by `.gitignore` — unignored the Story 6.4 screenshot directory.
- [x] [Review][Patch] Story 6.4 fixture could seed a known credential without an explicit local-only opt-in — added a fixture-specific GUC guard to setup/cleanup and disabled the known login during cleanup.
- [x] [Review][Patch] Default dashboard date could drift between route and service at UTC rollover — fixed by resolving one dashboard date at the route boundary and passing it through to the service.
- [x] [Review][Patch] Department Head dashboard trusted role + department_id without verifying `departments.head_id` — added relationship validation and a negative integration test.
- [x] [Review][Patch] Savepoint rollback failures were ignored during optional dashboard degradation — made rollback/release failures propagate instead of committing a bad transaction.
- [x] [Review][Patch] Department Head malformed trend date strings lacked HTTP integration coverage — added a 400 validation regression.
- [x] [Review][Patch] Assignment deep-link load failures could act on stale team data and duplicate `assign_resource_id` params were ambiguous — clear stale data on fetch failure and reject duplicate assign params with bounded errors.
- [x] [Review][Patch] Dashboard visible changes for budget health, member name/role, and overallocation state could miss flash bindings — added stable keys, flash bindings, and frontend regressions.
- [x] [Review][Patch] Trend table assumed the first member's periods defined every row's columns — render a sorted union of periods and preserve sparse cells.
- [x] [Review][Patch] Current-project change keys could collide on delimiter-containing project data — serialize sorted project tuples for deterministic change tracking.
- [x] [Review][Patch] Gate/manual artifacts mixed superseded PASS evidence with current REVIEW_PATCHED state — normalized AC#2/AC#3 to reverify-required and updated current test counts.
- [x] [Review][Patch] Fixture cleanup used broad UUID prefixes and an invalid disabled password hash — switched cleanup to exact fixture IDs and a randomized disabled email with a parseable Argon2 hash.
- [x] [Review][Patch] Duplicate Department Head range query keys could make non-DH dashboards fail before role resolution — fixed by moving dashboard query parsing behind role resolution with `RawQuery`; non-DH roles ignore duplicate DH-only params, while DH duplicate params return validation errors with route and integration regressions.
- [x] [Review][Patch] Frontend trend range presets used browser-local dates while the backend dashboard date is UTC — fixed by deriving preset ranges from UTC calendar dates and pure `NaiveDate` arithmetic with boundary tests.
- [x] [Review][Patch] Assignment deep-link could stay pending after a team fetch failure and open later after stale recovery — fixed by tracking team load failure and clearing the pending `assign_resource_id` query without opening the modal.
- [x] [Review][Patch] The existing at-risk member surface was removed while `top_at_risk` change keys remained — restored a compact At-risk Members panel so those visible values are preserved and flash bindings stay truthful.
- [x] [Review][Patch] Manual and trace artifacts still had contradictory AC labels and stale direct-deep-link wording — normalized AC#1/#2/#3 labels and replaced the stale "not a defect" appendix with the patched direct-navigation requirement.
- [x] [Review][Patch] Story 6.4 manual fixture hard-coded May 2026 allocation and budget periods — changed the fixture project, allocations, and department budget to use the current database month so future re-verification remains valid.
- [x] [Review][Patch] Review artifact test counts were stale after this rerun patch — updated story, sprint, gate, trace, and summary counts to dashboard_tests 57/57, backend lib 68/68, frontend lib 77/77, and 53 story-scoped tests.
- [x] [Review][Decision] AC#3 deep-link has no automated DOM/router harness — routed to Claude Opus 4.7; recommendation was to keep manual/browser reverify plus the existing cross-story frontend harness backlog because adding Playwright/wasm-bindgen-test infrastructure is a separate technical decision, not a Story 6.4 patch.
- [ ] [Review][Verification] Rerun Department Head browser verification for patched AC#2 selected-range behavior and AC#3 direct deep-link / guarded-error paths before promoting Story 6.4 to done.

## Dev Notes

### Developer Context

- Epic 6 is Dashboard & Reporting. Story 6.4 is the Department Head deepening story after Story 6.1 created the role-aware dashboard, Story 6.2 added polling/highlighting, and Story 6.3 deepened the Project Manager project-health cards. [Source: `_bmad-output/planning-artifacts/epics.md#Epic-6-Dashboard--Reporting`; `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md`; `_bmad-output/implementation-artifacts/6-2-real-time-dashboard-updates.md`; `_bmad-output/implementation-artifacts/6-3-project-health-dashboard.md`]
- Story 6.4 acceptance criteria require a Department Head team dashboard with team members, current utilization %, current projects, available capacity, trend range selection, assign action for resources below 50% current utilization, and budget summary with gauge. [Source: `_bmad-output/planning-artifacts/epics.md#Story-6.4-Team-Utilization-Dashboard`]
- PRD FR47 requires Department Heads to view team utilization rates and budget status. FR50/FR51 require 30-second dashboard polling and manual refresh. FR58 specifies the weighted working-day capacity formula; do not replace it with raw allocation sums. [Source: `_bmad-output/planning-artifacts/prd.md#7-Dashboard--Reporting`; `_bmad-output/planning-artifacts/prd.md#9-Capacity--Budget-Reporting-Enhancements-Post-Implementation`]
- Department Head access is relationship-based and department-scoped. PRD FR38 and FR60 require database relationship checks, not role-string-only authorization. [Source: `_bmad-output/planning-artifacts/prd.md#6-Access-Control--Security`; `_bmad-output/project-context.md#Security-Rules`]
- UX direction centers the Department Head flow on cost-aware assignment: view team dashboard, select a resource, see cost/budget impact, then confirm with context. The dashboard should speed that flow, not duplicate all assignment internals. [Source: `_bmad-output/planning-artifacts/ux-design-specification.md#Core-User-Experience`]

### Current File State To Preserve

- `src/backend/src/services/dashboard_service.rs`: `DepartmentHeadDashboard` currently returns `department_id`, `utilization`, optional `budget`, `overallocations`, `upcoming_assignments`, and `warnings`. `build_department_head_dashboard(...)` already uses `get_capacity_report_in_transaction(...)`, `get_team_members_in_transaction(...)`, `compute_department_budget_utilization(...)`, savepoints for optional upcoming/budget failure, and a 30-day default window.
- `src/backend/src/services/team_service.rs`: `get_team_members_in_transaction(...)` returns scoped employees with CTC status, current allocation percentage, overallocated flag, and active assignments. `get_capacity_report_in_transaction(...)` computes month-level weighted utilization using weekdays and allocation overlap.
- `src/backend/src/routes/team.rs`: `resolve_department_id(...)` uses the session/RLS department for Department Heads and only allows HR/Admin optional department override. `/team`, `/team/capacity-report`, `/team/budget`, and `/team/budget/breakdown` already enforce this scoping.
- `src/backend/src/services/budget_service.rs`: `DepartmentBudgetSummaryResponse` already includes `total_budget_idr`, `total_committed_idr`, `spent_actual_idr`, `spent_actual_source`, `remaining_idr`, `utilization_percentage`, `budget_health`, `alert_threshold_pct`, and `budget_configured`. The frontend dashboard DTO currently omits `spent_actual_idr` and `alert_threshold_pct`; add them if rendering those values.
- `src/frontend/src/pages/dashboard.rs`: `Dashboard` fetches only `/api/v1/dashboard`, preserves last successful data during refresh, uses request ordering/liveness guards, runs cleanup-safe 30-second polling, and flashes changed visible values through `dashboard_value_map(...)`. Preserve these protections.
- `src/frontend/src/pages/dashboard.rs::DepartmentHeadPanel`: currently renders average utilization, overallocated count, optional budget card/status, at-risk members, and upcoming assignments. This is the primary frontend surface to deepen for Story 6.4.
- `src/frontend/src/pages/team.rs`: already has the full team table, filtering/sorting, `/api/v1/team/capacity-report` range controls, department budget gauge/breakdown, assignment modal, assignable-project fetch, cost preview, CTC guard, overallocation confirmation, and post-submit team refresh. Reuse this workflow for assignment deep-linking.
- `src/frontend/style/tailwind.css`: existing Huly utilities and status classes should carry the dashboard. Only add utilities when repeated markup would otherwise become brittle.

### Technical Requirements

- Use backend-composed dashboard data for Department Head cards. The frontend should not call multiple role/domain endpoints from `/dashboard` to assemble ordinary dashboard state.
- Keep `GET /api/v1/dashboard` backward compatible. New query params should be optional and should not require PM/HR/Finance/Admin callers to change.
- Capacity trends must come from the existing weighted formula in `team_service.rs`, which divides allocated FTE-days by working days in the month. Do not use `current_allocation_percentage` as a substitute for selected-range trends.
- Current utilization is the current active allocation percentage from `TeamMemberResponse.current_allocation_percentage`.
- Available capacity display is `max(0, 100 - current utilization)`. Overallocated resources should show 0% available and negative/critical status.
- Underutilized means current utilization is strictly less than 50%. Keep this threshold explicit in tests.
- Budget gauge values must come from `DepartmentBudgetSummaryResponse`: Total Budget = `total_budget_idr`, Committed = `total_committed_idr`, Spent = `spent_actual_idr`, Available = `remaining_idr`.
- Money values remain integer IDR end to end. Use existing `format_idr(...)` in frontend; do not introduce floating-point currency math.
- Empty states are valid: no team members, no current projects, no configured budget, or no utilization trend rows must render gracefully.
- Do not add audit rows for successful dashboard reads. Existing mutations such as department budget upsert and assignment creation already audit through their domain flows.

### Architecture Compliance

- Keep Axum handlers thin. If route query params are added, route should parse/validate and delegate to service code.
- Keep service DTOs explicit with `#[derive(Debug, Serialize)]`; keep frontend DTOs explicit with `#[derive(Debug, Clone, Deserialize)]`.
- Use parameterized SQL and `.bind()` for any new queries. Do not concatenate date strings, ids, or sort fields into SQL.
- If `AssignmentSummary` needs project ids for stable current-project keys, add a typed optional `project_id` rather than replacing the existing field shape. Confirm the full `/team` page still deserializes correctly.
- Do not introduce a new dashboard route family. Story 6.4 should extend the existing role dashboard contract and deep-link into existing full-page workflows.
- No migration is expected. Use existing `resources`, `departments`, `allocations`, `projects`, `department_budgets`, and CTC tables.

### Library / Framework Requirements

- Local manifests/lockfiles are source of truth: backend uses Axum `0.7.9` and sqlx `0.7.4`; frontend uses Leptos `0.8.17`, `leptos_router` `0.8.12`, `gloo-timers` `0.3.0`, `reqwest` `0.11.27`, `web-sys` `0.3.85`, and Tailwind CSS `4.1.x`. Do not upgrade dependencies for this story. [Source: `Cargo.lock`; `Cargo.toml`; `src/backend/Cargo.toml`; `src/frontend/Cargo.toml`; `src/frontend/package-lock.json`]
- Official Leptos docs for the current line show effects rerun when reactive values read inside them change. If the dashboard fetch URL depends on range signals, ensure effect reruns cannot stack polling intervals or allow stale responses to win. [Source: `https://docs.rs/leptos/latest/leptos/reactive/effect/index.html`]
- Official Leptos `LocalResource` docs show `refetch`, but the existing dashboard already has explicit request ordering and visible-data continuity. Do not refactor to `LocalResource` unless those guarantees are preserved. [Source: `https://docs.rs/leptos/latest/leptos/prelude/struct.LocalResource.html`]
- Official gloo-timers docs for `Interval` state that dropping clears the interval and `forget()` leaks it. Preserve Story 6.2's owner-scoped interval cleanup. [Source: `https://oscore.gitlab.io/coap-ace-poc-webapp/doc/gloo_timers/callback/struct.Interval.html`; `https://docs.rs/crate/gloo-timers/0.3.0`]
- Official Axum docs describe `Query<T>` as deserializing query strings into a typed `serde::Deserialize` value. Use this pattern for any dashboard range query. [Source: `https://docs.rs/axum/latest/axum/extract/struct.Query.html`]
- `gloo-timers` latest is `0.4.0` and Leptos docs.rs latest is `0.8.19` as of research, but the project is locked to older compatible versions. Do not upgrade just to implement this story. [Source: `https://docs.rs/crate/gloo-timers/latest`; `Cargo.lock`]

### File Structure Requirements

- Primary backend targets:
  - `src/backend/src/routes/dashboard.rs`
  - `src/backend/src/services/dashboard_service.rs`
  - `src/backend/src/services/team_service.rs` only if `AssignmentSummary` or reusable DTOs need small additive fields
- Primary frontend targets:
  - `src/frontend/src/pages/dashboard.rs`
  - `src/frontend/src/pages/team.rs`
  - `src/frontend/style/tailwind.css` only if new reusable utilities are needed
  - `src/frontend/public/output.css` only after Tailwind build
- Primary tests:
  - `src/backend/tests/dashboard_tests.rs`
  - `src/backend/tests/team_tests.rs`
  - `src/backend/tests/budget_tests.rs`
  - `src/backend/tests/overallocation_tests.rs` if assignment/capacity behavior is touched
  - native tests inside `src/frontend/src/pages/dashboard.rs` for pure helpers/change-map keys where practical
- Avoid changing:
  - CTC detail routes and encryption logic
  - PM project-health files except shared dashboard DTO compile fallout
  - Finance dashboard, cash-flow, validation, and audit-report routes
  - Database migrations
  - Login routing, which already lands supported roles on `/dashboard`

### Testing Requirements

- Backend dashboard regression is mandatory: `cargo test --package xynergy-backend --test dashboard_tests`.
- Team scoping/capacity regression is mandatory: `cargo test --package xynergy-backend --test team_tests`.
- Budget regression is mandatory when rendering budget gauge fields: `cargo test --package xynergy-backend --test budget_tests`.
- Overallocation regression is mandatory if assignment/deep-link behavior touches assignment flow or capacity assumptions: `cargo test --package xynergy-backend --test overallocation_tests`.
- Frontend compile is mandatory: `cargo check -p xynergy-frontend --target wasm32-unknown-unknown`.
- Frontend native tests should run if helper tests are added: `cargo test -p xynergy-frontend --lib`.
- Tailwind regeneration is mandatory if `src/frontend/style/tailwind.css` or generated utility usage changes: `npm --prefix src/frontend run build`.
- Manual browser verification should use a Department Head account with at least one underutilized resource, one overallocated/resource-at-risk case, active assignments, and configured budget.

### Previous Story Intelligence

- Story 6.1 created `GET /api/v1/dashboard`, the Department Head dashboard summary, and backend tests for department scoping, utilization, overallocations, upcoming assignments, and optional-card degradation. Build on that route/service rather than adding a second dashboard API. [Source: `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md#Tasks--Subtasks`; `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md#Review-Findings`]
- Story 6.1 review fixed stale JWT department trust and required current DB/RLS department scoping. Story 6.4 must preserve that; do not accept arbitrary `department_id` from dashboard query params. [Source: `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md#Review-Findings`; `src/backend/src/routes/team.rs`]
- Story 6.2 added cleanup-safe polling, request ordering, generated timestamp display, and value-level changed flashes. New Department Head values must be included in `dashboard_value_map(...)`, and range-driven fetches must preserve stale-response protection. [Source: `_bmad-output/implementation-artifacts/6-2-real-time-dashboard-updates.md#Completion-Notes-List`; `src/frontend/src/pages/dashboard.rs`]
- Story 6.2 review fixed missing Department Head flash bindings for committed/total budget text and stale request invalidation. Keep those fixes when rewriting `DepartmentHeadPanel`. [Source: `_bmad-output/implementation-artifacts/6-2-real-time-dashboard-updates.md#Review-Findings`]
- Story 6.3 deepened the PM dashboard by adding fields to the existing role-dashboard contract, using existing services for canonical calculations, client-side controls for sorting, and a query deep-link to an existing full page instead of duplicating P&L UI. Apply the same pattern here: backend-composed summary, frontend operational controls, deep-link to `/team` for assignment. [Source: `_bmad-output/implementation-artifacts/6-3-project-health-dashboard.md#Completion-Notes-List`]
- Story 6.3 review found query deep-links can leave stale state if close actions do not clean query params through the router. Use router-aware replace navigation when closing the `/team` assignment deep-link. [Source: `_bmad-output/implementation-artifacts/6-3-project-health-dashboard.md#Review-Findings`]
- Story 3.5 and later team/budget work established `resolve_department_id()` consistency and budget health thresholds. Do not bypass the helper or create a second budget-health implementation for the dashboard. [Source: `_bmad-output/implementation-artifacts/3-5-department-budget-utilization.md`; `src/backend/src/routes/team.rs`; `src/backend/src/services/budget_service.rs`]

### Git Intelligence Summary

- Recent commits are vertical story slices with story artifact, implementation, focused tests, generated CSS when needed, browser verification, and review patches:
  - `12801ad feat: implement Story 6.3 - Project Health Dashboard...`
  - `47b15cd feat: implement Story 6.2 - Real-Time Dashboard Updates...`
  - `2ab8ce7 feat: implement Story 6.1 - Role-Based Dashboard...`
- The most recent dashboard work heavily touched `src/frontend/src/pages/dashboard.rs`, `src/backend/src/services/dashboard_service.rs`, `src/backend/tests/dashboard_tests.rs`, and generated Tailwind output. Read current code before editing and avoid overwriting polling/sort/deep-link fixes.
- The working tree was clean before this story file was created.

### Implementation Pitfalls To Avoid

- Do not add frontend fan-out from `/dashboard` to `/team`, `/team/capacity-report`, and `/team/budget` for normal dashboard rendering. Extend the backend dashboard contract instead.
- Do not build another assignment modal in `dashboard.rs`; reuse `/team`.
- Do not let the assignment deep-link bypass CTC-required checks, assignable-project filtering, cost preview, overallocation confirmation, or budget-impact preview.
- Do not use raw current allocation sums for trend AC. Use the weighted capacity report for selected ranges.
- Do not trust query params for department scope.
- Do not render missing-CTC employees as assignable just because they are under 50% utilized.
- Do not break Admin, HR, PM, or Finance dashboard sections while adding optional Department Head range behavior.
- Do not let date-range changes stack polling intervals or allow old dashboard responses to overwrite newer selected-range responses.
- Do not mark this story complete until the underutilized assignment action, trend range changes, budget gauge fields, polling refresh, changed-value flashes, and regression tests are verified.

### References

1. `_bmad-output/planning-artifacts/epics.md` - Epic 6 and Story 6.4 acceptance criteria.
2. `_bmad-output/planning-artifacts/prd.md` - FR38, FR45-FR51, FR58-FR61, NFR2, NFR7, and security/access constraints.
3. `_bmad-output/planning-artifacts/architecture.md` - Rust/Leptos/Axum/PostgreSQL constraints, dashboard/reporting direction, and implementation sequence.
4. `_bmad-output/planning-artifacts/ux-design-specification.md` - Department Head cost-aware assignment flow, 30-second polling MVP, Stripe/Linear financial-dashboard UX, status colors, and accessibility direction.
5. `_bmad-output/project-context.md` - project guardrails for Rust, Leptos, Axum, relationship-based RBAC, parameterized SQL, and testing.
6. `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md` - role dashboard contract, Department Head foundation, scoping review fixes.
7. `_bmad-output/implementation-artifacts/6-2-real-time-dashboard-updates.md` - polling, request ordering, cleanup, and change-highlight patterns.
8. `_bmad-output/implementation-artifacts/6-3-project-health-dashboard.md` - previous dashboard deepening pattern and router-aware deep-link lessons.
9. `src/backend/src/services/dashboard_service.rs` - current Department Head dashboard service and response contract.
10. `src/backend/src/services/team_service.rs` - canonical team members and weighted capacity report logic.
11. `src/backend/src/routes/team.rs` - Department Head scoping and team/budget/capacity endpoints.
12. `src/backend/src/services/budget_service.rs` - canonical department budget utilization contract and health logic.
13. `src/frontend/src/pages/dashboard.rs` - Department Head panel, polling, request ordering, and change detection.
14. `src/frontend/src/pages/team.rs` - existing team table, capacity report, budget gauge, and assignment workflow to reuse.
15. `src/backend/tests/dashboard_tests.rs` - role-dashboard and Department Head scoping regressions.
16. `src/backend/tests/team_tests.rs` - team endpoint scoping and allocation aggregation regressions.
17. `src/backend/tests/budget_tests.rs` - department budget contract and health threshold regressions.
18. `src/backend/tests/overallocation_tests.rs` - capacity/overallocation behavior.
19. `https://docs.rs/leptos/latest/leptos/reactive/effect/index.html` - Leptos effect behavior.
20. `https://docs.rs/leptos/latest/leptos/prelude/struct.LocalResource.html` - Leptos `LocalResource`/`refetch` reference.
21. `https://docs.rs/axum/latest/axum/extract/struct.Query.html` - typed query extractor reference.
22. `https://docs.rs/crate/gloo-timers/0.3.0` - project-locked timer crate reference.
23. `https://oscore.gitlab.io/coap-ace-poc-webapp/doc/gloo_timers/callback/struct.Interval.html` - gloo-timers 0.3 `Interval` RAII/leak behavior.

## Story Completion Status

- Status: review
- Completion note: Story 6.4 implementation complete — Department Head dashboard now exposes team utilization (per-member rows with current utilization, available capacity, current projects, status badge, and Assign deep-link), a range-aware utilization trend table, a full department budget gauge (Total/Committed/Spent/Available with health-colored bar), and extended change-detection keys for polling. Backend route extended with typed optional team_start_date/team_end_date query params (validated for ordering and 366-day cap). `/team` parses the `assign_resource_id` deep-link and reopens the existing assignment modal with router-replace cleanup. Backend dashboard/team/budget/overallocation tests all green; 14 new frontend native tests cover thresholds, helpers, and per-key change detection.

## Dev Agent Record

### Agent Model Used

claude-opus-4-7 via bmad-dev-story workflow.

### Debug Log References

- `cargo test --package xynergy-backend --test dashboard_tests` → 57 passed (Story 6.4 coverage plus review patch regressions for DH relationship RBAC, malformed/duplicate trend range params, and non-DH duplicate range immunity).
- `cargo test --package xynergy-backend --test team_tests` → 10 passed (no regression).
- `cargo test --package xynergy-backend --test budget_tests` → 18 passed (no regression).
- `cargo test --package xynergy-backend --test overallocation_tests` → 7 passed (no regression).
- `cargo test --package xynergy-backend --lib` → 68 passed (route range resolver and duplicate query parser unit tests green).
- `cargo test -p xynergy-frontend --lib` → 77 passed (Story 6.4 helper/change-map/date-preset coverage remains green after review patches).
- `cargo check -p xynergy-frontend --target wasm32-unknown-unknown` → clean (9 pre-existing dead-field warnings unrelated to this story).
- `cargo check --package xynergy-backend` (with `SQLX_OFFLINE=true`) → clean.
- `npm --prefix src/frontend run build` → Tailwind output regenerated.
- One initial test failure (`dept_head_budget_payload_has_gauge_fields`) traced to a wrong column name in the budget seed (`created_by`/`updated_by` not present in `department_budgets` schema); fixed by aligning the seed with the actual schema. Re-run → pass.

### Completion Notes List

- Backend dashboard contract additive: new `DepartmentHeadDashboard` fields `team_members`, `utilization_trends`, `underutilized_members` plus existing `utilization`, `budget`, `overallocations`, `upcoming_assignments`, `warnings` preserved.
- New optional query params `team_start_date` / `team_end_date` on `GET /api/v1/dashboard` are parsed and validated only for Department Head dashboards (start ≤ end, ≤ 366 inclusive days, duplicate keys rejected), then threaded to the service through a `DepartmentHeadTeamRange` value type with one route-resolved dashboard date to avoid UTC rollover drift. Non-DH roles ignore even malformed or duplicate team range params (covered by `dashboard_range_query_does_not_affect_admin_response`, `dashboard_malformed_range_query_is_ignored_for_admin`, and `dashboard_duplicate_range_query_is_ignored_for_admin`).
- Available capacity computed as `max(0, 100 - current_allocation_percentage)` and rounded to 1 dp; never used for authorization.
- Underutilized threshold: strict `< 50.0%`, encoded both on backend (`DH_UNDERUTILIZED_THRESHOLD_PCT`) and frontend (`is_underutilized_threshold`) with explicit 49.9/50.0 boundary tests.
- Trends derived from `get_capacity_report_in_transaction` over the user-selected range — preserves the canonical weighted working-day formula. The existing 30-day utilization summary remains anchored to the operational window; selected ranges affect only the trend bundle.
- Frontend `DepartmentHeadPanel` now renders four stat cards (avg utilization, avg available, underutilized count, overallocated count) plus optional budget tile, a full `DepartmentBudgetGauge` (Total/Committed/Spent/Available + colored progress bar + threshold), a dense `DepartmentTeamUtilizationTable` (per-row Assign button for assignable underutilized members, disabled state for missing CTC), a range-aware `DepartmentUtilizationTrend` table with inline bars, plus At-risk, Overallocated, and Upcoming Assignments panels.
- Range preset control (Current Month / Next 30 Days / 3 Months / 6 Months) is resolved on the frontend from UTC calendar dates via `js_sys::Date` + `NaiveDate` arithmetic and triggers a fresh fetch through a dedicated Effect that resets `prev_value_map` so a range switch doesn't flash the whole dashboard.
- 30-second polling preserved: the existing `Interval` reads `team_range.get_untracked()` on each tick, so it follows the latest selection without stacking timers.
- Deep-link reuse: `/dashboard` → `/team?assign_resource_id=<uuid>` → `team.rs` Effect waits for auth and team data, normalizes the UUID, opens the existing `open_assign_modal` for in-scope assignable members, and clears only the `assign_resource_id` query param with replace navigation. Out-of-scope, malformed, duplicate, CTC-missing, non-assignable, or team-load-failed IDs show bounded errors/cleanup without leaking cross-department details. This reuses the assignable-projects fetch, cost preview, CTC guard, overallocation confirmation, and post-submit team refresh from Story 3.x.
- `dashboard_value_map` extended with: `department_head.team.member_count`, `.underutilized_count`, `.avg_available_capacity_pct`, per-member `team.<id>.{resource_name, role, current_utilization_pct, available_capacity_pct, is_underutilized, is_overallocated, ctc_status, current_projects}`, overallocated-member keys, per-period `trend.<id>.<period>`, and budget `spent_actual_idr` / `alert_threshold_pct` / `budget_health`.
- Frontend DTO additions are tolerant (`#[serde(default)]`) so an older backend deploy doesn't break the dashboard during a rolling rollout.
- Stable bar heights / fixed-width inline trend bars prevent polling highlights from shifting layout.
- Manual browser verification artifacts exist from the initial implementation run, but code-review patches changed deep-link timing/error behavior and range handling. Re-run patched AC#2/AC#3 Department Head browser verification before promoting to done.

### File List

- `src/backend/src/routes/dashboard.rs` — modified: added role-gated raw query parsing + `resolve_team_range` validation with unit tests; threaded `team_range` and one route-resolved dashboard date into `build_dashboard`.
- `src/backend/src/services/dashboard_service.rs` — modified: added `TeamUtilizationMember`, `TeamUtilizationCurrentProject`, `TeamUtilizationTrendBundle`, `TeamUtilizationTrend`, `TeamUtilizationTrendPeriod`, `DepartmentHeadTeamRange`, constants (`DH_TEAM_RANGE_DEFAULT_DAYS`, `DH_TEAM_RANGE_MAX_DAYS`, `DH_UNDERUTILIZED_THRESHOLD_PCT`, `DH_CURRENT_PROJECTS_PER_MEMBER_LIMIT`); extended `DepartmentHeadDashboard` and `build_department_head_dashboard` with team-member projection, trend bundle, underutilized filter, relationship-based Department Head validation, and propagating savepoint rollback failures.
- `src/backend/tests/dashboard_tests.rs` — modified: added Story 6.4 integration coverage, review regressions for malformed/duplicate non-DH/DH range params and Department Head relationship RBAC, and `get_dashboard_with_query` helper.
- `src/frontend/src/pages/dashboard.rs` — modified: new DTOs (`TeamUtilizationMember`, `TeamUtilizationCurrentProject`, `TeamUtilizationTrendBundle`, `TeamUtilizationTrend`, `TeamUtilizationTrendPeriod`); extended `DepartmentBudgetSummary` with `spent_actual_idr` + `alert_threshold_pct`; `UtilizationSummary` gained `start_date`/`end_date`; `TrendRangePreset` enum + UTC `NaiveDate` range helpers; `fetch_role_dashboard` now accepts an optional preset and appends typed query params; `Dashboard` component owns the `team_range` signal and triggers re-fetch on change with `prev_value_map` reset; `DashboardBody` threads the range to `DepartmentHeadPanel`; new components `DepartmentBudgetGauge`, `DepartmentTeamUtilizationTable`, `DepartmentUtilizationTrend`; At-risk Members panel restored; extended `dashboard_value_map` with all new visible Department Head keys; 20 new native tests in `tests` module.
- `src/frontend/src/pages/team.rs` — modified: imported `NavigateOptions`; added deep-link Effect that consumes one `?assign_resource_id=<uuid>`, validates UUID + RLS scope + CTC posture, waits for loaded team data, rejects duplicate params, handles team-load failure cleanup, opens the existing assignment modal, and clears the query param with `navigate("/team", replace: true)`.
- `src/frontend/public/output.css` — regenerated via `npm --prefix src/frontend run build`.
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — modified: 6-4 moved ready-for-dev → in-progress → review with audit comments.

## Change Log

| Date       | Author | Change |
|------------|--------|--------|
| 2026-05-22 | Putu   | Created Story 6.4 ready-for-dev context for the Team Utilization Dashboard. |
| 2026-05-22 | Putu   | Implemented Story 6.4 — backend dashboard contract extended with team utilization surface and trend range query params; frontend `DepartmentHeadPanel` rewritten with dense team table, trend range control, and budget gauge; `/team` deep-link wired for the assignment workflow; full backend + frontend test suites green. Story moved to review. |
| 2026-05-22 | Putu   | Applied rerun code-review patches: DH relationship RBAC, dashboard-date consistency, savepoint rollback propagation, assignment deep-link guards, dashboard flash/trend-table hardening, fixture cleanup safety, and review-patched gate artifact normalization. Story remains review pending patched browser re-verification. |
| 2026-05-22 | Putu   | Applied full-diff review rerun patches: duplicate DH query key handling, UTC trend preset dates, team-load-failure deep-link cleanup, At-risk panel restoration, dynamic manual fixture month, artifact consistency, and Claude-routed AC#3 harness decision. Story remains review pending patched browser re-verification. |
