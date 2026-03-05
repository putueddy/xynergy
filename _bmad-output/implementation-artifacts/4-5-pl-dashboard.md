# Story 4.5: P&L Dashboard

Status: done

<!-- Validated via validate-create-story checklist. -->

## Story

As a **Project Manager**,
I want **to view a real-time P&L dashboard**,
so that **I can monitor project profitability at a glance**.

## Acceptance Criteria

1. **Given** I navigate to Project -> P&L **when** the page loads (<2 seconds) **then** I see: Revenue, Total Costs, Gross Profit, Margin %.
2. **Given** I view the P&L **when** I select a time period **then** I see month-by-month breakdown with charts.
3. **Given** I set a target margin (e.g., 40%) **when** the current margin differs from target by >5% **then** I see an alert: "Margin below target: 33% vs 40% target".
4. **Given** I view the P&L chart **when** I hover over a data point **then** I see the breakdown: Revenue, Resource Costs, Non-Resource Costs, Margin.

## Scope Boundary

- **In scope**: project-level P&L aggregation using existing budget/resource-cost/expense/revenue data, summary KPI cards, monthly P&L breakdown for selected year, hover breakdown details, target-margin threshold alerting, PM/admin authorization checks, and dashboard rendering in existing Projects page.
- **Not in scope**: completion forecasting engine (Story 4.6), cash-in/cash-out views (Epic 5), ERP scheduler/circuit-breaker orchestration improvements, cross-project portfolio analytics, advanced BI exports, and multi-currency support.
- **Dependencies from prior stories**: Story 4.2 (`project_expenses`), Story 4.3 (`project_cost_service`), Story 4.4 (`project_revenues`) are mandatory data sources and must be reused, not reimplemented.

## Tasks / Subtasks

