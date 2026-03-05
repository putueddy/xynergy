# Story 4.6: Profitability Forecasting

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **Project Manager**,
I want **to forecast project profitability at completion**,
so that **I can take corrective action before it is too late**.

## Acceptance Criteria

1. **Given** I view the P&L dashboard **when** I click "Forecast" **then** the system calculates projected final cost based on current burn rate.
2. **Given** the forecast is generated **when** I review the projection **then** I see Current Spend, Projected Total, Forecast Margin, and Variance from Target.
3. **Given** the forecast shows declining margins **when** I analyze the breakdown **then** I can see which cost categories are over-running and identify opportunities for resource mix adjustments.
4. **Given** I make resource adjustments **when** allocations change **then** the forecast updates automatically with new projections.

## Scope Boundary

- **In scope**: project-level profitability forecasting from existing project budget, expense, allocation-cost, and revenue data; forecast endpoint; forecast view in existing P&L panel; category overrun breakdown; resource-mix driver visibility; integration tests; Epic 4 regression rerun.
- **Not in scope**: ML/AI predictions, portfolio-level forecasting, cash flow forecasting (Epic 5), automated re-assignment engine, ERP forecasting imports, and schedule optimization.
- **Dependencies from prior stories**: Story 4.2 (`project_expenses`), Story 4.3 (`project_cost_service`), Story 4.4 (`project_revenue_service`), Story 4.5 (`project_pl_service`, `/pl` UI and endpoints) must be reused and extended.

## Tasks / Subtasks

