# Story 6.3: Project Health Dashboard

Status: done

<!-- Ultimate context engine analysis completed - comprehensive developer guide created. -->

## Story

As a **Project Manager**,
I want **a project health dashboard showing budget, P&L, and margin status**,
so that **I can quickly identify projects needing attention**.

## Acceptance Criteria

1. **Given** I navigate to Projects Dashboard **when** the page loads **then** I see project cards with: Name, Budget Status (green/yellow/red), Current Margin, Forecast Margin.
2. **Given** I view the project list **when** a project exceeds budget **then** its status indicator shows red **and** a warning icon appears.
3. **Given** I view the project list **when** a project's margin is below target **then** the margin is displayed in orange/red **and** I can click for detailed P&L view.
4. **Given** I have multiple projects **when** I view the dashboard **then** I can sort by: Margin, Budget Utilization, End Date.

## Scope Boundary

- **In scope**: deepen the Project Manager section of the existing role-based `/dashboard`, extend the backend PM dashboard card contract with forecast margin and budget-overrun fields, render green/yellow/red budget status, render current and forecast margin status against target, add PM-card sorting by margin/budget utilization/end date, preserve Story 6.2 polling/change-highlight behavior for the new visible PM values, and add a direct route from a PM card to the existing `/projects` P&L detail surface.
- **In scope for `/projects`**: add a narrow query/deep-link behavior such as `/projects?view=pnl&project_id=<uuid>` so dashboard clicks open the already implemented P&L panel for that project. The existing Projects list, budget, resource-cost, revenue, and forecast workflows must keep working without query params.
- **Not in scope**: new dashboard endpoint families, new charting libraries, SSE/WebSockets, Story 6.4 team utilization dashboard, Story 6.5 CTC completeness dashboard, executive BI, new P&L formulas, budget mutation workflows, revenue entry changes, or replacing the Story 6.1/6.2 dashboard polling implementation.
- **Hard dependencies**: reuse `GET /api/v1/dashboard`, `get_project_pl_dashboard(...)`, `get_project_pl_forecast(...)`, and the existing `/api/v1/projects/:id/pl` and `/api/v1/projects/:id/pl/forecast` detail endpoints. Do not recalculate P&L or forecast formulas in the frontend.

## Tasks / Subtasks

