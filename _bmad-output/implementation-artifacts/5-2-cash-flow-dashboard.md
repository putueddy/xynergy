# Story 5.2: Cash Flow Dashboard

Status: complete

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **Finance Team member**,
I want **to view cash flow by month with in/out breakdown**,
so that **I can monitor liquidity and plan payments**.

## Acceptance Criteria

1. **Given** I navigate to Finance -> Cash Flow Dashboard **when** the page loads **then** I see: Monthly Cash In, Cash Out, Net Cash Flow, Cumulative Position.
2. **Given** I view the cash flow chart **when** I select a date range **then** I see a line chart showing cumulative cash position over time.
3. **Given** I filter by project **when** I select a specific project **then** the dashboard shows only cash flow for that project **and** displays project-level net cash position.
4. **Given** I view cash flow details **when** I expand a month **then** I see all individual entries with drill-down capability.

## Scope Boundary

- **In scope**: finance/admin cash-flow dashboard for monthly cash in/out summaries, net cash flow, cumulative cash position, date-range and project filters, monthly drill-down into existing cash-flow entries, dashboard-specific backend aggregation endpoint(s), frontend charts/tables, integration tests, and Story 5.1 regression rerun.
- **Not in scope**: new cash-flow entry CRUD, payroll/CTC validation reports (Story 5.3), compliance audit reports (Story 5.4), ERP cash automation, export workflows, advanced variance analytics explicitly marked post-MVP in architecture, and role-based executive dashboards from Epic 6.
- **Dependencies from prior stories**: Story 5.1 (`cash_flow_entries`, finance RBAC helper, finance page patterns, integration test helpers) is the direct base and must be reused rather than reimplemented; Story 4.5 P&L dashboard and Story 4.6 forecast establish the canonical dashboard/chart/table/accessibility patterns to mirror.

## Tasks / Subtasks