- [x] **Task 1: Add backend P&L service and canonical formulas** (AC: #1, #2, #4)
  - [x] Create `src/backend/src/services/project_pl_service.rs` for all P&L aggregation logic (keep `project.rs` handlers thin).
  - [x] Add service entry point:
    ```rust
    pub async fn get_project_pl_dashboard(
        pool: &PgPool,
        project_id: Uuid,
        year: i32,
    ) -> Result<ProjectPlDashboardResult>
    ```
  - [x] Reuse existing sources (see Data Source Integration Details below for exact signatures and format conversions):
    - `project_revenue_service::get_revenue_grid(pool, project_id, year)` → `ProjectRevenueGridResult` with `months: Vec<MonthRevenueEntry>` containing `month: u32` (1..12) and `amount_idr: i64`.
    - `project_cost_service::compute_project_resource_costs(pool, project_id)` → `ProjectResourceCostResult` with `monthly_breakdown: Vec<MonthlyCostEntry>` containing `month: String` (format `"YYYY-MM"`) and `cost_idr: i64`. **This returns ALL allocation periods, not year-scoped.** Filter entries where `entry.month` starts with the requested year string (e.g., `"2026-"`) and parse month number: `NaiveDate::parse_from_str(&format!("{}-01", entry.month), "%Y-%m-%d").map(|d| d.month())`.
    - `project_expenses` table (no existing monthly aggregation function). Write a SQL query directly in the P&L service:
      ```sql
      SELECT DATE_TRUNC('month', expense_date)::DATE as expense_month,
             COALESCE(SUM(amount_idr), 0)::BIGINT as total_idr
      FROM project_expenses
      WHERE project_id = $1
        AND expense_date >= $2 AND expense_date < $3
      GROUP BY DATE_TRUNC('month', expense_date)
      ORDER BY expense_month
      ```
      Map sparse results to dense 12-month grid with `0` for months without expenses. Keep SQL aggregate casting explicit to avoid `sqlx` `NUMERIC` inference mismatches when decoding into `i64`.
  - [x] Build dense 12-month P&L grid by merging all three data sources. For months where a source has no data, use `0`.
  - [x] Compute month-level fields for all 12 months:
    - `revenue_idr`
    - `resource_cost_idr`
    - `non_resource_cost_idr`
    - `total_cost_idr = resource_cost_idr + non_resource_cost_idr`
    - `gross_profit_idr = revenue_idr - total_cost_idr`
    - `margin_pct = if revenue_idr > 0 { gross_profit_idr as f64 / revenue_idr as f64 * 100.0 } else { 0.0 }`
  - [x] Compute summary fields for selected year from monthly values:
    - `total_revenue_idr`
    - `total_cost_idr`
    - `gross_profit_idr`
    - `margin_pct`
  - [x] Enforce deterministic month ordering `1..12` and stable labels (`Jan..Dec`) to match existing revenue grid style.
  - [x] Performance: consider batching the revenue fetch, cost fetch, and expense SQL in a `tokio::try_join!` to minimize sequential round trips.

- [x] **Task 2: Add project-level P&L settings for target margin alerting** (AC: #3)
  - [x] The `projects` table already has budget columns (`total_budget_idr`, `budget_hr_idr`, `budget_software_idr`, `budget_hardware_idr`, `budget_overhead_idr`). Add new columns alongside them.
  - [x] Add migration `migrations/<timestamp>_add_project_pl_settings.up.sql`:
    - [x] `ALTER TABLE projects ADD COLUMN IF NOT EXISTS target_margin_pct NUMERIC(5,2) NOT NULL DEFAULT 40.00`
    - [x] `ALTER TABLE projects ADD COLUMN IF NOT EXISTS margin_alert_threshold_pct NUMERIC(5,2) NOT NULL DEFAULT 5.00`
    - [x] Constraints: `target_margin_pct >= 0 AND target_margin_pct <= 100`, `margin_alert_threshold_pct >= 0 AND margin_alert_threshold_pct <= 100`.
  - [x] Add matching down migration dropping added columns.
  - [x] **sqlx type mapping**: PostgreSQL `NUMERIC(5,2)` maps to `BigDecimal` in sqlx by default, NOT `f64`. When querying these columns, use SQL cast: `target_margin_pct::FLOAT8 as "target_margin_pct!"` and `margin_alert_threshold_pct::FLOAT8 as "margin_alert_threshold_pct!"`. Alternatively use the project's `bigdecimal_to_f64()` helper from `budget_service.rs`.
  - [x] Add DTO(s) in `src/backend/src/routes/project.rs`:
    - [x] `SetProjectPlSettingsRequest { target_margin_pct: f64, margin_alert_threshold_pct: Option<f64> }`
    - [x] Include settings in P&L response payload.
  - [x] Validation:
    - [x] Reject out-of-range values using `AppError::Validation`.
    - [x] If threshold omitted, default to `5.0`.

- [x] **Task 3: Expose P&L API endpoints in project routes** (AC: #1, #2, #3, #4)
  - [x] Add route(s) in `src/backend/src/routes/project.rs`:
    - [x] `GET /api/v1/projects/:id/pl?year=YYYY`
    - [x] `PUT /api/v1/projects/:id/pl/settings` (target/threshold updates)
  - [x] Add query parameter DTO with default year (consistent with revenue endpoint):
    ```rust
    #[derive(Debug, Deserialize)]
    pub struct PlQueryParams {
        #[serde(default = "default_current_year")]
        pub year: i32,
    }
    fn default_current_year() -> i32 {
        chrono::Utc::now().naive_utc().date().year()
    }
    ```
    When `year` param is omitted, default to current year. Use `axum::extract::Query<PlQueryParams>`.
  - [x] Reuse `enforce_project_mutation_access(...)` for PM/admin enforcement on both endpoints.
  - [x] Keep project existence checks consistent with existing budget/revenue/resource-cost handlers:
    ```sql
    SELECT id FROM projects WHERE id = $1
    ```
  - [x] Return dense 12-month breakdown payload suitable for chart + table rendering.
  - [x] Add response DTOs:

    ```rust
    #[derive(Debug, Serialize)]
    pub struct ProjectPlDashboardResponse {
        pub project_id: Uuid,
        pub year: i32,
        pub total_revenue_idr: i64,
        pub total_cost_idr: i64,
        pub gross_profit_idr: i64,
        pub margin_pct: f64,
        pub target_margin_pct: f64,
        pub margin_alert_threshold_pct: f64,
        pub margin_alert: Option<String>,
        pub months: Vec<ProjectPlMonth>,
    }

    #[derive(Debug, Serialize)]
    pub struct ProjectPlMonth {
        pub month: u32,
        pub month_label: String,
        pub revenue_idr: i64,
        pub resource_cost_idr: i64,
        pub non_resource_cost_idr: i64,
        pub total_cost_idr: i64,
        pub gross_profit_idr: i64,
        pub margin_pct: f64,
    }
    ```

  - [x] AC#3 alert rule in backend payload:
    - [x] Trigger when `target_margin_pct - current_margin_pct > margin_alert_threshold_pct`.
    - [x] Format message exactly in style: `"Margin below target: {current}% vs {target}% target"`.

- [x] **Task 4: Implement frontend P&L dashboard section in Projects page** (AC: #1, #2, #4)
  - [x] Extend `src/frontend/src/components/project_list.rs` with `P&L` action callback (`on_view_pnl`) near existing Budget/Costs/Revenue actions.
  - [x] Extend `src/frontend/src/pages/projects.rs` with P&L state, fetch, and UI:
    - [x] Year selector and selected project binding.
    - [x] KPI cards: Revenue, Total Costs, Gross Profit, Margin %.
    - [x] Month-by-month chart view.
    - [x] Equivalent month-by-month data table (NFR25 accessibility parity for chart).
  - [x] Use established Leptos patterns already used for revenue (NOT imperative `spawn_local` — Story 4.4 review caught this mistake):
    - [x] `create_resource` for `GET /projects/:id/pl`:
      ```rust
      let pl_resource = create_resource(
          move || (selected_project_id.get(), selected_pl_year.get()),
          move |(pid, year)| async move {
              if let Some(id) = pid {
                  fetch_pl_dashboard(id, year).await.ok()
              } else { None }
          },
      );
      ```
    - [x] `create_action` for `PUT /projects/:id/pl/settings`. After successful action, refetch `pl_resource` to update display.
  - [x] Render chart as native SVG elements in Leptos `view!` macro (no new chart dependency). Use computed `<rect>` for bar charts or `<line>`/`<polyline>` for line charts. Use Tailwind-positioned `<div>` for hover tooltip with exact AC#4 fields:
    - [x] Revenue
    - [x] Resource Costs
    - [x] Non-Resource Costs
    - [x] Margin
  - [x] Accessibility + interaction parity for chart: support both pointer hover and keyboard focus for tooltip reveal, and include semantic attributes (`role="img"`, meaningful `aria-label`, focusable data points with `tabindex="0"`).
  - [x] Surface P&L fetch failures into the shared page error state (do not silently swallow failed dashboard requests).
  - [x] Keep layout/style consistent with existing Tailwind card/table style in `projects.rs`.

- [x] **Task 5: Target-margin UI and alert behavior** (AC: #3)
  - [x] Add editable target margin input in P&L section (default 40%, threshold default 5%).
  - [x] Save settings through `PUT /projects/:id/pl/settings`.
  - [x] Show alert banner when deviation rule is met; include current vs target values.
  - [x] Ensure alert text is visible in both light and dark themes and remains keyboard/screen-reader discoverable.

- [x] **Task 6: Add integration tests for P&L correctness, auth, and alert logic** (AC: #1, #2, #3, #4)
  - [x] Create `src/backend/tests/project_pl_tests.rs`.
  - [x] Required tests:
    - [x] PM can view P&L for own project.
    - [x] PM denied on non-owned project; admin allowed.
    - [x] Non-existent project returns `404`.
    - [x] Dense 12-month payload always returned.
    - [x] Formula correctness: `gross_profit = revenue - total_cost`, `margin% = profit / revenue`.
    - [x] Breakdown correctness: `total_cost = resource + non_resource`.
    - [x] Target margin alert triggers only when deviation exceeds threshold.
    - [x] Settings validation rejects invalid percentages.
  - [x] Full regression baseline (249+ tests as of Story 4.4): run `cargo test` for ALL backend tests, not just the suites listed below. Named suites to watch:
    - [x] `project_budget_tests.rs`
    - [x] `project_expense_tests.rs`
    - [x] `project_resource_cost_tests.rs`
    - [x] `project_revenue_tests.rs`
  - [x] One-command Epic 4 quick regression (expected during review cycles): `cargo test --package xynergy-backend --test project_budget_tests && cargo test --package xynergy-backend --test project_expense_tests && cargo test --package xynergy-backend --test project_resource_cost_tests && cargo test --package xynergy-backend --test project_revenue_tests && cargo test --package xynergy-backend --test project_pl_tests`.

- [x] **Task 7: Performance and accessibility guardrails** (AC: #1, #2, #4)
  - [x] Keep P&L generation within NFR1 target (<2s for 12-month horizon) by avoiding N+1 query patterns.
  - [x] Prefer pre-aggregated monthly SQL + existing service outputs; avoid repeated per-month per-resource loops in handlers.
  - [x] Add `aria-label` and keyboard-focus support for chart points/hover targets (NFR26/NFR22).
  - [x] Preserve chart-table parity to satisfy NFR25 (all chart information must be available in table form).

## Dev Notes

### Developer Context

- Story 4.1 established project budget storage and budget endpoints in `src/backend/src/routes/project.rs`.
- Story 4.2 added `project_expenses` and budget-spend integration.
- Story 4.3 added canonical project resource-cost computation in `src/backend/src/services/project_cost_service.rs` and integrated resource spend into budget totals.
- Story 4.4 added project revenue ledger (`project_revenues`) and month-grid APIs in `project_revenue_service.rs`.
- Story 4.5 must aggregate 4.2 + 4.3 + 4.4 outputs into a single PM-facing profitability dashboard without duplicating prior business logic.
- **`project_expenses` table schema** (for direct SQL in P&L service): columns are `id UUID`, `project_id UUID`, `category TEXT` (hr/software/hardware/overhead), `description TEXT`, `amount_idr BIGINT`, `expense_date DATE`, `vendor TEXT`, `created_by UUID`, `created_at TIMESTAMPTZ`, `updated_at TIMESTAMPTZ`. Indexed on `(project_id, category)`.

### Dev Guardrails

**Calculation and Data Sources**

- P&L formulas must be deterministic and centralized in `project_pl_service.rs`.
- Do not duplicate revenue-grid month normalization logic; reuse 4.4 service output.
- Do not recalculate resource-cost formulas in P&L route; reuse 4.3 service output.
- Keep all money values as integer IDR (`i64`/`BIGINT`) in API and storage.

**Self-Verification Discipline**

- CRITICAL: After completing each task, verify the actual code matches every `[ ]` claim before marking done. Story 4.4 had 3 critical review findings for "claimed but not implemented" items (idempotency-key handling, Leptos create_resource pattern, entered_by display). Do not repeat this pattern.

**Story 4.5 Review Intelligence (Applied in Implementation)**

- Keep backend/frontend/test route contracts strictly aligned (`/api/v1/projects/:id/pl` and `/api/v1/projects/:id/pl/settings`) to avoid SPA fallback HTML responses being misread as API success.
- Margin visual cues must reflect actual threshold semantics (below target -> warning/error state) in both KPI and table rows.
- AC#4 requires explicit hover/focus breakdown details; do not ship chart-only aggregates without Revenue/Resource/Non-Resource/Margin tooltip details.
- Keep SVG chart accessibility attributes explicit (`role`, `aria-label`, keyboard focus targets) and preserve chart-table data parity.
- Validate target margin and threshold inputs before mutation dispatch; reject parse/range errors early and surface user-facing feedback.
- Ensure P&L fetch errors are visible in UI state; avoid silent failures in reactive resources.

**Authorization and Audit**

- Reuse `enforce_project_mutation_access(...)` for PM/admin consistency with expense/resource/revenue endpoints.
- Log denied access (`ACCESS_DENIED`) with entity type `project_pl` / `project_pl_settings`.
- Log settings updates with before/after payload via `audit_payload(...)`.

**API/Route Consistency**

- Keep route style: `/api/v1/projects/:id/...` in `project_routes()`.
- Keep project existence checks explicit and early.
- Use `Query` default-year pattern consistent with revenue endpoint.

**Frontend UX Consistency**

- Keep P&L section in existing `Projects` page flow; do not create a separate P&L page route.
- Follow existing card/table styling and interaction patterns from budget/revenue sections.
- Ensure chart and table convey identical values.

### Architecture Compliance

- Handlers remain thin; aggregation/calculation in service layer.
- Favor `sqlx::query!`/`query_as!` where feasible with compile-time checks.
- Maintain existing module boundaries and naming conventions.
- Avoid introducing heavy new frontend dependencies unless absolutely required.

### Library/Framework Requirements

- Keep stack pinned: Rust 1.75+, Axum 0.7, sqlx 0.7, Leptos 0.6.
- Maintain existing Axum route syntax (`/projects/:id/...`).
- Preserve current auth/JWT/header extraction patterns.

### File Structure Requirements

- Backend:
  - `migrations/<timestamp>_add_project_pl_settings.up.sql` (new)
  - `migrations/<timestamp>_add_project_pl_settings.down.sql` (new)
  - `src/backend/src/services/project_pl_service.rs` (new)
  - `src/backend/src/services/mod.rs` (modified: export `project_pl_service`)
  - `src/backend/src/routes/project.rs` (modified: DTOs + handlers + routes)
- Frontend:
  - `src/frontend/src/pages/projects.rs` (modified: P&L section, chart/table, alerts)
  - `src/frontend/src/components/project_list.rs` (modified: P&L action)
- Tests:
  - `src/backend/tests/project_pl_tests.rs` (new)

### Testing Requirements

- Use integration pattern: `#[sqlx::test(migrations = "../../migrations")]`.
- Reuse helper setup style from `project_budget_tests.rs` and `project_revenue_tests.rs`.
- Validate formula correctness with concrete monthly fixtures.
- Cover auth-denied + not-found + validation failures.
- Keep prior Epic 4 regressions green.

### Previous Story Intelligence (4.4)

- 4.4 stabilized patterns to reuse:
  - `create_resource` + `create_action` on frontend for year-scoped financial data.
  - Dense 12-month API response for reliable chart/table rendering.
  - Idempotent ERP semantics already captured in revenue source rows.
  - PM/admin mutation-access helper naming (`enforce_project_mutation_access`).
- 4.4 code review caught 3 critical issues — learn from these:
  - **Missing claimed feature**: Task 4 claimed idempotency-key handling was implemented, but no code existed. Fix: always verify code matches `[x]` claims.
  - **Wrong Leptos pattern**: Revenue flow used imperative `spawn_local` instead of required `create_resource` + `create_action`. Fix: use declarative resource/action patterns, not imperative signal + spawn_local.
  - **Missing UI element**: Task 6 required `entered_by` display in row details, but frontend only rendered `entry_date`. Fix: cross-check every AC field against rendered UI.
- Apply these lessons to P&L dashboard: verify every task claim against actual code before marking complete.

### Git Intelligence Summary

- Recent Epic 4 implementation trend is vertical slice extension of:
  - `src/backend/src/routes/project.rs`
  - `src/backend/src/services/*`
  - `src/frontend/src/pages/projects.rs`
  - Story-specific integration tests in `src/backend/tests/`
- Follow same structure to minimize regression risk and merge friction.

### Latest Technical Information

- Axum 0.7 extractor and rejection behavior is documented in official docs.rs (`axum::extract` 0.7.4); keep current extractor patterns and avoid mixing 0.8 style assumptions.
- Leptos ecosystem has chart crates (e.g., `leptos-chartistry`), but this project currently has no chart dependency. Use native SVG elements in Leptos `view!` macro: `<svg>` with computed `<rect>` elements for bar charts (one per month), `<text>` for axis labels, and Tailwind-positioned `<div>` overlays for hover tooltips. Keep chart simple — 12 bars for months, color-coded segments for revenue vs costs.
- Accessibility NFRs require chart data parity with table alternative and ARIA labeling for interactive chart elements.
- `project_cost_service::compute_project_resource_costs()` returns `MonthlyCostEntry` with `month: String` format (e.g., `"2026-01"`), not month number. Parse with `NaiveDate::parse_from_str` for year filtering and month extraction.

### Project Context Reference

- Core context: `_bmad-output/project-context.md`
- Planning artifacts:
  - `_bmad-output/planning-artifacts/epics.md`
  - `_bmad-output/planning-artifacts/prd.md`
  - `_bmad-output/planning-artifacts/architecture.md`
  - `_bmad-output/planning-artifacts/ux-design-specification.md`

### References

1. `_bmad-output/planning-artifacts/epics.md` - Story 4.5 narrative + ACs, Epic 4 sequencing.
2. `_bmad-output/planning-artifacts/prd.md` - FR24-FR29 and NFR1/NFR25/NFR26 constraints for P&L dashboard.
3. `_bmad-output/planning-artifacts/architecture.md` - planned P&L endpoint patterns and service-layer architecture decisions.
4. `_bmad-output/planning-artifacts/ux-design-specification.md` - dashboard UX, hover transparency, chart/table clarity expectations.
5. `_bmad-output/project-context.md` - stack/version guardrails and backend/frontend coding rules.
6. `_bmad-output/implementation-artifacts/4-4-revenue-entry.md` - prior-story data contracts and implementation learnings.
7. `src/backend/src/routes/project.rs` - existing budget/resource/revenue route and auth patterns.
8. `src/backend/src/services/project_revenue_service.rs` - canonical monthly revenue aggregation used by P&L.
9. `src/frontend/src/pages/projects.rs` - existing financial sections and Leptos fetch/mutation patterns.
10. `src/backend/src/services/project_cost_service.rs` - resource cost computation; `compute_project_resource_costs(pool, project_id)` returns all-period `monthly_breakdown` with `month: String` format.
11. `migrations/20260305100000_add_project_expenses.up.sql` - `project_expenses` table schema for direct SQL aggregation in P&L service.

## Dev Agent Record

### Agent Model Used

openai/gpt-5.3-codex

### Debug Log References

- Workflow source: `_bmad/bmm/workflows/4-implementation/create-story/workflow.yaml`
- Workflow instructions: `_bmad/bmm/workflows/4-implementation/create-story/instructions.xml`
- Validation checklist: `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- Sprint tracking source: `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Completion Notes List

- Story created from explicit user target `4-5` (`4-5-pl-dashboard`); no auto-discovery required.
- Context synthesized from Epic 4 planning artifacts, architecture constraints, UX dashboard guidance, project context rules, and Story 4.4 implementation learnings.
- Story intentionally enforces reuse of existing 4.2/4.3/4.4 financial primitives to prevent formula drift and duplicated logic.
- Explicit data source integration details added: revenue grid reuse, resource cost year-filtering with String→u32 month format conversion, and new expense aggregation SQL for monthly non-resource costs.
- sqlx NUMERIC(5,2)→f64 type mapping documented with cast requirements to prevent BigDecimal conversion errors.
- Self-verification discipline added based on Story 4.4 review findings (3 critical "claimed but not implemented" issues).
- Full regression baseline (249+ tests) documented; all backend tests must pass, not just named suites.
- Concrete Leptos `create_resource` pattern and SVG chart approach specified to prevent pattern mistakes.
- 2026-03-05: Story implementation plus review follow-up fixes completed (API contract alignment, chart hover breakdown, accessibility, validation, and margin-color logic); status transitioned `review` -> `done` after Epic 4 regression passed.
- 2026-03-05: Re-validated via `validate-create-story`; applied 3 critical clarifications (expense SQL type-cast guardrail, callback naming alignment, route-contract/runtime guardrail), 3 enhancements (chart accessibility/interaction parity, visible fetch-error requirement, one-command Epic 4 regression command), and 1 LLM optimization (consolidated review intelligence checklist for fast implementation scanning).
- 2026-03-06: Story file bookkeeping reconciled — task checkboxes, File List, and Change Log were not updated during original dev workflow. All implementation artifacts verified present; checkboxes marked [x] retroactively.

### File List

- `_bmad-output/implementation-artifacts/4-5-pl-dashboard.md` — this story artifact
- `migrations/20260305184822_add_project_pl_settings.up.sql` — target_margin_pct + margin_alert_threshold_pct columns (new)
- `migrations/20260305184822_add_project_pl_settings.down.sql` — rollback migration (new)
- `src/backend/src/services/project_pl_service.rs` — P&L aggregation service: dashboard + forecast (new)
- `src/backend/src/services/mod.rs` — added `pub mod project_pl_service;` (modified)
- `src/backend/src/routes/project.rs` — P&L DTOs, handlers (`get_project_pl_dashboard`, `get_project_pl_forecast`, settings PUT), route registration (modified)
- `src/backend/tests/project_pl_tests.rs` — 6 integration tests (new)
- `src/frontend/src/pages/projects.rs` — P&L dashboard section with KPI cards, SVG chart, month table, hover tooltips, year nav, target-margin settings, alert banner (modified)
- `src/frontend/src/components/project_list.rs` — added `on_view_pnl` callback + P&L button (modified)
- `.sqlx/` — query cache files refreshed (modified)

### Change Log

- 2026-03-05: Story implementation completed (Tasks 1-7) with all backend services, migrations, API endpoints, frontend dashboard, and integration tests.
- 2026-03-05: Code review completed; review follow-up fixes applied (API contract alignment, chart hover breakdown, accessibility, validation, margin-color logic). Status transitioned `review` → `done` after Epic 4 regression passed.
- 2026-03-05: Re-validated via `validate-create-story`; applied clarifications and enhancements.
- 2026-03-06: Story file bookkeeping reconciled — all task/subtask checkboxes marked [x], File List populated with actual implementation files, Change Log added. Implementation was verified complete via file existence checks, grep of route registrations, service modules, frontend integration, and test files.