- [x] **Task 1: Extend PM dashboard backend health card contract** (AC: #1, #2, #3)
  - [x] Update `src/backend/src/services/dashboard_service.rs::ProjectHealthCard` to keep all existing fields and add forecast/health fields needed by the UI, recommended:
    - `budget_utilization_pct: f64`
    - `is_over_budget: bool`
    - `budget_overrun_idr: i64`
    - `projected_total_cost_idr: i64`
    - `forecast_margin_pct: f64`
    - `forecast_variance_from_target_pct: f64`
    - `health_status: String` or equivalent stable severity (`healthy`, `warning`, `critical`, `unconfigured`)
  - [x] Continue deriving `Debug` and `Serialize`; keep response fields explicit and typed.
  - [x] In `build_project_manager_dashboard(...)`, keep PM scoping from `projects.project_manager_id = user_id`, `LOWER(status) = 'active'`, `end_date >= today`, and `PM_ACTIVE_PROJECT_LIMIT`.
  - [x] Reuse `get_project_pl_dashboard(pool, project_id, current_year)` for current margin, target margin, margin threshold, revenue, cost, and current margin alert.
  - [x] Reuse `get_project_pl_forecast(pool, project_id, current_year, Some(today))` for forecast margin, projected total cost, and variance from target.
  - [x] Do not duplicate forecast, burn-rate, margin, or resource-cost formulas in `dashboard_service.rs`; only adapt canonical service outputs into dashboard card fields.
  - [x] If P&L or forecast data fails for one project, keep the role dashboard usable: add a bounded warning and return a safe card for that project instead of blanking the whole PM dashboard unless the failure is security-critical.
  - [x] Keep currency values as integer IDR (`i64`); use `f64` only for percentages already represented as percentages.
  - [x] Do not add audit rows for successful dashboard reads. Existing project detail endpoints already audit access-denied cases.

- [x] **Task 2: Preserve performance and data isolation while adding forecast data** (AC: #1, #2, #3)
  - [x] Avoid unbounded fan-out. The dashboard must never fetch forecasts for more than `PM_ACTIVE_PROJECT_LIMIT` projects.
  - [x] Prefer a helper that builds one card from one project row and uses `tokio::try_join!` for that project's P&L plus forecast calls where practical.
  - [x] If collecting multiple card futures, use the existing `futures` dependency and preserve per-card warning degradation rather than failing all cards on one non-security data error.
  - [x] Ensure PM users cannot infer non-owned, planning, completed, cancelled, closed, or already-ended projects through card counts, warnings, sort order, or links.
  - [x] Preserve Admin dashboard behavior. This story targets the Project Manager role section only; do not replace Admin's operational totals with PM cards.

- [x] **Task 3: Update frontend PM DTOs and change-detection keys** (AC: #1, #2, #3)
  - [x] Update `src/frontend/src/pages/dashboard.rs::ProjectHealthCard` to deserialize all new backend fields. Existing extra backend fields `target_margin_pct` and `margin_alert_threshold_pct` should become visible to the frontend if used for status coloring.
  - [x] Extend `dashboard_value_map(...)` with deterministic keys for every new visible PM value, for example:
    - `project_manager.project.<project_id>.budget_utilization_pct`
    - `project_manager.project.<project_id>.is_over_budget`
    - `project_manager.project.<project_id>.budget_overrun_idr`
    - `project_manager.project.<project_id>.forecast_margin_pct`
    - `project_manager.project.<project_id>.projected_total_cost_idr`
    - `project_manager.project.<project_id>.forecast_variance_from_target_pct`
    - `project_manager.project.<project_id>.health_status`
  - [x] Preserve Story 6.2 rules: ignore `generated_at` for change detection, use stable project ids where available, treat newly visible cards as changed, and keep highlight timeout cleanup unchanged.
  - [x] Do not include hidden or sensitive values in the comparison map. PM dashboard values are budget/P&L summary values only; never add CTC salary components, ciphertext, key metadata, or raw audit payloads.

- [x] **Task 4: Render project health cards with clear financial status** (AC: #1, #2, #3)
  - [x] In `ProjectManagerPanel`, keep the existing Huly/Tailwind layout and replace the current plain project-health text block with scannable cards showing at minimum:
    - Project name
    - End date
    - Budget status badge/indicator
    - Budget utilization percentage
    - Budget spent/remaining or overrun amount
    - Current margin and target margin
    - Forecast margin and variance from target
  - [x] Map budget status/severity to existing design tokens/classes:
    - healthy/on target -> positive styling
    - warning/near target -> warning styling
    - critical/over budget -> negative styling
    - unconfigured/no data -> neutral styling
  - [x] When `is_over_budget` is true or `budget_spent_idr > total_budget_idr`, render a red status indicator and warning icon next to the project status/budget summary. Existing inline SVG style is acceptable in this codebase; keep it accessible with a text label or `aria-label`.
  - [x] Color current margin using target/threshold logic: positive when at or above target, warning when below target but not past the alert threshold, negative when `margin_alert` is present or the threshold is exceeded.
  - [x] Color forecast margin using `forecast_variance_from_target_pct` and `margin_alert_threshold_pct` with the same positive/warning/negative posture.
  - [x] Keep cards compact and stable: no layout shift during polling highlights, no text resizing, no auto-scroll, and no marketing-style hero content.
  - [x] Keep changed-value highlight bindings for all new visible values.

- [x] **Task 5: Add sorting controls for Project Manager cards** (AC: #4)
  - [x] Add a compact segmented control or equivalent button group in the Project Health toolbar with sort modes: Margin, Budget Utilization, End Date.
  - [x] Sort client-side from the already-loaded `active_projects` data; do not add a dashboard API query parameter unless there is a proven need.
  - [x] Recommended ordering:
    - Margin: lowest current margin first, then project name.
    - Budget Utilization: highest utilization or overrun first, then project name.
    - End Date: soonest end date first, then project name.
  - [x] Make the selected sort mode keyboard reachable and visually clear; use `aria-pressed` or a selected state equivalent.
  - [x] Sorting must not reset polling state, clear warnings, or trigger extra network requests.

- [x] **Task 6: Deep-link PM dashboard cards to the existing P&L detail view** (AC: #3)
  - [x] Add a per-project "P&L" or "View P&L" action on each PM health card.
  - [x] Preferred implementation: navigate to `/projects?view=pnl&project_id=<uuid>` and update `src/frontend/src/pages/projects.rs` to read the query once on mount or location change, validate/parse the UUID, set `pnl_project_id`, and open the existing P&L panel.
  - [x] The Projects page must still work normally at `/projects` with no query params.
  - [x] Do not create a parallel P&L modal in `dashboard.rs`; reuse the existing `Projects` page P&L resource, forecast toggle, settings, and year controls.
  - [x] If the deep-linked project is no longer accessible, rely on existing project/P&L endpoint access checks and show the existing bounded error state without leaking project details.

- [x] **Task 7: Verify backend dashboard contract and regressions** (AC: #1, #2, #3)
  - [x] Extend `src/backend/tests/dashboard_tests.rs` for the new PM card fields:
    - PM cards include forecast margin and budget utilization fields.
    - Over-budget project returns `is_over_budget = true`, positive `budget_overrun_idr`, and critical/negative health status.
    - Current margin below target surfaces existing `margin_alert` semantics.
    - Forecast margin below target surfaces forecast variance/critical or warning status.
    - PM still sees only owned active non-ended projects.
    - Active project cards remain bounded to `PM_ACTIVE_PROJECT_LIMIT`.
  - [x] Run `cargo test --package xynergy-backend --test dashboard_tests`. Passed live with `DATABASE_URL=postgres://xynergy:xynergy@localhost:5432/xynergy cargo test -p xynergy-backend --test dashboard_tests` after starting local Postgres; 37 passed, 0 failed.
  - [x] Re-run targeted P&L forecast regression coverage: `cargo test --package xynergy-backend --test project_pl_forecast_tests`. Passed live with `DATABASE_URL=postgres://xynergy:xynergy@localhost:5432/xynergy cargo test -p xynergy-backend --test project_pl_forecast_tests`; 11 passed, 0 failed.
  - [x] If shared budget/P&L helpers are touched, also run `cargo test --package xynergy-backend --test project_pl_tests` and the relevant project budget/expense suite. Passed live with `DATABASE_URL=postgres://xynergy:xynergy@localhost:5432/xynergy cargo test -p xynergy-backend --test project_pl_tests`; 10 passed, 0 failed. `project_budget_tests` compiled cleanly in prior verification.

- [x] **Task 8: Verify frontend behavior and compile** (AC: #1, #2, #3, #4)
  - [x] Add pure frontend unit tests for any sort helper if practical in `dashboard.rs`:
    - margin sort puts lowest margin first
    - budget utilization sort puts highest/over-budget first
    - end date sort puts soonest date first
  - [x] Add or update change-detection tests for the new PM forecast and over-budget keys.
  - [x] Run `cargo test -p xynergy-frontend --lib` if the frontend test target remains usable. 51 tests passed (34 pre-existing + 17 Story 6.3 expansion tests covering PM sort modes, visual-state helpers, forecast/over-budget change keys, and PM sensitive-field exclusion).
  - [x] Run `cargo check -p xynergy-frontend --target wasm32-unknown-unknown`. Passed with only pre-existing dead-code warnings in `team.rs`.
  - [x] Run `npm --prefix src/frontend run build` if `src/frontend/style/tailwind.css` or Tailwind classes/output changed. Rebuilt `src/frontend/public/output.css` to refresh utility scanning.
  - [x] Manually verify `/dashboard` and the PM card deep-link in a browser if a dev server is available:
    - Project Manager dashboard shows project health cards with current and forecast margin.
    - Over-budget project shows red status plus warning icon.
    - Below-target margin shows warning/negative styling.
    - Sorting by Margin, Budget Utilization, and End Date works without fetching new data.
    - Clicking card P&L action opens the existing `/projects` P&L panel for that project.
    - 30-second polling and manual Refresh still update card values and apply changed-value flash.
    - _Completed 2026-05-22 with local Postgres + `xynergy-server` + in-app browser. PM fixture `pm.verify@xynergy.com` verified 4 project health cards, over-budget accessible warning label (`Over budget by Rp 8.000.000`), below-target margin alerts, sort ordering for Margin / Budget Utilization / End Date, `/projects?view=pnl&project_id=<uuid>` deep-link into the existing P&L panel, close-query cleanup back to `/projects`, manual Refresh timestamp update, and 30-second polling value update with `dashboard-change-flash`._

### Review Findings

- [x] [Review][Patch] Zero-revenue forecast variance marked new budgeted projects critical — fixed by requiring a real forecast revenue signal before forecast variance can drive `health_status` warning/critical.
- [x] [Review][Patch] Budget status badge was driven by combined `health_status` — fixed by mapping the top budget badge from `budget_status`/over-budget only and rendering `health_status` as a separate visible health row.
- [x] [Review][Patch] Low/no-revenue cards could render current/forecast margin as red without canonical revenue signal — fixed by muting margin and forecast styling until revenue exists unless a canonical margin alert is present.
- [x] [Review][Patch] `projected_total_cost_idr` could flash as a hidden value — fixed by rendering projected cost in the forecast row.
- [x] [Review][Patch] `/projects` P&L deep-link state could stay stale when query params changed or became invalid — fixed by clearing the active P&L panel on non-`pnl` or invalid query params and applying new valid project IDs idempotently.
- [x] [Review][Patch] Existing Projects forecast polling effect stacked intervals after Story 6.3 touched the file — fixed by storing a single owner-scoped `Interval` handle and clearing it on effect changes/unmount.
- [x] [Review][Patch] Healthy/warning `health_status` branches were claimed but not asserted — fixed by adding backend assertions for healthy and warning branch outcomes.
- [x] [Review][Patch] Story automation text still reported 34 frontend tests after the automation expansion — fixed to 51 tests and aligned the badge-helper test name in trace artifacts.
- [x] [Review][Decision] Gate/coverage artifacts appeared to mark 100% coverage while manual browser and live PostgreSQL execution remained pending — routed through Claude (`claude-opus-4-7`, xhigh), then superseded by generated-artifact review to separate static/logic coverage from runtime verification; final follow-up verification moved the gate to `PASS_WITH_ADVISORY`.
- [x] [Review][Patch] Final pass found forecast styling still used annual revenue as the revenue signal — fixed by passing `forecast_has_revenue_signal` from backend to frontend and binding forecast color/text to that field.
- [x] [Review][Patch] P&L-unavailable fallback cards could look budget-healthy with 0% spend — fixed by returning neutral `unconfigured` budget status from `safe_unavailable_card`.
- [x] [Review][Patch] PM sort mode reset after refresh because the signal lived inside the rebuilt panel — fixed by lifting PM sort state to the dashboard component and passing it through the body/panel.
- [x] [Review][Patch] `margin_alert_threshold_pct` and forecast revenue-signal changes were present in the value map but not flash bindings — fixed by including both in card, margin, and forecast flash key sets and folding assertions into the existing frontend key test.
- [x] [Review][Patch] Closing the `/projects?view=pnl&project_id=<uuid>` panel left stale query params in the URL — fixed by navigating back to `/projects` when the P&L panel is closed.
- [x] [Review][Patch] Traceability artifacts described forecast failure as the `safe_unavailable_card` path — fixed wording to state forecast failure is inline while `safe_unavailable_card` is the P&L failure fallback.
- [x] [Review][Decision] The full diff includes untracked `automation-summary-6-1.md` while the generic automation summary now documents Story 6.3 — routed through Claude (`claude-opus-4-7`, xhigh), resolved as a technical artifact-tracking issue by keeping `automation-summary-6-1.md` as the outgoing Story 6.1 snapshot per the existing per-story summary convention (`automation-summary-5-3.md`, `automation-summary-6-2.md`).
- [x] [Review][Patch] Frontend lib tests do not compile because `pm_card_default` omits `forecast_unavailable` — fixed by initializing the field in the PM test helper and re-running `SQLX_OFFLINE=true cargo test -p xynergy-frontend --lib` successfully.
- [x] [Review][Patch] Story verification claims are stale because mandatory frontend tests fail and backend/forecast suites were not executed — fixed by updating the story verification notes with successful frontend/backend checks; final follow-up ran live `dashboard_tests`, `project_pl_forecast_tests`, and `project_pl_tests` successfully.
- [x] [Review][Patch] Forecast-unavailable PM cards can still report `health_status="healthy"` — fixed by making forecast unavailability on configured-budget PM cards produce warning health unless a critical budget/current-margin condition dominates.
- [x] [Review][Patch] Negative forecast variance with a zero alert threshold renders warning instead of critical — fixed by treating any negative variance beyond `max(threshold, 0)` as negative styling.
- [x] [Review][Patch] Closing the P&L deep link mutates history outside the Leptos router and can leave stale query navigation state — fixed by moving the close action into a small router-aware component that navigates to `/projects` with `replace: true`.
- [x] [Review][Patch] Forecast-driven warning/critical PM health has no focused backend regression coverage — fixed by adding a dashboard integration test where healthy current spend/margin is overridden by below-target forecast variance.
- [x] [Review][Patch] Generated automation and traceability artifacts contradicted the story state and unchecked verification subtasks — fixed first by explicitly recording blockers, then by updating final artifacts after manual browser, live backend, and forecast/P&L regression verification passed.
- [x] [Review][Patch] Backend Story 6.3 test inventory is stale after the forecast-driven health regression and duplicates `6.3-INT-006` — fixed by assigning unique 6.3-INT-001..010 IDs in code comments and traceability artifacts.
- [x] [Review][Patch] Gate artifacts overclaim complete coverage while live backend, forecast regression, and browser verification remain pending — fixed by separating static/logic coverage from runtime verification, then updating final gate posture to `PASS_WITH_ADVISORY` after required verification passed.
- [x] [Review][Patch] Failed live backend execution is misreported as skipped/PostgreSQL-not-running instead of attempted setup timeout, and the slim gate omits the forecast regression blocker — fixed by documenting the initial setup failure and then replacing blocker language after live regression commands passed.
- [x] [Review][Patch] Traceability report points at an ephemeral `/tmp` coverage matrix that is not part of the generated artifact set — fixed by marking the full coverage matrix as not committed and relying on the committed slim JSON trace artifacts.

## Dev Notes

### Developer Context

- Epic 6 is Dashboard & Reporting. Story 6.3 is the Project Manager deepening story after Story 6.1 established role-specific dashboard cards and Story 6.2 added 30-second polling, manual refresh, request ordering, and changed-value highlighting. [Source: `_bmad-output/planning-artifacts/epics.md#Epic-6-Dashboard--Reporting`; `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md`; `_bmad-output/implementation-artifacts/6-2-real-time-dashboard-updates.md`]
- Story 6.3 acceptance criteria require Project Manager project cards with name, budget status, current margin, forecast margin, red over-budget warnings, orange/red below-target margin, P&L click-through, and sorting by margin, budget utilization, and end date. [Source: `_bmad-output/planning-artifacts/epics.md#Story-6.3-Project-Health-Dashboard`]
- PRD FR48 requires Project Managers to view project health dashboard data covering budget, P&L, and margin. FR50 and FR51 require 30-second polling and manual refresh, already implemented by Story 6.2 and must not regress. [Source: `_bmad-output/planning-artifacts/prd.md#7-Dashboard--Reporting`]
- NFR2 targets dashboard load under 1 second after initial data fetch with data retrieval under 500ms; NFR7 targets updates within 30 seconds; NFR21-NFR28 require WCAG 2.1 AA, keyboard access, chart/table alternatives, ARIA labels, touch targets, and screen reader compatibility. [Source: `_bmad-output/planning-artifacts/prd.md#Non-Functional-Requirements`]
- UX direction is "Stripe Financial": clear financial cards and tables, trustworthy status colors, drill-downs, transparent calculations, and desktop-first operational density. Keep the result utilitarian and scan-friendly. [Source: `_bmad-output/planning-artifacts/ux-design-specification.md#Chosen-Direction`; `_bmad-output/planning-artifacts/ux-design-specification.md#Implementation-Approach`]

### Current File State To Preserve

- `src/backend/src/services/dashboard_service.rs`: `ProjectManagerDashboard` currently returns bounded `active_projects`, `margin_alerts`, and warnings. `build_project_manager_dashboard(...)` scopes by `projects.project_manager_id`, active status, and non-ended projects, then calls `get_project_pl_dashboard(...)` for each card. Preserve this access model and bounded list.
- `src/backend/src/services/project_pl_service.rs`: canonical P&L and forecast formulas already exist. `get_project_pl_dashboard(...)` computes current yearly revenue/cost/profit/margin and `margin_alert`; `get_project_pl_forecast(...)` computes projected total cost, forecast margin, variance from target, category overruns, and resource drivers. Reuse these functions.
- `src/backend/src/routes/project.rs`: existing endpoints include `/api/v1/projects/:id/pl` and `/api/v1/projects/:id/pl/forecast`, with project access enforcement before service calls. Preserve those contracts.
- `src/frontend/src/pages/dashboard.rs`: `Dashboard` already fetches only `/api/v1/dashboard`, handles session expiry, uses one cleanup-safe polling interval, protects request ordering, keeps prior data visible during refresh, and flashes changed values via `dashboard_value_map(...)`. Do not replace this with `LocalResource` unless it clearly simplifies without losing those protections.
- `src/frontend/src/pages/dashboard.rs::ProjectManagerPanel`: currently renders Active Projects, Margin Alerts, and a simple Project Health card grid. This is the primary frontend target for Story 6.3.
- `src/frontend/src/pages/projects.rs`: already has `handle_view_pnl`, `pnl_project_id`, `fetch_pl_dashboard(...)`, `fetch_pl_forecast(...)`, a Forecast toggle, P&L settings, and year controls. Add deep-link entry behavior instead of creating a duplicate P&L viewer in the dashboard.
- `src/frontend/style/tailwind.css`: existing Huly utilities include `panel`, `toolbar`, `stat-card`, `badge-positive`, `badge-warning`, `badge-negative`, `badge-neutral`, `alert-error`, `empty-state`, and `dashboard-change-flash`. Prefer these before adding new utilities.

### Technical Requirements

- Backend remains authoritative for access and role scope. The frontend renders the returned PM dashboard section; it must not call forbidden domain endpoints based only on role strings.
- Project Manager scope must be relationship-based through `projects.project_manager_id`, not merely JWT role claims. [Source: `_bmad-output/project-context.md#Security-Rules`]
- Budget status should be derived from existing dashboard budget numbers:
  - `unconfigured` when `total_budget_idr <= 0`
  - healthy when utilization is below 50%
  - warning when utilization is 50% to below 80%
  - critical when utilization is 80% or higher, and always critical when over budget
  This preserves Story 6.1 dashboard service behavior while making over-budget explicit.
- Current margin and forecast margin must be compared to `target_margin_pct` and `margin_alert_threshold_pct`. Do not invent a second business threshold in the frontend.
- Use integer IDR for money and formatted display with existing `format_idr(...)`. Avoid float currency math.
- Keep visible lists bounded. Story 6.3 must not turn `/dashboard` into a full project analytics page.
- Do not expose CTC components, encrypted CTC values, key metadata, raw audit payloads, or export bytes. PM dashboard data is project/budget/P&L summary data only.
- Empty/no-data is valid. If a PM has no active projects, keep the existing empty state.
- If one optional card has unavailable forecast/P&L data, return a safe warning and do not blank the whole role dashboard.

### Architecture Compliance

- Keep Axum route handlers thin. Story 6.3 should primarily alter service DTOs and frontend rendering; no new backend route is expected.
- If a query parameter is added for `/projects` deep-linking, it is a frontend route concern. Backend project detail endpoints already use `Path(project_id)` and typed `Query(...)` for P&L year/as-of filters.
- Use parameterized SQL and `.bind()` for any new backend query. Do not concatenate user-controlled values into SQL.
- Keep shared calculations in services. If additional shared helper code is needed, extract it deliberately rather than copying logic across routes.
- Preserve existing route behavior for `/dashboard`, `/projects`, `/projects/:id/pl`, `/projects/:id/pl/forecast`, `/team`, `/ctc/*`, `/cash-flow/*`, and `/audit-logs/*`.

### Library / Framework Requirements

- Local manifests/lockfiles are source of truth: backend uses Axum `0.7.9`, sqlx `0.7.4`; frontend uses Leptos `0.8.17`, `leptos_router` `0.8.12`, `gloo-timers` `0.3.0`, and Tailwind CSS `4.1.18`. Do not upgrade dependencies for this story. [Source: `Cargo.lock`; `src/backend/Cargo.toml`; `src/frontend/Cargo.toml`; `src/frontend/package-lock.json`]
- The older planning/project-context text mentions Leptos 0.6; that is stale for the current repo. Use current manifests and existing Leptos 0.8 patterns.
- Official Leptos docs for the current 0.8 line show `LocalResource::new`, `refetch`, and reactive reads, but the current dashboard already uses explicit polling/request-ordering state. Keep the existing approach unless a refactor preserves cleanup, stale-response protection, and visible-data continuity. [Source: `https://docs.rs/leptos/latest/leptos/prelude/struct.LocalResource.html`]
- Official Leptos effect docs describe effects as side effects that rerun when reactive values read inside them change; avoid interval or query-param effects that can stack or repeatedly reopen panels. [Source: `https://docs.rs/leptos/latest/leptos/reactive/effect/index.html`]
- Official Axum docs describe `Query<T>` as deserializing query strings into a `serde::Deserialize` type and rejecting invalid parses with `400`. Existing project routes already follow typed query DTOs for P&L. [Source: `https://docs.rs/axum/latest/axum/extract/struct.Query.html`]
- Official gloo-timers docs cover browser timer APIs for `setTimeout` and `setInterval`; Story 6.2 established the cleanup-safe dashboard polling pattern for this repo. Story 6.3 should not alter that polling lifecycle. [Source: `https://docs.rs/gloo-timers/latest/gloo_timers/`; `_bmad-output/implementation-artifacts/6-2-real-time-dashboard-updates.md#Completion-Notes-List`]

### File Structure Requirements

- Primary backend target:
  - `src/backend/src/services/dashboard_service.rs`
- Primary frontend targets:
  - `src/frontend/src/pages/dashboard.rs`
  - `src/frontend/src/pages/projects.rs`
- Primary test targets:
  - `src/backend/tests/dashboard_tests.rs`
  - `src/backend/tests/project_pl_forecast_tests.rs` if forecast behavior is touched or needs regression confidence
  - `src/frontend/src/pages/dashboard.rs` native test module for pure sorting/change-map helpers, if practical
- Possible generated target:
  - `src/frontend/public/output.css` only if Tailwind output changes after `npm --prefix src/frontend run build`
- Avoid changing:
  - Database migrations. No schema change is expected.
  - CTC, team, cash-flow, audit-report, or revenue mutation routes unless a real regression is discovered.
  - Login routing. Story 6.1 already routes supported roles to `/dashboard`.

### Testing Requirements

- Backend dashboard regression is mandatory: `cargo test --package xynergy-backend --test dashboard_tests`.
- Forecast regression is mandatory if forecast fields are added to dashboard cards: `cargo test --package xynergy-backend --test project_pl_forecast_tests`.
- Frontend compile is mandatory: `cargo check -p xynergy-frontend --target wasm32-unknown-unknown`.
- Frontend native tests should be run when pure helper tests are added: `cargo test -p xynergy-frontend --lib`.
- Tailwind regeneration is mandatory if style utilities or output classes change: `npm --prefix src/frontend run build`.
- Manual/browser verification should cover dashboard card rendering, sort controls, P&L deep-link behavior, over-budget visual warning, below-target margin styling, polling refresh, and changed-value flash.

### Previous Story Intelligence

- Story 6.1 created the backend dashboard service and PM project health cards, then review patches fixed PM cards missing budget status/amounts and excluding non-active/non-owned projects. Do not regress these fixes. [Source: `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md#Review-Findings`]
- Story 6.1 deliberately left full Story 6.3 sorting/detail UX out of scope. This story should complete that PM-specific depth without expanding into Story 6.4 or Story 6.5. [Source: `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md#Scope-Boundary`]
- Story 6.2 added cleanup-safe 30-second polling, request ordering, session-expiry invalidation, generated timestamp display, and value-level highlights. New PM fields must be included in `dashboard_value_map(...)` so polling updates do not silently change visible financial values. [Source: `_bmad-output/implementation-artifacts/6-2-real-time-dashboard-updates.md#Completion-Notes-List`]
- Story 6.2 review fixed stale request invalidation and missing project-health highlight coverage. Keep request id/liveness guards and highlight timeout cleanup intact. [Source: `_bmad-output/implementation-artifacts/6-2-real-time-dashboard-updates.md#Review-Findings`]
- `src/frontend/src/pages/projects.rs` currently stores a polling `Interval` for the forecast panel using `StoredValue::new_local(Some(interval))` inside an effect. If Story 6.3 touches that code, check for repeated effect runs and avoid stacking intervals. Prefer owner-scoped handles and cleanup patterns from `dashboard.rs` if adjustments are needed.

### Git Intelligence Summary

- Recent commits are vertical story slices with story artifact, implementation, focused tests, and review patches:
  - `47b15cd feat: implement Story 6.2 - Real-Time Dashboard Updates...`
  - `2ab8ce7 feat: implement Story 6.1 - Role-Based Dashboard...`
  - `d89082b feat: implement Story 5.4 - Compliance Audit Reports...`
- `src/frontend/src/pages/dashboard.rs` changed heavily in Story 6.2; read current code before editing and do not overwrite request-ordering or polling changes.
- The working tree was clean before this story creation.

### Implementation Pitfalls To Avoid

- Do not build a second project-health API endpoint when `/api/v1/dashboard` already owns role dashboard composition.
- Do not recalculate current or forecast margin in frontend.
- Do not fetch every project's details from the frontend dashboard; backend role scope and card composition already exist.
- Do not use unbounded project fan-out for forecasts.
- Do not let a single forecast failure blank the PM dashboard.
- Do not color every low/zero-revenue project red unless the canonical margin/forecast data indicates risk; handle no-data/unconfigured states separately.
- Do not break Story 6.2 polling cleanup or changed-value flash by refactoring the dashboard component casually.
- Do not add a P&L modal inside `dashboard.rs`; deep-link to the existing Projects P&L flow.
- Do not mark this story complete until sorting, over-budget status, margin coloring, P&L click-through, polling refresh, and regression tests are actually verified.

### References

1. `_bmad-output/planning-artifacts/epics.md` - Epic 6 and Story 6.3 acceptance criteria.
2. `_bmad-output/planning-artifacts/prd.md` - FR45-FR51 and NFR2/NFR7/NFR21-NFR28 dashboard requirements.
3. `_bmad-output/planning-artifacts/architecture.md` - REST/Axum patterns, P&L API direction, Rust/Leptos/Axum/PostgreSQL constraints.
4. `_bmad-output/planning-artifacts/ux-design-specification.md` - Stripe Financial direction, dashboard/P&L layout guidance, accessibility expectations.
5. `_bmad-output/project-context.md` - project guardrails for Rust, Leptos, auth, BigDecimal, date handling, scoping, and testing.
6. `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md` - dashboard contract, PM card foundation, review fixes.
7. `_bmad-output/implementation-artifacts/6-2-real-time-dashboard-updates.md` - polling, refresh, highlight, and cleanup patterns to preserve.
8. `src/backend/src/services/dashboard_service.rs` - PM dashboard service and response contract.
9. `src/backend/src/services/project_pl_service.rs` - canonical current P&L and forecast calculations.
10. `src/backend/src/routes/project.rs` - existing P&L and forecast endpoints plus access checks.
11. `src/frontend/src/pages/dashboard.rs` - PM dashboard rendering, polling, and change detection.
12. `src/frontend/src/pages/projects.rs` - existing Project P&L and forecast detail UI to deep-link into.
13. `src/backend/tests/dashboard_tests.rs` - dashboard role/scoping regression coverage.
14. `src/backend/tests/project_pl_forecast_tests.rs` - forecast math and access regression coverage.
15. `https://docs.rs/leptos/latest/leptos/prelude/struct.LocalResource.html` - current Leptos 0.8 `LocalResource` behavior.
16. `https://docs.rs/leptos/latest/leptos/reactive/effect/index.html` - Leptos effect behavior.
17. `https://docs.rs/axum/latest/axum/extract/struct.Query.html` - typed query extractor behavior.
18. `https://docs.rs/gloo-timers/latest/gloo_timers/` - browser timer APIs used by the existing polling implementation.

## Story Completion Status

- Status: done
- Completion note: Implementation, code-review patches, generated-artifact alignment, live backend regression execution, forecast/P&L regression execution, and manual browser verification are complete. PM dashboard ships forecast/budget overrun/health-status fields, sortable cards, deep-link to the existing P&L view, refreshed change-detection coverage, and verified browser polling/refresh behavior. Remaining advisory is future repeatable browser/component automation and a deterministic forecast-service failure injection seam.

## Dev Agent Record

### Agent Model Used

claude-opus-4-7

### Debug Log References

- `SQLX_OFFLINE=true cargo check -p xynergy-backend` → finished clean (1 future-incompat warning from `sqlx-postgres 0.7.4`, unchanged from prior stories).
- `SQLX_OFFLINE=true cargo check -p xynergy-frontend --target wasm32-unknown-unknown` → finished clean except for pre-existing dashboard/team dead-code warnings.
- `SQLX_OFFLINE=true cargo test -p xynergy-frontend --lib` → 51 tests passed (34 pre-existing + 17 Story 6.3 expansion tests covering PM sort modes, visual-state helpers, forecast/over-budget change keys, and PM sensitive-field exclusion).
- `SQLX_OFFLINE=true cargo test -p xynergy-backend --lib` → 60 tests passed (no regressions).
- `SQLX_OFFLINE=true cargo test -p xynergy-backend --test dashboard_tests --no-run` → test binary compiled.
- `cargo test -p xynergy-backend --test dashboard_tests` → initial live execution failed during sqlx setup with `PoolTimedOut`; after starting local Postgres and applying migrations, `DATABASE_URL=postgres://xynergy:xynergy@localhost:5432/xynergy cargo test -p xynergy-backend --test dashboard_tests` passed 37/37.
- `DATABASE_URL=postgres://xynergy:xynergy@localhost:5432/xynergy cargo test -p xynergy-backend --test project_pl_forecast_tests` → 11/11 passed.
- `DATABASE_URL=postgres://xynergy:xynergy@localhost:5432/xynergy cargo test -p xynergy-backend --test project_pl_tests` → 10/10 passed.
- `npm --prefix src/frontend run build` → Tailwind output regenerated (≈42ms, no errors).
- `./build-frontend.sh` → regenerated Tailwind CSS and current WASM bundle for browser verification (9 pre-existing frontend dead-code warnings, no errors).
- Manual browser verification on `http://127.0.0.1:3000/dashboard` → PM dashboard cards, over-budget warning label, sort controls, P&L deep-link, P&L close-query cleanup, manual Refresh, and 30-second polling flash verified.

### Completion Notes List

- Extended backend `ProjectHealthCard` with `budget_utilization_pct`, `is_over_budget`, `budget_overrun_idr`, `projected_total_cost_idr`, `forecast_margin_pct`, `forecast_variance_from_target_pct`, `forecast_has_revenue_signal`, and `health_status`. All canonical numbers come from `get_project_pl_dashboard` and `get_project_pl_forecast`; no formulas are duplicated in `dashboard_service.rs`.
- `build_project_manager_dashboard` now fans out to both services per-project with `tokio::join!`, bounded by `PM_ACTIVE_PROJECT_LIMIT`. P&L failures still emit a safe placeholder card with a bounded warning. Forecast failures degrade the single card (zeroed forecast fields + per-card warning) instead of blanking the PM dashboard.
- New backend helpers: `safe_unavailable_card`, `compute_budget_utilization_pct`, and `derive_health_status`. Health severity is derived deterministically from P&L + forecast signals and the existing budget thresholds — no second business threshold.
- Frontend `ProjectHealthCard` DTO updated to deserialize the new fields with `serde(default)` for forward compatibility. `dashboard_value_map` now emits stable change-detection keys for every new visible value; PM cards never surface CTC components, ciphertext, key metadata, or audit payload fields (verified by new test `pm_value_map_excludes_sensitive_keys_for_pm_section`).
- `ProjectManagerPanel` rewritten as scannable cards: header with optional warning SVG + health badge, end date + status, budget summary (utilization %, spent + remaining/overrun), revenue/cost, colored current margin vs target, colored forecast vs target, and a per-card "View P&L →" button. Cards remain compact and stable; existing changed-value flash bindings cover all new values.
- Added `PmSortMode` (Margin / Budget Utilization / End Date) with `sort_pm_cards` helper. Toolbar uses a `role="group"` button group with `aria-pressed` and a clear selected state, sorting purely client-side from `active_projects`. Sort changes never refetch.
- Added `/projects?view=pnl&project_id=<uuid>` deep-link behavior in `pages/projects.rs`. The page reacts to `use_query_map`, validates the UUID, sets `pnl_project_id`/`pnl_year`, and reuses the existing P&L panel. The close action clears stale query params by navigating back to `/projects`.
- Backend test suite gained 6 Story 6.3 scenarios (PM forecast/overrun fields present, over-budget marks critical, below-target margin still surfaces margin_alert, forecast variance below target drives critical health, active cards remain bounded to PM_ACTIVE_PROJECT_LIMIT, scope unchanged for non-owned/completed/ended projects).
- Frontend test suite gained 8 new tests (margin/budget-utilization/end-date sort orderings + name tiebreaker, forecast_margin_pct change key, over-budget transition keys, PM sensitive-field exclusion).
- No new migration, no new endpoint family, no new dependency; existing `tokio`, `futures`, `chrono`, `leptos_router`, and design tokens carry the implementation.
- Code review patches added focused coverage/fixes for forecast-unavailable health, zero-threshold forecast styling, router-aware P&L close navigation, and forecast-driven PM health regression coverage.
- Generated automation and traceability artifacts now report Story 6.3 as verified with `PASS_WITH_ADVISORY` gate posture. Manual browser verification, live `dashboard_tests`, and forecast/P&L regression execution are green; remaining advisory is the lack of repeatable browser/component automation and the missing forecast-service error injection seam.

### File List

- src/backend/src/services/dashboard_service.rs
- src/backend/tests/dashboard_tests.rs
- src/frontend/src/pages/dashboard.rs
- src/frontend/src/pages/projects.rs
- src/frontend/public/output.css
- _bmad-output/test-artifacts/automation-summary.md
- _bmad-output/test-artifacts/traceability/6-3-e2e-trace-summary.json
- _bmad-output/test-artifacts/traceability/6-3-gate-decision.json
- _bmad-output/test-artifacts/traceability/6-3-project-health-dashboard-traceability.md

## Change Log

| Date       | Author | Change |
|------------|--------|--------|
| 2026-05-22 | Putu   | Created Story 6.3 ready-for-dev context for the Project Health Dashboard. |
| 2026-05-22 | Putu   | Implemented Story 6.3: backend PM card now includes forecast + over-budget + health-status fields; frontend renders scannable PM cards with severity colors, over-budget warning icon, sortable toolbar (Margin / Budget Utilization / End Date), and P&L deep-link via `/projects?view=pnl&project_id=<uuid>`. Added backend integration tests and 8 frontend native tests. Story moved in-progress -> review. |
| 2026-05-22 | Putu   | Applied Story 6.3 code review fixes and aligned generated automation/traceability artifacts. Story stayed open until manual browser verification, live backend integration execution, and forecast/P&L regression execution were completed. |
| 2026-05-22 | Putu   | Completed remaining Story 6.3 verification: live dashboard/P&L backend regression suites passed and manual PM dashboard browser verification completed. Story moved in-progress -> done. |