- [x] **Task 1: Add backend cash-flow dashboard aggregation service and canonical formulas** (AC: #1, #2, #3, #4)
  - [x] Extend `src/backend/src/services/cash_flow_service.rs` or create `src/backend/src/services/cash_flow_dashboard_service.rs` if separation keeps route handlers thinner; prefer extending existing service if logic remains cohesive with Story 5.1.
  - [x] Add service entry point similar to Story 4.5/4.6 aggregation style:
    ```rust
    pub async fn get_cash_flow_dashboard(
        pool: &PgPool,
        filters: CashFlowDashboardFilters,
    ) -> Result<CashFlowDashboardResult>
    ```
  - [x] Define `CashFlowDashboardFilters` with explicit fields:
    - [x] `start_date: NaiveDate`
    - [x] `end_date: NaiveDate`
    - [x] `project_id: Option<Uuid>`
  - [x] Use existing `cash_flow_entries` as the single source of truth; do not duplicate or reinterpret P&L `project_revenues` as cash receipts (FR35 distinction is mandatory).
  - [x] Aggregate monthly data with dense month buckets across the requested range using SQL, including months with zero activity.
  - [x] Compute headline metrics with deterministic formulas:
    - [x] `monthly_cash_in_idr = sum(amount_idr where entry_type = 'cash_in')`
    - [x] `monthly_cash_out_idr = sum(amount_idr where entry_type = 'cash_out')`
    - [x] `monthly_net_cash_flow_idr = monthly_cash_in_idr - monthly_cash_out_idr`
    - [x] `cumulative_position_idr[n] = cumulative_position_idr[n-1] + monthly_net_cash_flow_idr`
    - [x] `total_cash_in_idr = sum(monthly_cash_in_idr)`
    - [x] `total_cash_out_idr = sum(monthly_cash_out_idr)`
    - [x] `net_cash_flow_idr = total_cash_in_idr - total_cash_out_idr`
    - [x] `ending_cumulative_position_idr = last(monthly cumulative value) or 0 when no rows`
  - [x] For drill-down, return per-month entry collections ordered `entry_date DESC, created_at DESC`, reusing the `CashFlowEntryResponse` shape or a dashboard-specific equivalent with the same canonical fields.
  - [x] Validate filter semantics:
    - [x] reject `start_date > end_date`
    - [x] if `project_id` is present, verify project exists via existing helper
    - [x] normalize grouping boundaries to month starts/ends so chart/table values are stable
  - [x] Prefer a single aggregation query plus one details query keyed by month rather than N+1 per month loops.

- [x] **Task 2: Expose finance dashboard API contract in cash-flow routes** (AC: #1, #2, #3, #4)
  - [x] Extend `src/backend/src/routes/cash_flow.rs` with dashboard request/response DTOs.
  - [x] Add query DTO:
    ```rust
    #[derive(Debug, Deserialize)]
    pub struct CashFlowDashboardQuery {
        pub start_date: Option<chrono::NaiveDate>,
        pub end_date: Option<chrono::NaiveDate>,
        pub project_id: Option<Uuid>,
    }
    ```
  - [x] Default date range sensibly for first load; recommended default is current calendar year (`YYYY-01-01` to `YYYY-12-31`) to match existing year-based finance views.
  - [x] Add endpoint under existing finance route module, keeping `/api/v1/cash-flow/...` consistency:
    - [x] `GET /api/v1/cash-flow/dashboard?start_date=YYYY-MM-DD&end_date=YYYY-MM-DD&project_id=<uuid>`
  - [x] Reuse Story 5.1 `enforce_finance_access(...)` for finance/admin authorization.
  - [x] Keep project existence and validation errors explicit and early, mapped to `AppError::Validation` / `AppError::NotFound`.
  - [x] Define response DTOs with explicit `Debug + Serialize` derives, e.g.:
    - [x] `CashFlowDashboardResponse`
    - [x] `CashFlowDashboardMonth`
    - [x] `CashFlowDashboardEntry`
  - [x] Ensure payload supports both chart and table views without additional client-side recomputation beyond rendering.

- [x] **Task 3: Implement Finance -> Cash Flow Dashboard UI with chart/table parity** (AC: #1, #2, #3, #4)
  - [x] Extend `src/frontend/src/pages/cash_flow.rs` instead of creating a second disconnected finance page unless route-level complexity becomes unmanageable; keeping entry + dashboard in one finance surface aligns better with Story 5.1 patterns and UX continuity.
  - [x] Add dashboard state and fetch helpers near existing entry-list fetch logic:
    - [x] typed dashboard DTOs mirroring backend response
    - [x] `start_date`, `end_date`, and `project_id` signals for filters
    - [x] dashboard loading/error state integrated with existing page-level alerts
  - [x] Prefer Leptos `LocalResource`/reactive resource pattern consistent with current project usage; do not build new imperative polling-only fetch loops as the primary data source.
  - [x] Add dashboard KPI cards for:
    - [x] Total Cash In
    - [x] Total Cash Out
    - [x] Net Cash Flow
    - [x] Ending Cumulative Position
  - [x] Render cumulative cash position as a native SVG line chart, following the Story 4.5/4.6 no-new-chart-library pattern already established in `src/frontend/src/pages/projects.rs`.
  - [x] Include monthly in/out/net table directly below the chart to satisfy NFR25 chart-data parity.
  - [x] Add filter controls:
    - [x] start date
    - [x] end date
    - [x] project dropdown including "All projects"
    - [x] manual refresh action
  - [x] Add lightweight polling refresh while the dashboard is visible (30-second cadence) so dashboard behavior stays aligned with dashboard NFR expectations.
  - [x] Add monthly drill-down interaction from dashboard rows/cards/chart focus state into the underlying entry list for that month; acceptable implementations include expandable table rows or a month-detail panel below the summary table.
  - [x] When a project filter is active, surface a project-specific headline net position indicator as required by AC #3.
  - [x] Preserve role guard behavior from Story 5.1 so only finance/admin users can access the page.

- [x] **Task 4: Accessibility, interaction, and dashboard UX guardrails** (AC: #2, #4)
  - [x] Keep chart semantics explicit:
    - [x] `role="img"`
    - [x] meaningful `aria-label`
    - [x] keyboard-focusable data points or equivalent focus targets
  - [x] Provide equivalent tabular monthly data and drill-down entry data directly in the DOM, not chart-only tooltips.
  - [x] Ensure all filter controls and expandable month-detail controls are keyboard reachable and screen-reader labeled.
  - [x] Keep touch targets at least 44x44 where clickable rows/controls are introduced.
  - [x] Use the existing Huly/Tailwind visual language from finance pages and P&L dashboards; avoid introducing a competing design system or generic widget library.

- [x] **Task 5: Add integration tests for dashboard math, filters, and access control** (AC: #1, #2, #3, #4)
  - [x] Create `src/backend/tests/cash_flow_dashboard_tests.rs`.
  - [x] Reuse Story 5.1 finance test helpers and setup patterns where possible.
  - [x] Required tests:
    - [x] finance can fetch dashboard
    - [x] admin can fetch dashboard
    - [x] project_manager/hr/department_head denied with `403`
    - [x] invalid date range returns `400`
    - [x] nonexistent `project_id` returns `404`
    - [x] monthly aggregation returns dense month buckets for selected range
    - [x] `net_cash_flow = cash_in - cash_out`
    - [x] cumulative position rolls forward month-to-month correctly
    - [x] project filter limits both summary and drill-down entries
    - [x] months with no entries return zeros but still appear in range
  - [x] Rerun regression suites most likely to break from route/service reuse:
    - [x] `src/backend/tests/cash_flow_tests.rs`
    - [x] `src/backend/tests/project_pl_tests.rs`
    - [x] `src/backend/tests/project_revenue_tests.rs`

- [x] **Task 6: Performance and correctness guardrails** (AC: #1, #2, #3, #4)
  - [x] Keep dashboard generation within existing dashboard expectations (<2 seconds for typical 12-month range).
  - [x] Avoid N+1 query patterns for per-month drill-down data.
  - [x] Keep all currency values as integer IDR (`i64`/`BIGINT`) end to end.
  - [x] Preserve FR35 terminology in UI copy and comments: this dashboard is about actual cash receipts/payments, not invoiced revenue.

## Dev Notes

### Developer Context

- Story 5.1 already created the authoritative `cash_flow_entries` table, finance/admin access helper, project-linked entry endpoint, and finance page shell. Story 5.2 should layer dashboard aggregation on top of those foundations instead of building parallel cash-flow models. [Source: `_bmad-output/implementation-artifacts/5-1-cash-flow-entry.md`]
- The current finance page `src/frontend/src/pages/cash_flow.rs` already includes role guard logic, project dropdown loading, IDR formatting, and the entry list. The dashboard should reuse this page context or its helper patterns to minimize duplication and regression risk. [Source: `src/frontend/src/pages/cash_flow.rs`]
- Story 4.5 P&L Dashboard and Story 4.6 Profitability Forecasting are the strongest local precedents for KPI cards, reactive dashboard loading, SVG chart rendering, table parity, alert/error surfacing, and backend service-layer aggregation. [Source: `_bmad-output/implementation-artifacts/4-5-pl-dashboard.md`; `_bmad-output/implementation-artifacts/4-6-profitability-forecasting.md`; `src/frontend/src/pages/projects.rs`; `src/backend/src/services/project_pl_service.rs`]

### Technical Requirements

- Cash flow math must use actual `cash_flow_entries` only; never mix `project_revenues` into this dashboard because FR35 explicitly distinguishes invoiced revenue (P&L) from received cash (cash flow). [Source: `_bmad-output/planning-artifacts/prd.md`; `_bmad-output/planning-artifacts/epics.md`]
- Use integer IDR values (`i64`/`BIGINT`) for dashboard totals and monthly values, matching Story 5.1 schema and project-wide finance conventions. [Source: `migrations/20260306120000_add_cash_flow_entries.up.sql`; `_bmad-output/project-context.md`]
- Existing canonical cash-flow values remain unchanged:
  - `entry_type`: `cash_in | cash_out`
  - `cash_in` categories: `client_payment | interest | other_income`
  - `cash_out` categories: `payroll | vendor_payment | expense | tax`
  The dashboard should group and label these values, not reinterpret them. [Source: `src/backend/src/services/cash_flow_service.rs`; `_bmad-output/implementation-artifacts/5-1-cash-flow-entry.md`]
- Date filters must use `chrono::NaiveDate` and reject inverted ranges via `AppError::Validation`. [Source: `_bmad-output/project-context.md`]
- Aggregation should return dense month buckets for stability in charts/tables, following the same deterministic month-grid strategy used by the P&L dashboard service. [Source: `src/backend/src/services/project_pl_service.rs`; `_bmad-output/implementation-artifacts/4-5-pl-dashboard.md`]
- Dashboard behavior should include manual refresh and 30-second polling alignment because dashboard updates are an explicit product expectation, even when the underlying implementation remains simple polling rather than SSE. [Source: `_bmad-output/planning-artifacts/epics.md`; `_bmad-output/planning-artifacts/ux-design-specification.md`]

### Architecture Compliance

- Keep route handlers thin and place dashboard computation in the service layer. This project consistently treats routes as DTO/auth shells and services as business-logic homes. [Source: `_bmad-output/project-context.md`; `_bmad-output/planning-artifacts/architecture.md`]
- Keep cash-flow routes under `/api/v1/cash-flow/...` and continue exporting `cash_flow_routes() -> Router<PgPool>`. [Source: `src/backend/src/routes/cash_flow.rs`; `_bmad-output/project-context.md`]
- Reuse `enforce_finance_access(...)` for finance/admin authorization rather than PM-specific helpers from project routes. [Source: `src/backend/src/routes/cash_flow.rs`; `_bmad-output/implementation-artifacts/5-1-cash-flow-entry.md`]
- Prefer explicit, typed SQL and map all failures to `AppError`; avoid `unwrap()` in production paths. [Source: `_bmad-output/project-context.md`]
- Avoid adding a new charting dependency unless absolutely necessary; the architecture and prior stories already established native SVG chart rendering as the accepted pattern. [Source: `_bmad-output/implementation-artifacts/4-5-pl-dashboard.md`; `_bmad-output/planning-artifacts/architecture.md`]

### Library/Framework Requirements

- Stack remains pinned for this story: Rust 1.75+, Axum 0.7, sqlx 0.7, Leptos 0.6, PostgreSQL 15+. [Source: `_bmad-output/project-context.md`; `_bmad-output/planning-artifacts/architecture.md`]
- Keep current Leptos reactive fetch patterns already present in this repo. The local codebase uses `LocalResource` and signal-driven reload behavior; the dashboard should align with those established patterns. [Source: `src/frontend/src/pages/projects.rs`; `src/frontend/src/pages/cash_flow.rs`]
- External research confirms SVG-based Leptos chart approaches and data-table alternatives are appropriate, but local precedent still favors native SVG over pulling in a new crate for this story. Official Leptos guidance also supports resource-based async loading and `Transition`/`Suspense` for reactive UX. [Source: `https://book.leptos.dev/async/10_resources.html`; librarian research session `bg_2e48c12d`]
- Frontend data access should continue using `authenticated_get`/existing auth helpers rather than raw `reqwest` calls so URL resolution and auth handling stay consistent with current WASM constraints. [Source: `src/frontend/src/pages/cash_flow.rs`; `_bmad-output/project-context.md`]

### File Structure Requirements

- Backend likely touch points:
  - `src/backend/src/routes/cash_flow.rs`
  - `src/backend/src/services/cash_flow_service.rs` or new `src/backend/src/services/cash_flow_dashboard_service.rs`
  - `src/backend/src/services/mod.rs` if a new service file is introduced
  - `src/backend/tests/cash_flow_dashboard_tests.rs`
- Frontend likely touch points:
  - `src/frontend/src/pages/cash_flow.rs`
  - `src/frontend/src/pages/mod.rs` only if component exposure changes
  - potentially `src/frontend/src/components/app_sidebar.rs` only if navigation label changes (not expected)
- Existing files that should stay unchanged unless absolutely necessary:
  - `migrations/20260306120000_add_cash_flow_entries.up.sql`
  - `src/backend/src/routes/project.rs`
  - `src/backend/src/services/project_pl_service.rs`
  These are references/pattern sources, not primary implementation targets for Story 5.2.

### Testing Requirements

- Follow backend integration style already used in Story 5.1 and Story 4.x suites: `#[sqlx::test(migrations = "../../migrations")]`. [Source: `src/backend/tests/cash_flow_tests.rs`; `_bmad-output/implementation-artifacts/4-5-pl-dashboard.md`]
- Cover role access, validation failures, formula correctness, dense range output, and project-filter behavior with explicit JSON assertions. [Source: `src/backend/tests/cash_flow_tests.rs`; `src/backend/tests/project_pl_tests.rs`]
- Verify chart-data parity at the UI level by ensuring every charted monthly value is also present in a table; this is an implementation requirement from NFR25 even if frontend tests are not yet automated. [Source: `_bmad-output/planning-artifacts/epics.md`; `_bmad-output/planning-artifacts/ux-design-specification.md`]
- Re-run Story 5.1 regression suite because route/service reuse makes `cash_flow_tests.rs` the highest-risk collateral area. [Source: `_bmad-output/implementation-artifacts/5-1-cash-flow-entry.md`]
- Verify polling/manual-refresh behavior does not duplicate requests uncontrollably when filters change or the dashboard is hidden. Reuse the guarded refresh pattern already established for forecast refresh behavior where helpful. [Source: `_bmad-output/implementation-artifacts/4-6-profitability-forecasting.md`; `src/frontend/src/pages/projects.rs`]

### Previous Story Intelligence (5.1)

- Story 5.1 intentionally stopped at entry CRUD + retrieval and repeatedly called out that aggregation, net cash flow, and cumulative charts belong in Story 5.2. Treat that separation as a hard guardrail to avoid duplicating entry logic here. [Source: `_bmad-output/implementation-artifacts/5-1-cash-flow-entry.md:31`; `_bmad-output/implementation-artifacts/5-1-cash-flow-entry.md:115`]
- Review findings on Story 5.1 exposed concrete failure modes to avoid now:
  - finance project access path must use endpoints available to finance users
  - checked-off subtasks must match real implementation
  - missing-field / filter coverage must be explicit in tests
  - user-facing rows should display readable names, not raw UUIDs, when names are available
  [Source: `_bmad-output/implementation-artifacts/5-1-cash-flow-entry.md:273`]
- Concretely, load project filter options from `/api/v1/projects` or another finance-accessible endpoint; do not regress to `/api/v1/projects/assignable`, which excluded finance users in Story 5.1 review. [Source: `_bmad-output/implementation-artifacts/5-1-cash-flow-entry.md:286`; `src/frontend/src/pages/cash_flow.rs`]
- Story 5.1 frontend uses `spawn_local` loading helpers, but Story 4.4/4.5 review history found imperative fetch patterns easier to regress. For dashboard state, prefer the stronger reactive resource patterns already proven in `projects.rs` unless there is a compelling reason not to. [Source: `_bmad-output/implementation-artifacts/5-1-cash-flow-entry.md`; `_bmad-output/implementation-artifacts/4-5-pl-dashboard.md`]

### Git Intelligence Summary

- Recent implementation sequence is highly vertical-slice oriented: migration/service/route/frontend/tests together per story. The last five commits show Story 4.5, Story 4.6, and Story 5.1 each landed as cohesive end-to-end slices rather than isolated backend-only or frontend-only work. [Source: `git log -5 --oneline`]
- Relevant recent commit messages:
  - `cc39b62 feat: implement Story 5.1 - Cash Flow Entry with backend API, frontend integration, and database migration`
  - `b991c2a feat: implement Story 4.6 - Profitability Forecasting with project-level cost projections, margin analysis, and resource mix insights`
  - `54a550c feat: implement Story 4.5 - P&L Dashboard with project profitability tracking, margin alerts, and detailed monthly breakdowns`
- For Story 5.2, keeping the same slice shape (`services` + `routes` + `frontend page` + `tests` + story artifact) will fit repo history and reduce review friction.

### Latest Technical Information

- Leptos resource documentation recommends resource-based async loading for reactive dashboards and distinguishes browser-only work from server-safe async tasks; this supports using the project's existing resource pattern for filter-driven dashboard refreshes. [Source: `https://book.leptos.dev/async/10_resources.html`]
- Accessibility guidance for charts continues to emphasize adjacent/equivalent data-table alternatives and semantic labeling, which directly reinforces existing project NFR25/NFR26 requirements rather than suggesting a different UX direction. [Source: `https://www.w3.org/WAI/tutorials/images/complex/`; librarian research session `bg_2e48c12d`]
- External Leptos chart libraries exist (`leptos-chartistry`, `leptos_chart`), but given this repo already ships native SVG charts in `projects.rs`, introducing one here would add dependency and consistency risk without a demonstrated local need. [Source: librarian research session `bg_2e48c12d`; `src/frontend/src/pages/projects.rs`]

### Project Structure Notes

- This story aligns cleanly with the existing unified project structure: backend route files in `src/backend/src/routes`, service files in `src/backend/src/services`, frontend pages in `src/frontend/src/pages`, and integration tests in `src/backend/tests`. [Source: `_bmad-output/project-context.md`]
- There is one local variance to respect: `cash_flow.rs` currently co-locates page concerns for entry form and entry list, so the dashboard may either extend that page or extract internal helper components later, but should not prematurely create a disconnected second route unless required by complexity.

### References

1. `_bmad-output/planning-artifacts/epics.md` - Epic 5 / Story 5.2 narrative and acceptance criteria.
2. `_bmad-output/planning-artifacts/prd.md` - FR30-FR35 cash-flow requirements and P&L vs cash-flow distinction.
3. `_bmad-output/planning-artifacts/architecture.md` - service-layer, dashboard, accessibility, and API design constraints.
4. `_bmad-output/planning-artifacts/ux-design-specification.md` - dashboard/data-table, finance user, drill-down, and accessibility UX requirements.
5. `_bmad-output/project-context.md` - Rust/Axum/Leptos/sqlx guardrails, AppError rules, audit expectations, and file-structure conventions.
6. `_bmad-output/implementation-artifacts/5-1-cash-flow-entry.md` - direct predecessor learnings, scope boundaries, and review findings.
7. `_bmad-output/implementation-artifacts/4-5-pl-dashboard.md` - canonical dashboard/chart/table/accessibility implementation pattern.
8. `_bmad-output/implementation-artifacts/4-6-profitability-forecasting.md` - additive dashboard extension and regression/test expectations.
9. `src/backend/src/routes/cash_flow.rs` - current finance RBAC helper and cash-flow route conventions.
10. `src/backend/src/services/cash_flow_service.rs` - canonical cash-flow validation and project-existence helper.
11. `src/frontend/src/pages/cash_flow.rs` - existing finance page shell, project loading, formatting helpers, and list patterns.
12. `src/frontend/src/pages/projects.rs` - native SVG chart and reactive dashboard patterns already accepted in repo.
13. `src/backend/tests/cash_flow_tests.rs` - Story 5.1 integration-test style and reusable helper patterns.
14. `https://book.leptos.dev/async/10_resources.html` - current Leptos resource guidance.
15. `https://www.w3.org/WAI/tutorials/images/complex/` - chart accessibility guidance supporting data-table alternatives.

## Story Completion Status

- Status: complete
- Completion note: All 6 tasks implemented, review findings remediated, 15 integration tests pass, and all regression suites are green (cash_flow 31/31, project_pl 10/10, project_revenue 18/18).

## Dev Agent Record

### Agent Model Used

openai/gpt-5.4

### Debug Log References

- Workflow source: `_bmad/bmm/workflows/4-implementation/create-story/workflow.yaml`
- Workflow instructions: `_bmad/bmm/workflows/4-implementation/create-story/instructions.xml`
- Validation checklist: `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- Sprint tracking source: `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Completion Notes List

- Story created from explicit user target `5-2`; story key resolved as `5-2-cash-flow-dashboard`.
- Context synthesized from Epic 5 story requirements, PRD FR32-FR35, architecture constraints, UX dashboard guidance, project-context rules, Story 5.1 learnings, Story 4.5/4.6 dashboard patterns, repository code search, and external Leptos/accessibility research.
- Guidance explicitly separates cash-flow dashboard behavior from P&L revenue semantics to prevent FR35 regression.
- Guidance keeps Story 5.2 additive to Story 5.1 and aligned with existing native SVG dashboard implementations already present in the repo.
- Manual checklist validation applied: added polling/manual-refresh guidance, finance-safe project-filter endpoint guidance, stronger frontend auth-helper guidance, and regression notes for guarded refresh behavior.
- Validation workflow task file `_bmad/core/tasks/validate-workflow.xml` may need manual fallback if still absent, as noted in Story 5.1.
- Post-review fixes normalized dashboard month boundaries, moved dashboard loading to `LocalResource` + guarded interval refresh, disabled refresh while loading, and replaced clickable table rows with explicit month-detail buttons.

### File List

- `_bmad-output/implementation-artifacts/5-2-cash-flow-dashboard.md` (story artifact)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (story lifecycle tracking updated to `done`)
- `src/backend/src/services/cash_flow_service.rs` (extended with dashboard aggregation service, types, and formulas)
- `src/backend/src/routes/cash_flow.rs` (added GET /api/v1/cash-flow/dashboard endpoint with DTOs and RBAC)
- `src/frontend/src/pages/cash_flow.rs` (added dashboard UI: KPI cards, SVG line chart, monthly table, filters, accessible drill-down controls, `LocalResource`, and guarded polling)
- `src/backend/tests/cash_flow_dashboard_tests.rs` (new: 15 integration tests)