- [ ] **Task 1: Add backend forecast computation service and canonical formulas** (AC: #1, #2, #3, #4)
  - [ ] Extend `src/backend/src/services/project_pl_service.rs` with forecast DTOs and service entry point:
    ```rust
    pub async fn get_project_pl_forecast(
        pool: &PgPool,
        project_id: Uuid,
        year: i32,
        as_of: Option<NaiveDate>,
    ) -> Result<ProjectPlForecastResult>
    ```
  - [ ] Reuse existing financial sources; do not duplicate prior-story business logic:
    - [ ] Revenue source: `project_revenue_service::get_revenue_grid(pool, project_id, year)`.
    - [ ] Resource cost source: add reusable helper in `project_cost_service.rs` for date-window clipping instead of duplicating allocation math from `compute_project_resource_costs`.
    - [ ] Expense source: aggregate `project_expenses` by category and date range via SQL.
    - [ ] Budget source: `projects.total_budget_idr`, `budget_hr_idr`, `budget_software_idr`, `budget_hardware_idr`, `budget_overhead_idr`, `target_margin_pct`, `margin_alert_threshold_pct`, `start_date`, `end_date`.
  - [ ] Define forecast baseline window:
    - [ ] `as_of_date = query.as_of.unwrap_or(Utc::now().date_naive())`.
    - [ ] Clamp `as_of_date` into project date range `[start_date, end_date]`.
    - [ ] Use elapsed project duration from `start_date..=as_of_date` and full project duration from `start_date..=end_date`.
  - [ ] Define canonical formulas (document in code comments for maintainability):
    - [ ] `current_spend_idr = current_resource_cost_idr + current_non_resource_cost_idr`.
    - [ ] `burn_rate_idr_per_day = current_spend_idr / max(elapsed_days, 1)`.
    - [ ] `projected_total_cost_idr = round(burn_rate_idr_per_day * total_project_days)`.
    - [ ] `remaining_cost_projection_idr = projected_total_cost_idr - current_spend_idr`.
    - [ ] `current_revenue_idr = sum(revenue months in selected year up to as_of month)`.
    - [ ] `forecast_margin_pct = if current_revenue_idr > 0 { ((current_revenue_idr - projected_total_cost_idr) / current_revenue_idr) * 100 } else { 0.0 }`.
    - [ ] `variance_from_target_pct = forecast_margin_pct - target_margin_pct`.
  - [ ] Compute cost category forecast and overrun signals:
    - [ ] Categories: `hr`, `software`, `hardware`, `overhead`.
    - [ ] Map resource costs into `hr`; map expense categories into their matching buckets.
    - [ ] Forecast each category proportionally to current spend share.
    - [ ] `overrun_amount_idr = max(projected_category_cost_idr - budget_category_idr, 0)`.
    - [ ] Include only positive overruns in summary list.
  - [ ] Compute resource mix opportunity dataset:
    - [ ] Top cost drivers from current resource-cost window (`resource_name`, `total_cost_idr`, `share_pct`).
    - [ ] Return top 5 sorted descending by cost share for UI decision support.

- [ ] **Task 2: Expose forecast API endpoint in project routes** (AC: #1, #2, #3, #4)
  - [ ] Add route in `src/backend/src/routes/project.rs`:
    - [ ] `GET /api/v1/projects/:id/pl/forecast?year=YYYY&as_of=YYYY-MM-DD` (`as_of` optional, default today).
  - [ ] Add query DTO:
    ```rust
    #[derive(Debug, Deserialize)]
    struct ProjectPlForecastQuery {
        #[serde(default = "default_revenue_year")]
        year: i32,
        as_of: Option<chrono::NaiveDate>,
    }
    ```
  - [ ] Reuse `enforce_project_mutation_access(...)` to keep PM/admin access parity with existing `/pl` endpoints.
  - [ ] Keep explicit project existence check (`SELECT id FROM projects WHERE id = $1`) before computing forecast.
  - [ ] Add response DTOs with `#[derive(Debug, Serialize)]`:
    - [ ] `ProjectPlForecastResponse` (headline metrics)
    - [ ] `ForecastCategoryEntry` (budget/current/projected/overrun)
    - [ ] `ForecastResourceDriver` (resource mix insights)

- [ ] **Task 3: Add Forecast UI to existing P&L dashboard** (AC: #1, #2, #3, #4)
  - [ ] Extend `src/frontend/src/pages/projects.rs`:
    - [ ] Add `ProjectPlForecastResponse` DTOs mirroring backend response.
    - [ ] Add `forecast_resource` using `create_resource`, keyed by `(pnl_project_id, pnl_year, forecast_reload_nonce)`.
    - [ ] Add `Forecast` toggle/button in P&L header.
    - [ ] Add forecast cards: Current Spend, Projected Total, Forecast Margin, Variance from Target.
    - [ ] Add cost-category overrun table with clear status badges (`On Track`, `At Risk`, `Overrun`).
    - [ ] Add top resource driver section for resource-mix decision support.
  - [ ] Keep existing Leptos pattern constraints:
    - [ ] Use `create_resource` and reactive signals; do not use imperative `spawn_local` for forecast fetching.
    - [ ] Surface forecast fetch errors through existing `error` signal.
    - [ ] Preserve chart-table parity and keyboard accessibility.
  - [ ] AC#4 auto-update behavior:
    - [ ] Trigger forecast refetch when P&L reload nonce changes.
    - [ ] Add lightweight polling refresh while forecast panel is visible (30-second cadence consistent with dashboard refresh NFRs).

- [ ] **Task 4: Add integration tests for forecast correctness and access control** (AC: #1, #2, #3, #4)
  - [ ] Create `src/backend/tests/project_pl_forecast_tests.rs` (or extend `project_pl_tests.rs` if team prefers single P&L suite).
  - [ ] Required tests:
    - [ ] PM can fetch forecast for owned project.
    - [ ] Admin can fetch forecast for any project.
    - [ ] PM denied on non-owned project (403 + denied-access audit entry).
    - [ ] Non-existent project returns 404.
    - [ ] Burn-rate projection math correctness with deterministic fixture data.
    - [ ] Forecast margin and variance-from-target formulas are correct.
    - [ ] Overrun categories identified only when projected exceeds category budget.
    - [ ] Resource driver list sorted by descending cost share.
    - [ ] Allocation update changes forecast output on subsequent fetch (AC#4).
  - [ ] Use optional `as_of` query parameter in tests to avoid time-based flakiness.

- [ ] **Task 5: Performance, accessibility, and regression guardrails** (AC: #1, #2, #3, #4)
  - [ ] Keep forecast generation within NFR1 envelope (<2 seconds target for project-level yearly horizon).
  - [ ] Avoid N+1 query patterns in forecast assembly.
  - [ ] Ensure Forecast panel supports keyboard navigation and readable semantic labels.
  - [ ] Run Epic 4 regression suites after implementation:
    - [ ] `project_budget_tests.rs`
    - [ ] `project_expense_tests.rs`
    - [ ] `project_resource_cost_tests.rs`
    - [ ] `project_revenue_tests.rs`
    - [ ] `project_pl_tests.rs` (+ new forecast suite)

## Dev Notes

### Developer Context

- Story 4.5 already provides core P&L aggregation and UI shell (`/projects/:id/pl`, settings endpoint, P&L cards, chart, monthly table) and is the required base for Story 4.6.
- Forecasting must be additive to the existing P&L dashboard UX: the user should stay within the same Projects page context.
- Existing project schema already has project timeline (`start_date`, `end_date`) and budget-category fields required for completion projection and overrun analysis.

### Dev Guardrails

**Reuse Before Build**

- Do not re-implement allocation cost math from scratch; extract/reuse `project_cost_service` internals via a date-window helper.
- Reuse existing route auth/access pattern from Story 4.5 (`enforce_project_mutation_access` + explicit project existence checks).
- Keep response and request DTOs separate from DB rows.

**Formula and Data Integrity**

- Keep money values as integer IDR (`i64`/`BIGINT`) in storage and payloads.
- Use deterministic formulas and include explicit divide-by-zero guards.
- Keep `as_of` support in backend query params for deterministic testing.

**Error Handling and Audit**

- No `.unwrap()` in production paths; map all failures to `AppError`.
- Ensure denied access still logs audit entries (pattern already used in current project routes).
- Mutation endpoints require audit trails; this forecast endpoint is read-only.

### Architecture Compliance

- Maintain thin route handlers; all forecast computation belongs in service layer.
- Follow existing Axum route composition in `project_routes()`.
- Keep SQL typed and explicit (`sqlx` with clear column casts where needed).
- Avoid introducing new frontend charting/state libraries for this story.

### Library/Framework Requirements

- Stack remains unchanged: Rust 1.75+, Axum 0.7, sqlx 0.7, Leptos 0.6.
- Follow current Axum extractor patterns (`Query`, `Path`, `HeaderMap`, `State`) and existing rejection handling style.
- Follow current Leptos reactive fetch pattern (`create_resource` + signals + `create_effect` error propagation).

### File Structure Requirements

- Backend:
  - `src/backend/src/services/project_pl_service.rs` (extend with forecast result and computation)
  - `src/backend/src/services/project_cost_service.rs` (add reusable date-window helper)
  - `src/backend/src/routes/project.rs` (add forecast query DTO + endpoint + route registration)
- Frontend:
  - `src/frontend/src/pages/projects.rs` (forecast DTOs, fetch fn, resource, Forecast panel)
  - `src/frontend/src/components/project_list.rs` (no required change unless P&L action text is adjusted)
- Tests:
  - `src/backend/tests/project_pl_forecast_tests.rs` (new) or extension in `src/backend/tests/project_pl_tests.rs`

### Testing Requirements

- Use integration test pattern: `#[sqlx::test(migrations = "../../migrations")]`.
- Use seeded deterministic values for spend/revenue and pass explicit `as_of` in forecast requests.
- Validate formulas with explicit expected values (not only status-code checks).
- Verify AC#4 by mutating allocations between two forecast calls and asserting changed projection.
- Keep all existing Epic 4 suites passing.

### Previous Story Intelligence (4.5)

- Route-contract drift was a real failure mode in 4.5 (`/pl-dashboard` vs `/pl`); forecast route naming must stay aligned across backend handlers, frontend fetch URLs, and tests from first commit.
- 4.5 code review caught six concrete misses (route alignment, margin color logic, hover detail behavior, accessibility, settings validation, fetch error surfacing). Treat these as mandatory check items for forecast UI.
- Runtime incident in 4.5 showed stale backend process can mask route changes and return SPA HTML as 200; verify endpoint with direct API call after route additions.

### Git Intelligence Summary

- Recent implementation pattern is a vertical slice across:
  - `src/backend/src/routes/project.rs`
  - `src/backend/src/services/*`
  - `src/frontend/src/pages/projects.rs`
  - `src/backend/tests/*`
- Keep Story 4.6 in the same slice shape to reduce regression and review friction.

### Latest Technical Information

- Axum 0.7 extractor docs confirm current project pattern is correct for typed `Query` extraction and `Result<Json<T>, JsonRejection>` handling; keep this style for forecast query parsing and validation.
- Leptos resource guidance confirms source-driven `create_resource` is the right approach for reactive refetch and SSR/CSR compatibility; avoid imperative async fetch calls for forecast panel rendering.
- No additional dependency is required for forecast UI; native Leptos + existing Tailwind styles are sufficient.

### Project Context Reference

- Core context: `_bmad-output/project-context.md`
- Planning artifacts:
  - `_bmad-output/planning-artifacts/epics.md`
  - `_bmad-output/planning-artifacts/prd.md`
  - `_bmad-output/planning-artifacts/architecture.md`
  - `_bmad-output/planning-artifacts/ux-design-specification.md`

### References

1. `_bmad-output/planning-artifacts/epics.md` - Story 4.6 narrative and acceptance criteria.
2. `_bmad-output/planning-artifacts/prd.md` - FR27/FR28/FR29 and forecasting business context.
3. `_bmad-output/planning-artifacts/architecture.md` - endpoint shape consistency and service-layer architecture constraints.
4. `_bmad-output/project-context.md` - coding rules, AppError patterns, RBAC and sqlx conventions.
5. `_bmad-output/implementation-artifacts/4-5-pl-dashboard.md` - immediate predecessor context, review findings, and integration constraints.
6. `src/backend/src/services/project_pl_service.rs` - current P&L aggregation and margin alert baseline.
7. `src/backend/src/services/project_cost_service.rs` - canonical allocation-cost computation logic to reuse.
8. `src/backend/src/routes/project.rs` - existing P&L route and access-control implementation pattern.
9. `src/frontend/src/pages/projects.rs` - active P&L dashboard rendering and resource/action fetch pattern.
10. `src/backend/tests/project_pl_tests.rs` - current integration test patterns for P&L auth and formula validation.
11. `migrations/20260130111339_initial_schema.sql` - `projects.start_date` and `projects.end_date` timeline fields.

## Dev Agent Record

### Agent Model Used

openai/gpt-5.3-codex

### Debug Log References

- Workflow source: `_bmad/bmm/workflows/4-implementation/create-story/workflow.yaml`
- Workflow instructions: `_bmad/bmm/workflows/4-implementation/create-story/instructions.xml`
- Validation checklist: `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- Sprint tracking source: `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Completion Notes List

- Story created from explicit user target `4-6` (`4-6-profitability-forecasting`); no auto-discovery required.
- Context synthesized from Epic 4 planning artifacts, architecture constraints, UX direction, project context rules, and Story 4.5 implementation/review learnings.
- Forecast formulas and data contracts were made explicit to prevent ambiguous implementation and mismatched frontend/backend interpretation.
- Endpoint contract includes optional `as_of` date for deterministic testing and safer future maintenance.
- Story status prepared for developer handoff (`ready-for-dev`) with implementation guardrails focused on reuse and regression safety.
- Ultimate context engine analysis completed - comprehensive developer guide created.
- 2026-03-06: Adversarial code review findings were fixed end-to-end (forecast windowing to `as_of`, allocation-driven forecast recompute, frontend auto-refresh/polling, and stronger integration coverage for resource-driver sorting and allocation-change behavior).
- 2026-03-06: Validation completed via `cargo test -p xynergy-backend --test project_pl_forecast_tests`, `cargo test -p xynergy-backend --test project_pl_tests`, `cargo check -p xynergy-backend`, and `cargo check -p xynergy-frontend`; story transitioned `review` -> `done` and sprint tracking synchronized.

### File List

- `src/backend/src/services/project_cost_service.rs`
- `src/backend/src/services/project_pl_service.rs`
- `src/backend/src/routes/project.rs`
- `src/frontend/src/pages/projects.rs`
- `src/backend/tests/project_pl_forecast_tests.rs`
- `_bmad-output/implementation-artifacts/4-6-profitability-forecasting.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
