# Story 6.5: CTC Completeness Dashboard

Status: done

<!-- Ultimate context engine analysis completed - comprehensive developer guide created. -->

## Story

As an **HR Staff member**,
I want **to view CTC completeness status across all employees**,
so that **I can ensure all employees have complete cost data**.

## Acceptance Criteria

1. **Given** I navigate to HR -> CTC Dashboard **when** the page loads **then** I see: Total Employees, With CTC, Missing CTC, Completeness %.
2. **Given** I view completeness by department **when** I expand the breakdown **then** I see: Department name, Employee count, CTC complete count, % complete.
3. **Given** I identify employees missing CTC **when** I click on the count **then** I see a list of employees with "Add CTC" action.
4. **Given** I track completeness over time **when** I view the trend chart **then** I see completeness % by month showing progress toward 100%.

## Scope Boundary

- **In scope**: deepen the existing HR CTC completeness experience using the current `/dashboard` HR panel and `/ctc/completeness` route; surface the four headline metrics, expandable department breakdown, bounded missing-CTC list with `Add CTC` actions, and a month-by-month completeness trend.
- **In scope for backend**: extend the existing CTC completeness service/contract rather than creating a parallel reporting system. Use `ctc_records`, `ctc_revisions`, `resources`, and `departments` only; no schema change is expected.
- **In scope for frontend**: fix the existing `/ctc/completeness` summary parsing mismatch with `CompletenessReport` fields (`total_with_ctc`, `total_missing`, `overall_completion_pct`), align the page with current dashboard/Huly styles, and preserve HR dashboard polling/change-highlight behavior.
- **Not in scope**: new CTC data-entry forms, new chart libraries, SSE/WebSockets, Finance CTC validation changes, Department Head team utilization changes, salary/component exposure, encryption/key metadata exposure, database migrations, or replacing the role-based `/dashboard` route.
- **Hard dependencies**: reuse `GET /api/v1/dashboard`, `GET /api/v1/ctc/completeness`, `GET /api/v1/ctc/completeness/missing`, `src/backend/src/services/ctc_completeness.rs`, `src/frontend/src/pages/dashboard.rs`, and `src/frontend/src/pages/ctc_completeness.rs`. Do not reintroduce generic dashboard fan-out.

## Tasks / Subtasks

