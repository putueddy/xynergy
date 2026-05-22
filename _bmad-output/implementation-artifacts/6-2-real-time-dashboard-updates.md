# Story 6.2: Real-Time Dashboard Updates

Status: done

<!-- Ultimate context engine analysis completed - comprehensive developer guide created. -->

## Story

As a **Dashboard User**,
I want **dashboard data to update automatically**,
so that **I always see current information without manual refresh**.

## Acceptance Criteria

1. **Given** I am viewing a dashboard **when** data changes on the server **then** the dashboard updates within 30 seconds (polling interval).
2. **Given** I need immediate data **when** I click the "Refresh" button **then** the dashboard fetches latest data immediately **and** displays a timestamp of last update.
3. **Given** the dashboard is updating **when** new data arrives **then** changed values briefly highlight to indicate update.

## Scope Boundary

- **In scope**: automatic 30-second polling for the existing role-based `/dashboard`, manual refresh through the same fetch path, server-authoritative `generated_at` timestamp display, stale-response protection, cleanup-safe timer lifecycle, visible changed-value highlighting, Tailwind source/output updates if new highlight utility classes are added, and dashboard regression verification.
- **In scope for all supported roles**: HR, Department Head, Project Manager, Finance, and Admin dashboard sections created in Story 6.1 must all keep rendering and refreshing correctly.
- **Not in scope**: SSE/WebSockets, new backend push infrastructure, new dashboard domain cards, deeper Story 6.3/6.4/6.5 dashboard functionality, charting libraries, changing role scoping rules, changing dashboard formulas, or replacing the Story 6.1 backend dashboard contract.
- **Hard dependency**: reuse `GET /api/v1/dashboard` and the typed role-specific response contract from Story 6.1. Do not reintroduce frontend fan-out to `/resources`, `/allocations`, `/projects`, `/audit-logs`, or other domain endpoints for dashboard cards.

## Tasks / Subtasks