- [x] **Task 1: Extend the CTC completeness backend contract** (AC: #1, #2, #4)
  - [x] Update `src/backend/src/services/ctc_completeness.rs` with explicit response fields for monthly trend data, for example `trend: Vec<CompletenessTrendPoint>`.
  - [x] Keep the existing top-level fields (`departments`, `total_employees`, `total_with_ctc`, `total_missing`, `overall_completion_pct`) backward compatible for `HrDashboard`.
  - [x] Derive department rows from employee resources only (`resources.resource_type = 'employee'`) and active CTC records only (`ctc_records.status = 'Active'`), matching existing completeness semantics.
  - [x] Derive monthly trend using existing operational dates. Recommended: month buckets from the earliest relevant CTC/resource date through current month; count employees whose employee/resource existed by month end and whose active CTC was effective by month end. If the implementation cannot reliably infer historical employee population from available data, document the limitation and still compute a monotonic "active CTC effective by month" trend from `ctc_records.effective_date` / `ctc_revisions.effective_date`.
  - [x] Keep trend output bounded. Recommended default: last 12 months, with no more than 24 months exposed unless the route accepts a typed, validated range later.
  - [x] Use parameterized SQL and typed structs. Do not build SQL with string-concatenated department ids or dates.
  - [x] Do not include salary components, daily rates, ciphertext, encrypted payloads, encryption algorithms, key versions, or raw audit payloads in the completeness response.

- [x] **Task 2: Preserve HR access control and route behavior** (AC: #1, #2, #3)
  - [x] Keep `/api/v1/ctc/completeness` accessible to HR and the existing Department Head own-department path unless product explicitly removes that legacy behavior.
  - [x] Keep `/api/v1/ctc/completeness/missing` HR-only. This endpoint returns employees needing CTC action across departments and must not be opened to Finance, PM, or generic Department Head users.
  - [x] If a department filter is present for HR, validate it through typed query extraction and apply it consistently to totals, department rows, missing list, and trend.
  - [x] Preserve current dashboard role isolation: HR dashboard response must populate only `hr`; non-HR roles must not receive HR pending employee lists.
  - [x] Keep sensitive-read audit behavior from existing CTC routes; do not add noisy audit rows for ordinary dashboard reads unless a current route already audits that read.

- [x] **Task 3: Deepen the HR panel in `src/frontend/src/pages/dashboard.rs`** (AC: #1, #3, #4)
  - [x] Extend the frontend HR DTOs to match any added backend trend fields with `#[serde(default)]` where rolling-deploy tolerance is useful.
  - [x] Keep the HR stat cards for CTC completeness, pending updates, and compliance; add only compact dashboard detail needed by Story 6.5.
  - [x] Add stable change-detection keys for any new visible HR values, for example `hr.completeness.total_missing`, `hr.completeness.department.<department_id>.completion_pct`, and `hr.completeness.trend.<month>`.
  - [x] Bind `dashboard-change-flash` to new visible HR metric, department, missing-list, and trend values without layout shift.
  - [x] Preserve Story 6.2 protections: cleanup-safe 30-second polling, manual Refresh through the same fetch path, request ordering/liveness guards, `generated_at` timestamp, and no `generated_at`-only flashes.
  - [x] Keep HR dashboard actions as navigation to existing full pages (`/ctc/completeness` and `/ctc?resource_id=<uuid>`); do not embed a CTC edit form inside `dashboard.rs`.

- [x] **Task 4: Upgrade the existing `/ctc/completeness` page instead of replacing it** (AC: #1, #2, #3, #4)
  - [x] Fix `fetch_completeness(...)` in `src/frontend/src/pages/ctc_completeness.rs` to parse the backend's current top-level names: `total_with_ctc`, `total_missing`, and `overall_completion_pct`. The current code expects `with_ctc`, `missing_ctc`, and `completion_pct`, which can zero out the summary cards.
  - [x] Convert the department breakdown into an expandable/collapsible section while keeping the table-readable values for accessibility.
  - [x] Keep missing-CTC count clickable. When expanded, show employee name, department, and an `Add CTC` action to `/ctc?resource_id=<uuid>`.
  - [x] Add the monthly completeness trend using existing HTML/CSS bars or a compact table. Do not add a charting dependency.
  - [x] Keep the page operational and dense; avoid instructional copy about polling, chart mechanics, implementation details, or how the dashboard works.
  - [x] Use stable dimensions for cards, bars, gauges, rows, and buttons so dynamic polling/refresh states do not shift layout.
  - [x] Review role gates: existing UI allows `finance` but the backend completeness route returns 403 for Finance. Either align the frontend authorization to backend behavior or keep Finance only for compliance sections without calling forbidden completeness endpoints.

- [x] **Task 5: Keep navigation and Add CTC flows consistent** (AC: #3)
  - [x] Preserve `/ctc/completeness` route registration in `src/frontend/src/lib.rs`; do not add a duplicate route for the same concept.
  - [x] If the sidebar label changes from `CTC Status` to `CTC Dashboard`, keep role visibility aligned with the backend permission decision from Task 2.
  - [x] `Add CTC` links must route to the existing `/ctc?resource_id=<uuid>` flow and let the CTC management page own validation, encryption, audit logging, and save behavior.
  - [x] Do not leak cross-department details through malformed resource ids, guessed department ids, or stale filters. Error messages should be bounded and non-revealing.

- [x] **Task 6: Add backend regression coverage** (AC: #1, #2, #3, #4)
  - [x] Extend `src/backend/tests/ctc_validation_tests.rs` or `src/backend/tests/dashboard_tests.rs` with completeness coverage for headline totals and department rows.
  - [x] Add HR trend coverage: multiple employees with/without active CTC across months produce month buckets and correct percentages.
  - [x] Add a missing-employee list test that confirms missing employees are included, employees with active CTC are excluded, and the `Add CTC` identifier is the resource id.
  - [x] Add negative access tests: PM and Finance cannot access completeness; non-HR cannot access `/ctc/completeness/missing`; Department Head remains scoped to own department if legacy DH access is preserved.
  - [x] Add sensitive-field absence assertions for completeness/trend/missing responses: no encrypted fields, key metadata, salary component names, daily rates, or raw audit payloads.
  - [x] Run `cargo test --package xynergy-backend --test ctc_validation_tests`.
  - [x] Run `cargo test --package xynergy-backend --test dashboard_tests`.

- [ ] **Task 7: Add frontend regression coverage and verification** (AC: #1, #2, #3, #4) — automated regression coverage complete; live browser verification remains open and blocks release sign-off.
  - [x] Add or update pure frontend tests in `src/frontend/src/pages/dashboard.rs` for new HR change-detection keys.
  - [x] Add pure helper tests in `src/frontend/src/pages/ctc_completeness.rs` if parsing/trend helpers are extracted.
  - [x] Run `cargo test -p xynergy-frontend --lib` if native frontend tests remain usable.
  - [x] Run `cargo check -p xynergy-frontend --target wasm32-unknown-unknown`.
  - [x] Run `npm --prefix src/frontend run build` if Tailwind source or generated classes change. _(Skipped — only existing Tailwind utilities are composed; no new utility patterns introduced.)_
  - [ ] Manually verify in a browser as HR: _BLOCKING for release sign-off. Non-blocking for the automated AC gate. Verification deferred to deployed environment; current gate status is CONDITIONAL_PASS until this checkbox closes._
    - `/dashboard` shows CTC completeness summary and still polls/refreshes/highlights.
    - `/ctc/completeness` shows Total Employees, With CTC, Missing CTC, Completeness %.
    - Department breakdown expands and displays department name, employee count, CTC complete count, and % complete.
    - Missing CTC count opens a bounded employee list and `Add CTC` navigates to `/ctc?resource_id=<uuid>`.
    - Monthly trend renders for empty, partial, and 100% completeness states without layout overlap.

### Review Findings

- [x] [Review][Patch] Trend month buckets used session-timezone resource dates and treated NULL `resources.created_at` as historical from the beginning of the window — fixed by comparing `created_at` in UTC and treating legacy NULL `created_at` as current-anchor-month only; added a DB regression.
- [x] [Review][Patch] Initial `/ctc/completeness` fetch could overwrite a later department-filter response — fixed by guarding all completeness fetch result application against the currently selected department filter.
- [x] [Review][Patch] Story completion notes described Department Head completeness scoping as `resolve_user_department` only — clarified the relationship-based `departments.head_id` validation used by the route.
- [x] [Review][Patch] Frontend department rows defaulted missing CTC to `0` when `missing_ctc` was absent — fixed parser fallback to derive `total_employees - with_ctc` and added a native regression.
- [x] [Review][Patch] Current-month trend projected through future month-end dates — fixed trend SQL to cap the current bucket at today and added a DB regression for future effective CTC.
- [x] [Review][Patch] HR dashboard trend SQL failure could 500 the whole dashboard — split core completeness from trend loading, kept `/ctc/completeness` strict, and made `/dashboard` degrade only trend with a warning.
- [x] [Review][Patch] `/ctc/completeness` accepted non-finite numeric strings and could render `NaN%`/`inf%` — fixed parser finite checks and added a native regression.
- [x] [Review][Patch] In-flight completeness/missing requests could repopulate stale data after auth/role changes or failed department-filter fetches — fixed token/role/filter guards and clear-on-error state.
- [x] [Review][Patch] HR dashboard pending CTC rows exposed only the list link, not direct `Add CTC` actions — added `/ctc?resource_id=<uuid>` links and a native helper regression.
- [x] [Review][Patch] Trace/gate artifacts were stale after review-rerun tests — refreshed counts and added the new review regression inventory.

## Dev Notes

### Developer Context

- Epic 6 is Dashboard & Reporting. Story 6.5 is the HR deepening story after Story 6.1 created the role-aware `/dashboard`, Story 6.2 added polling/highlighting, Story 6.3 deepened the Project Manager dashboard, and Story 6.4 deepened the Department Head dashboard. [Source: `_bmad-output/planning-artifacts/epics.md#Epic-6-Dashboard--Reporting`; `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md`; `_bmad-output/implementation-artifacts/6-2-real-time-dashboard-updates.md`; `_bmad-output/implementation-artifacts/6-3-project-health-dashboard.md`; `_bmad-output/implementation-artifacts/6-4-team-utilization-dashboard.md`]
- Story 6.5 acceptance criteria require HR to see CTC completeness totals, department breakdown, missing-CTC employee list with `Add CTC`, and a monthly progress trend toward 100%. [Source: `_bmad-output/planning-artifacts/epics.md#Story-6.5-CTC-Completeness-Dashboard`]
- PRD FR46 requires HR Staff to view CTC completeness status across employees. FR50/FR51 and NFR7 require dashboard polling/manual refresh behavior; preserve the existing `/dashboard` refresh infrastructure. [Source: `_bmad-output/planning-artifacts/prd.md#7-Dashboard--Reporting`; `_bmad-output/planning-artifacts/prd.md#Non-Functional-Requirements`]
- The existing HR panel already shows CTC completeness, pending CTC updates, recent CTC changes, and compliance alerts from `GET /api/v1/dashboard`. Story 6.5 should deepen this rather than creating a second HR dashboard API. [Source: `src/backend/src/services/dashboard_service.rs`; `src/frontend/src/pages/dashboard.rs`]
- The existing `/ctc/completeness` page already attempts the dashboard/list/table flow, but its top-level summary parser expects old field names. The backend currently serializes `total_with_ctc`, `total_missing`, and `overall_completion_pct`. Fix this before adding new UI so the headline metrics are truthful. [Source: `src/backend/src/services/ctc_completeness.rs`; `src/frontend/src/pages/ctc_completeness.rs`]
- `src/backend/src/services/ctc_completeness.rs` is the canonical completeness service. It already exposes `DepartmentCompleteness`, `CompletenessReport`, and `MissingCtcEmployee`; extend this service instead of duplicating completeness SQL inside `dashboard_service.rs` or frontend code.
- `ctc_records` uses `resource_id` as primary key and tracks active operational CTC state. `ctc_revisions` is append-only revision history with `revision_number`, `effective_date`, `status`, `changed_by`, and `created_at`. Monthly trend should use these existing facts; do not add a new trend table for this story. [Source: `migrations/20260222134500_audit_exports_and_ctc_records.up.sql`; `migrations/20260222123816_ctc_revisions.up.sql`; `migrations/20260222140000_extend_ctc_records.up.sql`]
- The sidebar already links HR and Department Head users to `/ctc/completeness` under `CTC Status`; decide whether to keep that label or rename it to `CTC Dashboard`, but avoid creating a duplicate navigation target. [Source: `src/frontend/src/components/app_sidebar.rs`]

### Current File State To Preserve

- `src/backend/src/routes/dashboard.rs`: thin Axum route for `GET /api/v1/dashboard`; it resolves auth and role-gated query params before delegating to `build_dashboard(...)`. Do not move role data assembly into the route.
- `src/backend/src/services/dashboard_service.rs`: `HrDashboard` currently contains `completeness`, `pending_updates`, `recent_changes`, `compliance_alerts`, and `warnings`; optional failures degrade through warnings where safe. Preserve this posture.
- `src/backend/src/services/ctc_completeness.rs`: completeness counts are employee-only and active-CTC-only; department rows are ordered by department name.
- `src/backend/src/routes/ctc.rs`: `/ctc/completeness` allows HR and Department Head; Department Head is scoped to their own department. `/ctc/completeness/missing` is HR-only.
- `src/frontend/src/pages/dashboard.rs`: polling uses `StoredValue::new_local(Option::<Interval>::None)`, monotonic request ids, a liveness guard, previous value-map diffing, and `dashboard-change-flash`. Do not break these Story 6.2 safeguards.
- `src/frontend/src/pages/ctc_completeness.rs`: page uses `authenticated_get()`, department filter, missing-list toggle, and existing CTC compliance section. Keep auth/session behavior and replace only stale or incomplete parsing/UI.
- `src/frontend/src/pages/ctc.rs`: existing target for `Add CTC` actions. Do not duplicate CTC calculation, validation, encryption, or audit logic in dashboard code.

### Technical Requirements

- Use integer counts and `f64` percentages only for percentages; money/salary values must not be part of this story's dashboard payload.
- Trend month keys should be stable strings such as `YYYY-MM` and sorted ascending.
- Trend percentages should be rounded consistently with existing dashboard display (one decimal place for user-facing percentages is acceptable).
- Empty data is valid: no employees means `0` totals and `0.0%` completion; do not divide by zero or display NaN.
- Department filters must affect all visible completeness surfaces consistently.
- Missing employee lists must be bounded if rendered on `/dashboard`; the full `/ctc/completeness` page can show the existing endpoint result but should still render efficiently and deterministically.
- Dashboard change detection must include only visible values. Do not serialize and diff full backend payloads if that includes hidden data or timestamp-only changes.
- Keep backend optional-card failure handling strict for security and permissive for non-security dashboard degradation. Authentication, authorization, and validation errors should not be swallowed.

### Architecture Compliance

- Backend route handlers stay thin; business/reporting logic belongs in services.
- Reuse existing REST paths under `/api/v1`. Add a new endpoint only if the existing completeness endpoint cannot safely carry the trend data; if added, keep it under `/api/v1/ctc/completeness/...`.
- Use `#[derive(Debug, Serialize)]` for backend response structs and `#[derive(Debug, Deserialize)]` for query DTOs.
- Use `chrono::NaiveDate` for effective dates and month boundaries.
- Use sqlx with `.bind()` parameters. Do not assemble user-controlled SQL fragments.
- Keep Leptos UI state in signals/effects and authenticated network calls through `authenticated_get()`.
- No new charting or date libraries are needed; render trends with existing HTML/CSS bars and table equivalents.

### Library / Framework Requirements

- Local manifests/lockfile are source of truth: Rust 2021, Axum `0.7` (lockfile `0.7.9` for backend), sqlx `0.7.4`, Leptos `0.8.17`, `leptos_router` `0.8.x`, `gloo-timers` `0.3.0`, and Tailwind CSS `4.1.18`. The older project-context note that says Leptos 0.6 is stale for this repo. [Source: `Cargo.toml`; `src/backend/Cargo.toml`; `src/frontend/Cargo.toml`; `Cargo.lock`; `src/frontend/package.json`]
- Axum `RawQuery` is already used in the dashboard route so non-Department-Head dashboards can ignore malformed Department Head-only range params; do not replace this with a global `Query<T>` extractor that breaks other roles. [Source: `src/backend/src/routes/dashboard.rs`; `https://docs.rs/axum/latest/axum/extract/struct.RawQuery.html`]
- Axum `Query<T>` remains appropriate for typed CTC route filters that every caller of that endpoint must satisfy. [Source: `https://docs.rs/axum/latest/axum/extract/struct.Query.html`]
- Leptos 0.8 effects rerun when reactive values they read change; keep polling and trend/filter effects guarded so changes do not stack intervals or duplicate requests. [Source: `https://docs.rs/leptos/latest/leptos/reactive/effect/index.html`]
- `gloo_timers::callback::Interval` uses RAII cancellation on drop; do not call `forget()` for dashboard polling. [Source: `https://docs.rs/gloo-timers/latest/src/gloo_timers/callback.rs.html`]

### File Structure Requirements

- Primary backend targets:
  - `src/backend/src/services/ctc_completeness.rs`
  - `src/backend/src/routes/ctc.rs`
  - `src/backend/src/services/dashboard_service.rs` only to thread new HR completeness fields into `/api/v1/dashboard`
  - `src/backend/tests/ctc_validation_tests.rs`
  - `src/backend/tests/dashboard_tests.rs`
- Primary frontend targets:
  - `src/frontend/src/pages/dashboard.rs`
  - `src/frontend/src/pages/ctc_completeness.rs`
  - `src/frontend/src/components/app_sidebar.rs` only if navigation label/role visibility is aligned
  - `src/frontend/style/tailwind.css` only if reusable utilities are needed
  - `src/frontend/public/output.css` only after Tailwind build
- Reference files:
  - `src/frontend/src/pages/ctc.rs`
  - `src/frontend/src/auth.rs`
  - `src/backend/src/routes/ctc.rs`
  - `src/backend/src/services/dashboard_service.rs`
  - `src/backend/tests/ctc_validation_tests.rs`
  - `src/backend/tests/dashboard_tests.rs`
- Avoid changing:
  - CTC encryption/key-provider internals
  - Finance CTC validation report behavior
  - Department Head team dashboard code except shared compile fallout
  - Database migrations
  - Login routing, which already lands supported roles on `/dashboard`

### Testing Requirements

- Backend completeness route regression is mandatory: `cargo test --package xynergy-backend --test ctc_validation_tests`.
- Backend role dashboard regression is mandatory: `cargo test --package xynergy-backend --test dashboard_tests`.
- Frontend compile is mandatory: `cargo check -p xynergy-frontend --target wasm32-unknown-unknown`.
- Frontend native tests should run if helper tests are added: `cargo test -p xynergy-frontend --lib`.
- Tailwind regeneration is mandatory if source CSS or generated utility usage changes: `npm --prefix src/frontend run build`.
- Manual browser verification should use an HR account with at least two departments, at least one employee with active CTC, at least one employee missing CTC, and CTC effective dates spanning multiple months.

### Previous Story Intelligence

- Story 6.1 created the role-dashboard contract and bounded HR summary lists. Build on `HrDashboard`; do not replace it with frontend fan-out. [Source: `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md`]
- Story 6.1 review fixed dashboard scoping, optional-card degradation, and sensitive-field leakage. New HR completeness data must preserve those guarantees.
- Story 6.2 added polling, request ordering, liveness guards, `generated_at`, and changed-value highlighting. New HR trend and department values must participate in `dashboard_value_map(...)` without causing timestamp-only flashes or interval leaks. [Source: `_bmad-output/implementation-artifacts/6-2-real-time-dashboard-updates.md`; `src/frontend/src/pages/dashboard.rs`]
- Story 6.3 established the pattern of deep-linking from a dashboard card to the existing full page rather than duplicating detailed workflows. Use `/ctc?resource_id=<uuid>` for `Add CTC`. [Source: `_bmad-output/implementation-artifacts/6-3-project-health-dashboard.md`]
- Story 6.4 review found query parsing, UTC date handling, stale state, duplicate query params, and visible change-map gaps. Apply those lessons to completeness trend filters and Add CTC navigation. [Source: `_bmad-output/implementation-artifacts/6-4-team-utilization-dashboard.md#Review-Findings`]
- Existing `ctc_validation_tests.rs` already covers completeness counts, PM/Finance denial, and missing employee inclusion/exclusion. Extend those tests rather than starting a separate suite unless the dashboard-specific contract makes `dashboard_tests.rs` more appropriate.

### Git Intelligence Summary

- Recent commits are vertical story slices with story artifact, backend service/route changes, frontend page updates, focused integration/native tests, generated Tailwind output when needed, and review patches:
  - `5f3b281 Complete BMad story 6-4-team-utilization-dashboard`
  - `12801ad feat: implement Story 6.3 - Project Health Dashboard...`
  - `47b15cd feat: implement Story 6.2 - Real-Time Dashboard Updates...`
  - `2ab8ce7 feat: implement Story 6.1 - Role-Based Dashboard...`
- The current working tree was clean before this story was created. Keep the Story 6.5 implementation as a focused vertical slice and avoid unrelated refactors.

### Implementation Pitfalls To Avoid

- Do not build a second CTC completeness service in `dashboard_service.rs`.
- Do not leave `/ctc/completeness` summary cards reading stale top-level field names.
- Do not use `ctc_records.updated_at` alone for historical completeness trend unless the accepted limitation is explicit; `effective_date` and revision history are better signals for month buckets.
- Do not infer employee historical population incorrectly without documenting the data limitation.
- Do not expose salary components, daily rates, ciphertext, key versions, encryption algorithms, or audit payload internals.
- Do not open `/ctc/completeness/missing` to Finance or PM just because the UI includes a shared page.
- Do not add a charting dependency for one trend. Use existing bars/tables.
- Do not break dashboard polling, manual refresh, request ordering, or change-highlighting while adding new HR values.
- Do not mark the story complete until the trend, department breakdown, missing-list/Add CTC flow, access controls, and regression tests are verified.

### References

1. `_bmad-output/planning-artifacts/epics.md` - Epic 6 and Story 6.5 acceptance criteria.
2. `_bmad-output/planning-artifacts/prd.md` - FR46, FR50-FR51, NFR2, NFR7, and security requirements.
3. `_bmad-output/planning-artifacts/architecture.md` - Rust/Axum/Leptos/PostgreSQL architecture and CTC data model.
4. `_bmad-output/planning-artifacts/ux-design-specification.md` - Stripe-style operational dashboard/table guidance and 30-second polling MVP.
5. `_bmad-output/project-context.md` - project guardrails, with local manifest versions overriding stale Leptos version notes.
6. `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md` - HR dashboard foundation and scoping/sensitive-field lessons.
7. `_bmad-output/implementation-artifacts/6-2-real-time-dashboard-updates.md` - polling, request ordering, and change-highlight patterns.
8. `_bmad-output/implementation-artifacts/6-3-project-health-dashboard.md` - dashboard-to-full-page deep-link pattern.
9. `_bmad-output/implementation-artifacts/6-4-team-utilization-dashboard.md` - latest dashboard deepening and review lessons.
10. `src/backend/src/services/ctc_completeness.rs` - canonical completeness summaries and missing employees.
11. `src/backend/src/routes/ctc.rs` - completeness routes and HR/DH permission behavior.
12. `src/backend/src/services/dashboard_service.rs` - HR dashboard backend contract.
13. `src/frontend/src/pages/dashboard.rs` - HR dashboard panel, polling, request ordering, and change detection.
14. `src/frontend/src/pages/ctc_completeness.rs` - existing full CTC completeness page to upgrade.
15. `src/frontend/src/components/app_sidebar.rs` - existing `/ctc/completeness` navigation.
16. `src/backend/tests/ctc_validation_tests.rs` - existing completeness route regressions.
17. `src/backend/tests/dashboard_tests.rs` - role dashboard regressions.
18. `https://docs.rs/axum/latest/axum/extract/struct.RawQuery.html` - Axum raw query extractor reference.
19. `https://docs.rs/axum/latest/axum/extract/struct.Query.html` - Axum typed query extractor reference.
20. `https://docs.rs/leptos/latest/leptos/reactive/effect/index.html` - Leptos effect behavior.
21. `https://docs.rs/gloo-timers/latest/src/gloo_timers/callback.rs.html` - `Interval` cleanup/leak semantics.

## Story Completion Status

- Status: review
- Completion note: HR CTC Completeness Dashboard deepening implemented and review-patched. Backend completeness service exposes a bounded monthly trend capped at today's date for the current month; `/dashboard` degrades only the trend on trend-query failure while `/ctc/completeness` remains strict. The HR dashboard panel surfaces department breakdown + trend with change-flash keys and direct `Add CTC` actions, and the full `/ctc/completeness` page parses Story-6.5 field names, gates Finance away from forbidden completeness reads, and renders an expandable department breakdown and monthly trend using existing HTML/CSS bars. Backend and frontend regressions are green; manual browser verification still required against a deployed environment, and gate status is CONDITIONAL_PASS until that walk-through is recorded.

## Dev Agent Record

### Agent Model Used

claude-opus-4-7

### Debug Log References

- `cargo check -p xynergy-backend` — clean (warnings only from `sqlx-postgres 0.7.4`, unchanged).
- `cargo check -p xynergy-backend --tests` — clean (pre-existing unused-import / unused-variable warnings, unchanged).
- `cargo check -p xynergy-frontend --lib` — clean (existing dead-code warnings, unchanged).
- `cargo check -p xynergy-frontend --lib --tests` — clean (existing dead-code warnings, unchanged).
- `cargo check -p xynergy-frontend --target wasm32-unknown-unknown` — clean (existing dead-code warnings, unchanged).
- `cargo fmt --check` — clean.
- `cargo test -p xynergy-frontend --lib` — 93 passed / 0 failed (review-pass total after Story 6.5 parser/change-detection patches).
- `cargo test --package xynergy-backend --lib` — 74 passed / 0 failed (review-pass total; warnings only from pre-existing unused test imports).
- `cargo test --package xynergy-backend --test ctc_validation_tests` — 33 passed / 0 failed (review-pass total after department scoping, filtering, trend, sensitive-field, empty/unknown filter, and NULL `created_at` trend regressions).
- `cargo test --package xynergy-backend --test dashboard_tests` — 58 passed / 0 failed (review-pass total after HR completeness trend and sensitive-field assertions).
- `cargo fmt --check` — clean after review patches.
- `cargo check -p xynergy-backend --tests` — clean (pre-existing warnings only).
- `cargo check -p xynergy-frontend --lib --tests` — clean (pre-existing dead-code warnings only).
- `cargo check -p xynergy-frontend --target wasm32-unknown-unknown` — clean (pre-existing dead-code warnings only).
- `cargo test -p xynergy-frontend --lib` — 96 passed / 0 failed (review-rerun patch total after parser finite/fallback and Add CTC helper regressions).
- `cargo test --package xynergy-backend --lib` — 74 passed / 0 failed (review-rerun patch total; warnings only from pre-existing unused test imports).
- `cargo test --package xynergy-backend --test ctc_validation_tests` — 34 passed / 0 failed (review-rerun patch total after current-month future-effective trend regression).
- `cargo test --package xynergy-backend --test dashboard_tests` — 58 passed / 0 failed.

_Note: these debug log counts include the BMad code-review patch/rerun passes. Tailwind generation was intentionally skipped in this pass because no CSS source changed and the newly used utility classes (`min-w-0`, `truncate`, `block`, `whitespace-nowrap`, existing primary link classes) already exist in `src/frontend/public/output.css`. Browser E2E/manual HR walk-through remains deferred to a deployed environment because this repo has no Playwright/Cypress harness and the story's release gate explicitly tracks that live verification separately._

### Completion Notes List

- **Backend service contract.** `CompletenessReport.trend: Vec<CompletenessTrendPoint>` now ships with `/api/v1/ctc/completeness` and the HR dashboard. The trend defaults to last 12 month buckets and is capped at 24 even if a typed range is added later. SQL uses `UNNEST($2::date[])` joined to `resources` + `ctc_records` so trend rows are derived from existing operational dates (`resources.created_at`, `ctc_records.effective_date`) with no new tables. Resource `created_at` is compared in UTC; legacy NULL `created_at` rows are counted from the current anchor month only so they do not inflate historical buckets. Current-month trend is capped at today's date rather than future month-end. Documented limitation: only currently-Active CTC records and current department assignments are replayed against month buckets — historical CTC revisions and department assignment history are not replayed for this story, matching the "monotonic active CTC effective by month" path called out in the story scope.
- **Sensitive-data hygiene.** Trend / department / missing payloads are integer counts + percentage floats only. Verified via `assert_no_sensitive_fields` traversal in INT tests: no `base_salary`, allowance, BPJS, daily_rate, ciphertext, encryption_version, key_version, encryption_algorithm, encrypted_at, encrypted_components, encrypted_daily_rate, or raw `components` fields anywhere in the dashboard payloads.
- **Access control.** `/ctc/completeness` remains HR + DH (DH scope-locked via `resolve_department_head_completeness_department`, which requires both `users.department_id` and `departments.head_id` to identify the caller's own department; asserted by `completeness_department_head_is_scoped_to_own_department` and `completeness_department_head_must_match_department_head_record`). `/ctc/completeness/missing` remains HR-only; the new `missing_endpoint_is_hr_only` test exercises PM, Finance, and DH for explicit 403. Frontend page now also gates Finance away from the completeness fetch (no forbidden round-trip) while still allowing Finance to run BPJS Compliance against `/ctc/compliance-report`.
- **Dashboard change-flash extensions.** HR change-detection keys: `hr.completeness.total_missing`, `hr.completeness.departments.count`, `hr.completeness.department.{id}.completion_pct|with_ctc|missing_ctc|total_employees`, `hr.completeness.trend.{YYYY-MM}.completion_pct|total_with_ctc|total_employees`. All flash via the existing `dashboard-change-flash` plumbing, ride the existing 30s polling loop, and survive the `generated_at`-only delta guarantee (covered by `hr_generated_at_only_change_does_not_flash_completeness_keys`).
- **Frontend parsing fix.** `parse_completeness_summary` (in `src/frontend/src/pages/ctc_completeness.rs`) now reads the canonical `total_with_ctc`, `total_missing`, `overall_completion_pct`, and `trend` fields and ignores the pre-Story-6.5 names. Covered by `parse_completeness_ignores_pre_story_6_5_field_names`.
- **UI density preserved.** Expandable Department Breakdown + Monthly Completeness Trend collapsibles, stable widths/min-heights on stat cards and bars, no instructional copy. Add CTC links route only to `/ctc?resource_id=<uuid>` when a valid UUID target is present; otherwise the action renders unavailable. No charting dependency added — bars use the existing `progress-track` HTML/CSS pattern shared with Story 6.4.
- **Pending manual verification.** Live browser walk-through (last unchecked sub-bullet of Task 7) is the only step not exercised in this environment; everything else exercised by tests is green. Gate status is `CONDITIONAL_PASS` until the deployed-environment HR browser walk-through is recorded.

### File List

- `src/backend/src/services/ctc_completeness.rs` (modified) — added `CompletenessTrendPoint`, `trend` field on `CompletenessReport`, parameterized monthly-trend SQL, month-end calendar helpers, and unit tests.
- `src/backend/src/services/dashboard_service.rs` (modified) — HR dashboard now loads core completeness strictly and trend inside a recoverable savepoint with a warning fallback.
- `src/backend/src/services/mod.rs` (modified) — re-export `CompletenessTrendPoint` and split completeness trend/core helpers.
- `src/backend/tests/ctc_validation_tests.rs` (modified) — added and expanded Story 6.5 INT coverage, including NULL `resources.created_at` trend handling.
- `src/backend/tests/dashboard_tests.rs` (modified) — added HR completeness trend assertions to `hr_dashboard_returns_hr_section_only`.
- `src/frontend/src/pages/dashboard.rs` (modified) — extended `CompletenessReport` DTO + added `DepartmentCompleteness` / `CompletenessTrendPoint` DTOs; new HR change-detection keys (department + trend + total_missing); compact `HrCompletenessTrend` panel; updated test fixtures and added 4 new native tests.
- `src/frontend/src/pages/ctc_completeness.rs` (rewritten) — `parse_completeness_summary` reads Story-6.5 field names, expandable Department Breakdown and Monthly Trend, Finance gated away from completeness fetch, stable bar/card dimensions, added native parser + helper tests.
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (modified) — status transitions for 6-5-ctc-completeness-dashboard.
- `_bmad-output/implementation-artifacts/6-5-ctc-completeness-dashboard.md` (modified) — task checkboxes, dev agent record, file list, change log, status.

## Change Log

| Date       | Author | Change |
|------------|--------|--------|
| 2026-05-22 | Putu   | Created Story 6.5 ready-for-dev context for the CTC Completeness Dashboard. |
| 2026-05-22 | Putu   | Implemented Story 6.5 — backend trend, HR dashboard change-flash, /ctc/completeness page upgrade, regression tests; status moved to review. |
| 2026-05-22 | Putu   | Applied BMad code-review rerun patches for trend date handling, stale completeness fetch guards, and review artifact clarity; story remains review pending browser walk-through. |
| 2026-05-23 | Putu   | Applied full-diff BMad code-review rerun patches for trend as-of semantics, dashboard trend degradation, frontend stale-state/parser guards, HR Add CTC links, and trace/gate freshness; story remains review pending browser walk-through. |