- [x] **Task 1: Add cleanup-safe 30-second polling to `Dashboard`** (AC: #1)
  - [x] Update `src/frontend/src/pages/dashboard.rs` to import and use `gloo_timers::callback::Interval` for dashboard polling.
  - [x] Start exactly one dashboard polling interval while the dashboard component is mounted and the user is authenticated.
  - [x] Every interval tick must trigger the same refresh path used by the manual Refresh button.
  - [x] Do not call `Interval::forget()`. Keep the interval handle in a component-owned `StoredValue<Option<Interval>>` or equivalent owner-scoped storage, and drop/replace it on cleanup so polling stops after leaving `/dashboard`.
  - [x] Use `on_cleanup` or an equivalent owner cleanup hook to clear the interval and any pending change-highlight timeout when the component unmounts.
  - [x] Prevent interval stacking. Re-running an effect must not create additional active intervals for the same dashboard instance.
  - [x] Do not poll when the user is unauthenticated or after `SESSION_EXPIRED` handling has logged the user out.

- [x] **Task 2: Harden dashboard refresh state and request ordering** (AC: #1, #2)
  - [x] Keep the current dashboard data visible during background polling refreshes; do not blank the page or show the initial loading panel when data already exists.
  - [x] Track an in-flight request or monotonically increasing request id so a slower older response cannot overwrite a newer response.
  - [x] Manual refresh must fetch immediately even though automatic polling exists.
  - [x] If a manual refresh and polling tick collide, either coalesce the request or ensure only the latest response is applied.
  - [x] Preserve existing `SESSION_EXPIRED` behavior: clear auth state, show the session-expired error, and navigate to `/login`.
  - [x] Preserve existing error behavior for non-auth failures: keep the previous successful data visible when safe, show a bounded error message, and do not clear role widgets unnecessarily.

- [x] **Task 3: Preserve and polish the last-updated timestamp** (AC: #2)
  - [x] Continue displaying `generated_at` from `RoleDashboardResponse`; do not infer "last updated" from the client clock.
  - [x] Update the timestamp after every successful automatic or manual refresh.
  - [x] Keep the Refresh button disabled or visibly busy only while a refresh request is in flight.
  - [x] Add an unobtrusive status signal such as `aria-live="polite"` around last-updated/refresh status so assistive tech receives updates without adding instructional copy to the UI.
  - [x] Keep the existing role display, sign-out flow, unauthenticated redirect, and `/dashboard` landing behavior unchanged.

- [x] **Task 4: Detect changed visible dashboard values** (AC: #3)
  - [x] Add a small, explicit change-detection helper for `RoleDashboardResponse` that compares the previous successful snapshot to the next successful snapshot.
  - [x] Ignore `generated_at` when deciding whether values changed; polling will update timestamps even when business data is unchanged.
  - [x] Prefer stable keys based on backend ids where available. Add hidden id fields to frontend DTOs if needed, for example `resource_id`, `project_id`, `allocation_id`, export request id, or audit event id. Serde will already ignore extra backend fields, so this is a frontend-only DTO alignment task.
  - [x] Track changed keys in a `HashSet<String>` or equivalent, with deterministic key names such as:
    - `hr.completeness.overall_completion_pct`
    - `hr.pending_updates.missing_count`
    - `department_head.budget.utilization_percentage`
    - `department_head.upcoming_assignments.<allocation_id>`
    - `project_manager.project.<project_id>.margin_pct`
    - `finance.cash_position.ending_cumulative_position_idr`
    - `finance.export_requests.pending_count`
    - `admin.total_active_projects`
  - [x] Compare only values that are visible on the dashboard. Do not include encrypted CTC data, raw audit payloads, or hidden sensitive fields in the comparison map.
  - [x] Treat newly visible rows/cards as changed so users can see new assignments, projects, audit alerts, export requests, or CTC changes arrive.
  - [x] Clear the changed-key set after a short timeout, recommended 1.5-2.5 seconds.

- [x] **Task 5: Apply brief changed-value highlighting in the existing Huly UI** (AC: #3)
  - [x] Add a reusable frontend helper such as `changed_class(key, changed_keys)` or pass a `is_changed` callback to role panels.
  - [x] Apply highlighting to visible values/cards/lists where changes matter:
    - HR stat values, recent CTC change rows, pending CTC rows, compliance risk rows.
    - Department Head utilization/budget/overallocation stats, budget status, at-risk rows, upcoming assignment rows.
    - Project Manager active-project stat, margin-alert stat/rows, project health cards.
    - Finance cash/validation/audit/export stats and rows.
    - Admin operational stat cards.
  - [x] Use existing Huly/Tailwind design language. A short background/border flash is enough; do not create a new visual system.
  - [x] If a new utility is needed, add it to `src/frontend/style/tailwind.css`, for example `dashboard-change-flash`, and regenerate `src/frontend/public/output.css` with the existing frontend build script.
  - [x] Highlighting must be brief and non-disruptive: no layout shift, no text resize, no auto-scrolling, and no persistent animation.
  - [x] Do not rely only on color for status-critical information. The highlight is a transient update cue, not the only way to understand the value.

- [x] **Task 6: Preserve backend contract and security regressions** (AC: #1, #2, #3)
  - [x] Avoid backend changes unless a frontend compile/type issue proves they are necessary. The Story 6.1 backend contract already includes `generated_at` and role-specific sections.
  - [x] If backend changes are made, preserve the thin route in `src/backend/src/routes/dashboard.rs` and keep role data assembly in `src/backend/src/services/dashboard_service.rs`.
  - [x] Do not loosen HR, Department Head, Project Manager, Finance, or Admin scoping.
  - [x] Do not add successful dashboard-read audit spam. Continue relying on existing sensitive-route audit behavior.
  - [x] Do not expose salary components, ciphertext, key metadata, raw audit payloads, export bytes, or CTC validation internals in frontend DTOs or change detection.

- [x] **Task 7: Verify polling, refresh, highlight, and regressions** (AC: #1, #2, #3)
  - [x] Run `cargo check -p xynergy-frontend --target wasm32-unknown-unknown`.
  - [x] Run `npm --prefix src/frontend run build` if `src/frontend/style/tailwind.css` or Tailwind classes changed.
  - [x] Run `cargo test --package xynergy-backend --test dashboard_tests`.
  - [x] If frontend-native unit tests are feasible for a pure change-detection helper, add focused tests for "generated_at only does not highlight", "changed scalar highlights", and "new row highlights". If the frontend crate cannot run native tests because of WASM-only dependencies, document that and rely on WASM compile plus manual/browser verification.
  - [x] Manually verify `/dashboard` in a browser if a dev server is available:
    - [x] Initial load shows the role dashboard.
    - [x] Manual Refresh updates `Last updated`.
    - [x] Automatic polling triggers after 30 seconds.
    - [x] Changed values flash briefly.
    - [x] Navigating away from `/dashboard` stops polling.
    - [x] Session expiration still redirects to `/login`.
    - _Completed 2026-05-22 during code review: local server + in-app browser verified initial Admin dashboard load, manual refresh timestamp update, 30s polling timestamp update, temporary API-created user count flash with cleanup, no polling after navigating to `/projects` for 36s, and expired-token redirect to `/login`._

### Review Findings

- [x] [Review][Patch] Auth/session teardown does not invalidate in-flight dashboard requests or clear the previous snapshot [src/frontend/src/pages/dashboard.rs:854] — fixed by invalidating request ids on auth teardown/cleanup, clearing snapshot/highlight state, adding an owner-safe liveness guard, and skipping poll ticks while a request is in flight.
- [x] [Review][Patch] Recent CTC change rows use a non-unique change key for multiple revisions of the same resource [src/frontend/src/pages/dashboard.rs:367] — fixed by keying recent CTC rows with resource id plus revision number and created timestamp, with an equivalent name-based fallback.
- [x] [Review][Patch] Visible HR and Department Head sub-values can update without a change flash [src/frontend/src/pages/dashboard.rs:1272] — fixed by adding missing map keys and flash bindings for HR completeness counts, compliance detail counts, and Department Head committed/total budget text.
- [x] [Review][Patch] Project health cards omit several visible business values from change detection [src/frontend/src/pages/dashboard.rs:536] — fixed by tracking and binding project name, status, end date, budget totals/spend, revenue, cost, warning, and margin-alert values.
- [x] [Review][Patch] Finance validation and audit detail values can update without their own change flash [src/frontend/src/pages/dashboard.rs:636] — fixed by tracking null match-rate transitions, validation counts, and all audit alert sub-counts with matching flash bindings.
- [x] [Review][Decision] Story status is `done` while required manual `/dashboard` browser verification remains unchecked — resolved by performing and recording the manual browser pass on 2026-05-22; Story 6.2 remains `done`.
- [x] [Review][Patch] Gate artifacts overstate AC coverage while admitting deferred WASM-runtime gaps [_bmad-output/test-artifacts/traceability/6-2-e2e-trace-summary.json:80] — clarified coverage as automated test-layer coverage plus completed manual browser verification; remaining advisory is lack of repeatable automated frontend harness.
- [x] [Review][Patch] Traceability maps `generated_at` coverage to the wrong backend test id [_bmad-output/test-artifacts/automation-summary-6-2.md:68] — corrected to `6.1-INT-018`.
- [x] [Review][Patch] Trace/review artifacts contain stale `dashboard.rs` line references after review patches [_bmad-output/test-artifacts/traceability/6-2-real-time-dashboard-updates-traceability.md:99] — refreshed the frontend test inventory and review finding line references.
- [x] [Review][Patch] Story File List omits new automation, traceability, gate, and investigation artifacts [_bmad-output/implementation-artifacts/6-2-real-time-dashboard-updates.md:275] — added the new review/test artifacts to the File List.

## Dev Notes

### Developer Context

- Epic 6 is Dashboard & Reporting. Story 6.1 created the role-aware `/dashboard` surface; Story 6.2 adds the promised polling and changed-value UX on top of that surface. [Source: `_bmad-output/planning-artifacts/epics.md#Epic-6-Dashboard--Reporting`]
- Story 6.2 acceptance criteria are explicitly frontend-observable: poll every 30 seconds, manual refresh immediately, display last update timestamp, and briefly highlight changed values. [Source: `_bmad-output/planning-artifacts/epics.md#Story-6.2-Real-Time-Dashboard-Updates`]
- PRD FR50 requires 30-second polling-based dashboard updates and FR51 requires manual refresh on demand. NFR7 requires dashboard polling updates to display within 30 seconds of server-side data changes. [Source: `_bmad-output/planning-artifacts/prd.md#7-Dashboard--Reporting`; `_bmad-output/planning-artifacts/prd.md#Non-Functional-Requirements`]
- The UX specification explicitly sets 30-second polling as the MVP approach and SSE as post-MVP. Do not implement SSE/WebSockets for this story. [Source: `_bmad-output/planning-artifacts/ux-design-specification.md#Technical-Constraints`]
- Story 6.1 intentionally deferred full 30-second polling and changed-value highlight behavior to Story 6.2. Manual refresh and `generated_at` display already exist in `src/frontend/src/pages/dashboard.rs`. [Source: `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md#Tasks--Subtasks`; `src/frontend/src/pages/dashboard.rs`]
- The existing frontend dashboard currently uses a `refresh_token` signal, an `Effect`, and `spawn_local(fetch_role_dashboard())`. This is a valid starting point, but Story 6.2 must add polling lifecycle management, stale-response protection, and change comparison.
- The backend route `GET /api/v1/dashboard` already exists and delegates to `build_dashboard(&pool, &headers, &claims)`. Treat it as the authoritative source for role data. [Source: `src/backend/src/routes/dashboard.rs`; `src/backend/src/services/dashboard_service.rs`]

### Current File State To Preserve

- `src/frontend/src/pages/dashboard.rs`: typed DTOs mirror the backend dashboard response; `fetch_role_dashboard()` uses `authenticated_get("/api/v1/dashboard")`; `Dashboard` redirects unauthenticated users to `/login`, supports sign out, renders role panels, has a manual Refresh button, and displays `generated_at` as "Last updated".
- `src/frontend/src/auth.rs`: `authenticated_get()` resolves relative URLs, attaches Authorization headers, retries once after token refresh, and returns `SESSION_EXPIRED` when refresh fails. Continue using this helper.
- `src/backend/src/services/dashboard_service.rs`: role sections are scoped server-side and optional-card failures degrade to warnings/defaults where safe. Do not move role scoping to the frontend.
- `src/backend/tests/dashboard_tests.rs`: existing tests cover role section isolation, sensitive field non-exposure, generated_at presence, stale JWT department protection, active project filtering, cash-position windowing, and auth failures. These are high-value regressions for this story.
- `src/frontend/style/tailwind.css`: existing Huly utilities include `card-hover`, `row-highlight`, skeletons, transitions, alert styles, and status colors. Add any highlight utility here rather than hardcoding scattered inline styles.
- `src/frontend/public/output.css`: generated Tailwind output. Only change it by running the existing build script.

### Technical Requirements

- Polling interval must be 30,000 ms.
- Polling must stop when the dashboard component unmounts.
- Polling must not stack multiple intervals after auth state changes, component re-renders, or effect re-runs.
- Manual refresh and interval refresh must use one refresh implementation so they cannot diverge.
- The dashboard should keep the last successful data on screen while refreshing in the background.
- Only apply the latest request result. Protect against slow previous requests overwriting newer data.
- Changed-value comparison must ignore `generated_at`.
- Highlighting must use stable keys and visible business values, not full serialized response equality.
- Changed-key timeout must be cleaned up on unmount. If using `Timeout`, keep its handle alive just like `Interval`.
- Keep all role-specific panels compact and operational. Do not add explanatory in-app text about polling mechanics.

### Architecture Compliance

- This is primarily a frontend story. Backend changes should be unnecessary unless a missing contract field blocks stable highlighting.
- Keep the backend dashboard endpoint as `/api/v1/dashboard`.
- Keep frontend data loading through `authenticated_get()`.
- Do not fetch role-specific domain endpoints from the dashboard page. Story 6.1 removed that pattern because it created forbidden/error states for some roles.
- Do not add new crates or npm dependencies for timers, comparison, animation, or charts. The project already has `gloo-timers`, Leptos signals/effects, and Tailwind utilities.
- If frontend DTOs add id fields, make them match backend JSON fields already emitted by `RoleDashboardResponse`; do not ask the backend to duplicate fields only for UI keys.

### Library / Framework Requirements

- Local lockfile/manifests are source of truth: Leptos resolves to `0.8.17`, `leptos_router` to `0.8.12`, `gloo-timers` to `0.3.0`, `reqwest` to `0.11.27`, `web-sys` to `0.3.85`, and Tailwind CSS to `4.1.18`. [Source: `Cargo.lock`; `src/frontend/Cargo.toml`; `src/frontend/package-lock.json`]
- Official Leptos docs.rs for the current 0.8 line shows `LocalResource` supports `refetch`, but this story does not require a full loading-model rewrite. Use `LocalResource` only if it simplifies request ordering and snapshot handling. [Source: `https://docs.rs/leptos/latest/leptos/prelude/struct.LocalResource.html`]
- Official Leptos effect docs describe effects as side effects that rerun when reactive values they read change. Keep interval creation guarded so effect reruns do not create duplicate intervals. [Source: `https://docs.rs/leptos/latest/leptos/reactive/effect/index.html`]
- Official `gloo_timers::callback::Interval` source documents RAII behavior: dropping an `Interval` clears it, while `forget()` leaks it. Use RAII cleanup. [Source: `https://docs.rs/gloo-timers/latest/src/gloo_timers/callback.rs.html`]
- `gloo-timers` latest is `0.4.0`, but the project is locked to `0.3.0`; do not upgrade it for this story without a separate dependency-change reason. [Source: `https://docs.rs/crate/gloo-timers/latest`; `Cargo.lock`]

### File Structure Requirements

- Primary implementation targets:
  - `src/frontend/src/pages/dashboard.rs`
  - `src/frontend/style/tailwind.css` if adding a highlight utility
  - `src/frontend/public/output.css` if Tailwind is regenerated
- Reference files:
  - `src/frontend/src/pages/cash_flow.rs` for existing dashboard polling/manual-refresh patterns.
  - `src/frontend/src/pages/projects.rs` for existing forecast/P&L 30-second refresh patterns.
  - `src/frontend/src/auth.rs` for authenticated request/session-expiry behavior.
  - `src/backend/src/routes/dashboard.rs` and `src/backend/src/services/dashboard_service.rs` for backend contract/scoping.
  - `src/backend/tests/dashboard_tests.rs` for regression expectations.
- Avoid changing:
  - Domain route files (`ctc.rs`, `team.rs`, `cash_flow.rs`, `project.rs`, `audit_log.rs`) unless a real regression is discovered.
  - Database migrations. No schema change is expected.
  - Login routing. Story 6.1 already routes supported roles to `/dashboard`.

### Testing Requirements

- Frontend compile is mandatory: `cargo check -p xynergy-frontend --target wasm32-unknown-unknown`.
- Tailwind regeneration is mandatory if `src/frontend/style/tailwind.css` or new Tailwind classes are introduced: `npm --prefix src/frontend run build`.
- Backend dashboard regression is mandatory: `cargo test --package xynergy-backend --test dashboard_tests`.
- Add pure helper tests for change detection if practical. At minimum, the helper should be small enough to inspect and reason about directly:
  - initial load produces no changed keys
  - `generated_at`-only update produces no changed keys
  - scalar change produces the expected key
  - newly visible row/card produces the expected key
  - removed row/card does not panic
- Manual/browser verification should cover actual timer behavior because no established frontend E2E harness exists in the repo.

### Previous Story Intelligence

- Story 6.1 built the backend dashboard contract, frontend role panels, manual refresh, generated timestamp display, login redirect to `/dashboard`, and extensive backend regression tests. Build on these files instead of replacing them. [Source: `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md#Completion-Notes-List`]
- Story 6.1 frontend initially hit borrow/move issues when `Option::map` consumed role sections before checking empty state. Be careful when passing `RoleDashboardResponse` and nested structs into helper functions and `view!` closures. Clone deliberately where Leptos ownership requires it. [Source: `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md#Debug-Log-References`]
- Story 6.1 review patches fixed critical scoping and data-contract issues: Department Head dashboard uses current DB department/RLS, PM dashboard excludes non-active/non-owned projects, Finance cash excludes future-dated entries, optional cards degrade safely, and sensitive HR CTC fields are not exposed. Do not regress any of these. [Source: `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md#Review-Findings`]
- Story 5.2 added 30-second polling/manual refresh in `cash_flow.rs`, but the prior story notes caution against duplicate requests and interval leaks. Use it as a pattern source, not as something to copy blindly. [Source: `_bmad-output/implementation-artifacts/5-2-cash-flow-dashboard.md#Testing-Requirements`]
- Story 4.6 and current `projects.rs` use `Interval` plus `StoredValue::new_local(Some(interval))` for 30-second refresh while a panel is visible. For the main dashboard, improve on that by keeping one explicit owner-scoped interval handle and cleanup path. [Source: `src/frontend/src/pages/projects.rs`]

### Git Intelligence Summary

- Recent work lands as vertical slices with story artifact, code, tests, and generated frontend CSS where needed. The latest commit is `2ab8ce7 feat: implement Story 6.1 - Role-Based Dashboard with backend API, frontend integration, comprehensive testing, and review patches`.
- Story 6.2 should be a narrow frontend enhancement plus regression verification. It should not become a broad backend/dashboard refactor.
- The working tree was clean before this story file was created.

### Implementation Pitfalls To Avoid

- Do not poll by recursively spawning async loops without cleanup.
- Do not use `Interval::forget()`.
- Do not create an interval inside an effect that can rerun without replacing/dropping the old interval.
- Do not blank role panels during background refresh.
- Do not let an older, slower request overwrite a newer refresh result.
- Do not highlight every poll because `generated_at` changed.
- Do not compare raw full JSON if that causes timestamp-only highlights or includes hidden fields.
- Do not re-fetch role-specific domain endpoints from the dashboard page.
- Do not add SSE/WebSockets or new dependencies.
- Do not mark this story complete until polling, manual refresh, timestamp, highlight behavior, cleanup, and regression checks are all verified.

### References

1. `_bmad-output/planning-artifacts/epics.md` - Epic 6 and Story 6.2 acceptance criteria.
2. `_bmad-output/planning-artifacts/prd.md` - FR45-FR51 and NFR7 dashboard polling requirements.
3. `_bmad-output/planning-artifacts/architecture.md` - Rust/Leptos/Axum/PostgreSQL constraints, REST endpoint patterns, performance expectations.
4. `_bmad-output/planning-artifacts/ux-design-specification.md` - desktop SPA dashboard, 30-second polling MVP, SSE post-MVP, real-time feedback guidance.
5. `_bmad-output/project-context.md` - project guardrails for Rust, Leptos, frontend structure, testing, and auth helpers.
6. `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md` - direct predecessor implementation notes and review findings.
7. `_bmad-output/implementation-artifacts/5-2-cash-flow-dashboard.md` - prior polling/manual-refresh dashboard intelligence.
8. `src/frontend/src/pages/dashboard.rs` - primary implementation target.
9. `src/frontend/src/auth.rs` - authenticated request and session-expiry behavior.
10. `src/backend/src/services/dashboard_service.rs` - backend dashboard response contract and scoping.
11. `src/backend/tests/dashboard_tests.rs` - regression tests to preserve.
12. `https://docs.rs/leptos/latest/leptos/prelude/struct.LocalResource.html` - Leptos 0.8 resource/refetch docs.
13. `https://docs.rs/leptos/latest/leptos/reactive/effect/index.html` - Leptos effect behavior.
14. `https://docs.rs/gloo-timers/latest/src/gloo_timers/callback.rs.html` - `Interval` cleanup/leak semantics.

## Story Completion Status

- Status: done
- Completion note: Frontend-only real-time polling, request-ordered refresh, and changed-value highlighting added on top of the Story 6.1 dashboard surface. Code review patches resolved request invalidation and missing highlight coverage. Backend contract untouched; backend regression suite (`dashboard_tests`) passes.

## Dev Agent Record

### Agent Model Used

Claude Opus 4.7 via bmad-dev-story workflow.

### Debug Log References

- `cargo check -p xynergy-frontend --target wasm32-unknown-unknown` initially failed with `StoredValue::set_value` trait-bound errors when the variable was annotated as `StoredValue<Option<Interval>>` (defaults to `SyncStorage`). `gloo_timers::callback::Interval` and `Timeout` are `!Send+!Sync`, so the value must live in `LocalStorage`. Fixed by letting `StoredValue::new_local(Option::<Interval>::None)` infer the storage parameter (matches existing pattern in `pages/projects.rs` and `pages/cash_flow.rs`).
- Effect-based initial-load/polling split caused `Effect::new(move |prev| ...)` to fail with `type annotations needed` because `prev` is `Option<ReturnType>`. Replaced with a single effect that tracks initial-load state in a `StoredValue<bool>`, which also collapses two effects into one and keeps interval lifecycle in a single place.

### Completion Notes List

- Single explicit polling lifecycle: one `StoredValue<Option<Interval>, LocalStorage>` is replaced (RAII-dropping the previous handle) whenever auth state flips or the effect reruns, so intervals never stack. `on_cleanup` clears both the interval and the highlight timeout when the dashboard unmounts.
- Manual Refresh and the 30-second polling tick share one `load_dashboard()` closure, so the refresh path cannot diverge between manual and automatic.
- Stale-response protection uses a monotonic `next_request_id` / `latest_request_id` pair in `StoredValue<u64>`. A response is only applied when its captured id equals the most recently started one. This handles manual+poll collisions and slow networks without coalescing requests.
- `loading` is set true while the most recent request is in flight; current data stays on screen during background refresh — the page never blanks. The Refresh button shows "Refreshing…" and is disabled while in-flight.
- Last-updated label continues to render the server's `generated_at`, augmented with " · refreshing…" while a request is in flight. Wrapped in an `aria-live="polite"` region so screen readers receive timestamp updates without a visual instruction line.
- `dashboard_value_map()` produces a flat `HashMap<String, String>` of stable keys → display values for visible business data. `generated_at` is deliberately excluded. Stable IDs from the backend (`resource_id`, `project_id`, `allocation_id`, audit entry id, export request id) are added as `Option<Uuid>` fields on the frontend DTOs; serde happily ignores any other backend fields. The DTOs do not include salary components, ciphertext, key metadata, raw audit payloads, or export bytes.
- Newly visible rows surface as "changed" keys; removed rows simply drop out of the map and never produce a highlight. Verified by native unit tests `initial_load_produces_no_changed_keys`, `generated_at_only_change_produces_no_changed_keys`, `scalar_change_produces_expected_key`, `newly_visible_row_produces_expected_key`, and `removed_row_does_not_panic`.
- Changed keys are exposed through `ChangedKeysCtx` context so every role panel can pick them up without prop drilling. Stat values, list rows, and project cards opt in via `class:dashboard-change-flash=...` with closures that read the signal narrowly (using `.with(|set| set.contains(key))`) so we never clone the whole `HashSet` per render.
- A new `@utility dashboard-change-flash` was added to `src/frontend/style/tailwind.css` with a 1.8s blue-tinted background + inset border fade, plus a `prefers-reduced-motion` variant that holds a steady subtle highlight instead of animating. `npm --prefix src/frontend run build` regenerates `src/frontend/public/output.css` (43ms).
- A single `Timeout` lives in a `StoredValue<Option<Timeout>, LocalStorage>` and is replaced whenever a new set of changed keys arrives. Replacement drops the prior `Timeout` (cancelling it), so the highlight clear timer cannot fire late after navigation or after a rapid follow-up update. Default highlight window: 1800 ms.
- Zero backend changes; Story 6.1 contract preserved. Polling reuses `GET /api/v1/dashboard` via `authenticated_get`, so no new audit spam, scoping changes, or sensitive-data exposure. `SESSION_EXPIRED` path explicitly clears both the polling interval and the highlight timeout before navigating to `/login`.

### Verification

- `cargo check -p xynergy-frontend --target wasm32-unknown-unknown` — clean (only pre-existing unused-field warnings outside `dashboard.rs`).
- `cargo test -p xynergy-frontend --lib` — 5/5 dashboard change-detection tests pass.
- `cargo test --package xynergy-backend --test dashboard_tests` — 27/27 pass; no regressions in role scoping, sensitive-field exclusion, `generated_at` presence, scoping, or auth.
- `npm --prefix src/frontend run build` — Tailwind 4.1.18 compiled successfully; `public/output.css` regenerated.
- Manual `/dashboard` browser verification — completed 2026-05-22. Evidence: initial Admin role dashboard rendered; manual Refresh changed `Last updated` from `2026-05-22 00:04:11` to `2026-05-22 00:04:35`; automatic polling changed `Last updated` from `2026-05-22 00:04:41` to `2026-05-22 00:05:11`; temporary user-count change applied `.dashboard-change-flash` and cleared after the 1.8s window; `/projects` navigation left the dashboard request count unchanged after 36s; expired browser tokens redirected `/dashboard` to `/login`. Screenshot: `/tmp/xynergy-story-6-2-dashboard-verification.png`.
- Note: an unrelated pre-existing failure in `cost_preview_tests` is observable when running the full backend test suite. It is outside Story 6.2's scope (no backend code touched by this story) and was not introduced by this change.

### File List

- Modified: `src/frontend/src/pages/dashboard.rs` — Added polling lifecycle (Interval + on_cleanup), monotonic request ordering, change-detection (`dashboard_value_map`/`compute_changed_keys`), `ChangedKeysCtx` context, `flash_if_changed[_owned]` helpers, `aria-live="polite"` status, `class:dashboard-change-flash` bindings across HR/Department Head/Project Manager/Finance/Admin panels, `Option<Uuid>` id fields on DTOs for stable keys, and native unit tests for change detection.
- Modified: `src/frontend/style/tailwind.css` — Added `@utility dashboard-change-flash` and `@keyframes dashboard-change-flash` with `prefers-reduced-motion` fallback.
- Modified: `src/frontend/public/output.css` — Regenerated by Tailwind to include the new utility.
- Modified: `_bmad-output/implementation-artifacts/sprint-status.yaml` — `6-2-real-time-dashboard-updates`: `ready-for-dev` → `in-progress` → `review` → `done`.
- Modified: `_bmad-output/implementation-artifacts/6-2-real-time-dashboard-updates.md` — Task checkboxes, Dev Agent Record, File List, Change Log, Status.
- Added: `_bmad-output/test-artifacts/automation-summary-6-2.md` — Story 6.2 automation expansion and rerun evidence.
- Added: `_bmad-output/test-artifacts/traceability/6-2-real-time-dashboard-updates-traceability.md` — Traceability report and gate rationale.
- Added: `_bmad-output/test-artifacts/traceability/6-2-e2e-trace-summary.json` — Machine-readable coverage summary.
- Added: `_bmad-output/test-artifacts/traceability/6-2-gate-decision.json` — Machine-readable gate decision.
- Added: `_bmad-output/implementation-artifacts/investigations/bmad-automate-stuck-investigation.md` — Investigation artifact produced during automation review.

## Change Log

| Date       | Author | Change                                                                 |
|------------|--------|------------------------------------------------------------------------|
| 2026-05-21 | Putu   | Created Story 6.2 ready-for-dev context for real-time dashboard updates. |
| 2026-05-21 | Dev    | Implemented 30s polling with cleanup-safe Interval handle, monotonic request ordering, server-authoritative timestamp with aria-live status, change-detection helper, changed-value highlights across all role panels, and Tailwind `dashboard-change-flash` utility. Added native unit tests for change detection. Backend untouched; `dashboard_tests` regression suite passes 27/27. Status: review. |
| 2026-05-22 | Review | Applied code review patches for request invalidation, unique CTC revision keys, and missing highlight coverage across HR, Department Head, Project Manager, and Finance dashboard values. Status: done. |
