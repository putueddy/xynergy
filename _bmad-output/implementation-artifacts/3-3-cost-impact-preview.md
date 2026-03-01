# Story 3.3: Cost Impact Preview

Status: done

<!-- Validated: All improvements from validate-create-story applied (C1-C5, E1-E5, O1-O3, L1-L3). -->

## Story

As a **Department Head**,
I want **to see the cost impact of an assignment BEFORE confirming**,
so that **I can make cost-aware staffing decisions**.

## Acceptance Criteria

1. **Given** I am creating a new assignment **when** I enter date range and allocation percentage **then** the system calculates `daily_rate × count_working_days(start_date, end_date, include_weekend, holidays) × (allocation_percentage / 100)` and displays total cost impact in real time.
2. **Given** the cost impact is calculated **when** I view the preview panel **then** I see total cost, monthly breakdown, and impact on department budget with color-coded budget health indicator (🟢 green <50%, 🟡 yellow 50-80%, 🔴 red >80% utilization).
3. **Given** the assignment would exceed department budget **when** the preview is displayed **then** I see warning copy like `This assignment consumes Rp XXM of your Rp YYM budget` and approval requirement behavior follows configuration.
4. **Given** I review the cost impact **when** I click confirm assignment **then** a confirmation summary shows resource name + daily rate, project name, duration + allocation%, total cost, and remaining budget; assignment is saved and budget utilization updates in real time.

## Scope Boundary

- **In scope**: Cost preview for a single draft assignment, department budget impact, budget warning/approval behavior.
- **NOT in scope**: What-if resource comparison (swap Doni with Andi), multi-assignment batch preview, P&L projections (Epic 4), or timeline overlap highlighting fix (3.2 M2 follow-up).

## Tasks / Subtasks

- [x] **Task 1: Add backend cost-preview API contract for assignment draft input** (AC: #1, #2, #3)
  - [x] Add `GET /api/v1/allocations/cost-preview` in `allocation.rs` accepting query params: `resource_id`, `project_id`, `start_date`, `end_date`, `allocation_percentage`, `include_weekend`.
  - [x] **Architecture note**: The architecture doc shows `/allocations/:id/cost-preview` for existing allocations, but this endpoint previews a **draft** (no allocation ID exists yet). Using query parameters on a collection-level GET is the correct pattern here. Do not use `:id` in the route.
  - [x] Reuse existing guards in `allocation.rs`: role envelope (`admin`, `department_head`, `project_manager`) via `ensure_allocation_access()`, project-manager ownership checks, date-order validation, project date-bound validation, and CTC-exists validation. These guards ensure preview and create cannot disagree on validity.
  - [x] Return deterministic `CostPreviewResponse` payload (see [API Response Types](#api-response-types) below).
  - [x] Keep all money values as `i64` integer IDR in API payloads (no persisted float money fields).

- [x] **Task 2: Implement working-days and monthly cost-split computation by reusing allocation conventions** (AC: #1, #2)
  - [x] Reuse the existing working-days calculation in `allocation.rs` (lines ~154-174) which iterates dates, checks `include_weekend`, and queries the `holidays` table. Do NOT reimplement date iteration logic.
  - [x] Reuse BigDecimal patterns from `src/backend/src/services/ctc_calculator.rs` — specifically string-based parsing (`"value".parse::<BigDecimal>()`) and the `bigdecimal_to_f64` / `f64_to_bigdecimal` helpers. Do NOT use `BigDecimal::try_from(f64)` which causes precision loss.
  - [x] Add a pure helper function (e.g., in a new `src/backend/src/services/cost_preview.rs` or extending `ctc_calculator.rs`) for cost computation and monthly bucketing so it is unit-testable without HTTP layer.
  - [x] Formula: `total_cost_idr = daily_rate_idr × count_working_days(start_date, end_date, include_weekend, holidays) × (allocation_percentage / 100)`. Where `daily_rate_idr` comes from the resource's active CTC record `daily_rate` field (already an `i64`), and `count_working_days` reuses the allocation.rs date iteration approach.
  - [x] Monthly group-by: iterate working days, bucket each into `YYYY-MM` key, compute per-bucket cost as `daily_rate_idr × bucket_working_days × (allocation_percentage / 100)`.
  - [x] Rounding: truncate fractional IDR (floor to integer) — IDR has no subunit. Use integer arithmetic where possible; convert to BigDecimal only for the percentage multiplication step, then truncate back to `i64`.
- [x] **Task 3: Introduce minimal department budget schema and integrate budget impact** (AC: #2, #3, #4)
  - [x] **Create migration** `migrations/YYYYMMDDHHMMSS_add_department_budgets.up.sql`:
    ```sql
    CREATE TABLE department_budgets (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        department_id UUID NOT NULL REFERENCES departments(id),
        budget_period VARCHAR(7) NOT NULL,          -- 'YYYY-MM' format
        total_budget_idr BIGINT NOT NULL DEFAULT 0, -- total monthly budget in IDR
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        UNIQUE(department_id, budget_period)
    );
    ```
    Include matching `.down.sql` with `DROP TABLE IF EXISTS department_budgets;`.
  - [x] Keep scope minimal: this table stores budget caps only. `committed_idr` is computed at query time by summing active allocation costs for the department+period — do NOT store a denormalized committed amount that can drift.
  - [x] Compute budget impact fields for preview response:
    - `department_budget_total_idr`: from `department_budgets` row for the assignment's month(s)
    - `current_committed_idr`: SUM of `daily_rate × working_days × allocation%` for all active allocations in the department for the same period(s)
    - `projected_committed_idr`: `current_committed_idr + this_assignment_cost`
    - `remaining_after_assignment_idr`: `department_budget_total_idr - projected_committed_idr`
    - `utilization_percentage`: `(projected_committed_idr / department_budget_total_idr) × 100`
  - [x] If no `department_budgets` row exists for a given period, return `budget_impact: null` in the response (graceful fallback — preview still works for cost display, just without budget context).
  - [x] **Budget health thresholds** (from UX spec):
    - 🟢 `healthy`: utilization < 50%
    - 🟡 `warning`: utilization 50-80%
    - 🔴 `critical`: utilization > 80%
    - Return `budget_health: "healthy" | "warning" | "critical"` in response.
  - [x] **Warning copy**: when `critical`, include `warning: "This assignment consumes Rp {cost}M of your Rp {total}M budget ({utilization}% utilized)"`.
  - [x] Keep approval-required behavior config-driven and default-safe (warn-only when config is absent). Use an env var or app config flag `BUDGET_OVERRUN_POLICY=warn|block` defaulting to `warn`.
  - [x] Audit log budget warning events: when preview returns `critical` status, log an audit entry for visibility.

- [x] **Task 4: Add real-time preview panel to Team assignment modal** (AC: #1, #2, #3)
  - [x] Add reactive signals for preview state in `src/frontend/src/pages/team.rs`:
    ```rust
    let (preview_data, set_preview_data) = create_signal(None::<CostPreviewResponse>);
    let (preview_loading, set_preview_loading) = create_signal(false);
    let (preview_error, set_preview_error) = create_signal(None::<String>);
    ```
  - [x] Create a debounced effect watching `(assign_project_id, assign_start_date, assign_end_date, assign_pct)` signals. **Debounce interval: 300ms**. Only fire when ALL four fields are non-empty and valid. Use `set_timeout` from `gloo-timers` or equivalent WASM timer.
  - [x] Render preview panel below allocation% input and above action buttons, with three blocks:
    1. **Total Cost**: `Rp {total_cost_idr}` formatted via existing `format_idr()` helper, with daily rate × working days formula visible.
    2. **Monthly Breakdown**: table with columns `Month | Working Days | Cost (IDR)` — one row per `monthly_breakdown[]` entry.
    3. **Department Budget Impact**: progress bar showing utilization percentage, color-coded per budget health thresholds (green/yellow/red). Show `remaining_after_assignment_idr`. If `budget_impact` is null (no budget configured), show muted text: "Department budget not configured."
  - [x] **Hover formula tooltip**: on the total cost value, show tooltip: `"{daily_rate_idr} × {working_days} days × {allocation_percentage}% = {total_cost_idr}"`. Reuse Tailwind tooltip pattern: `absolute z-50 px-2 py-1 bg-gray-900 text-white text-xs rounded`.
  - [x] Render warning banner when `budget_health == "critical"`: red background, warning icon, exact copy from `warning` field. When `requires_approval == true`, show additional text: "Approval required for this assignment."
  - [x] Show skeleton/loading state during preview fetch. Show preview error state if API returns validation error.
- [x] **Task 5: Ensure confirm assignment updates budget utilization in same user flow** (AC: #4)
  - [x] Keep `POST /api/v1/allocations` as the assignment write endpoint.
  - [x] **Enhance confirmation step**: Before submit, show a confirmation summary panel (or enhance existing confirm UX) displaying:
    - Resource name + daily rate
    - Project name
    - Duration (start → end) + allocation%
    - Total cost (from preview)
    - Remaining department budget after assignment (from preview, if available)
  - [x] On successful POST, refresh data by:
    - Calling `set_team_members.set(...)` with fresh data (same pattern as existing 3-2 submit handler which calls team endpoint).
    - Clearing `set_preview_data.set(None)` to reset preview panel.
    - Clearing form signals and showing success toast.
  - [x] If user reopens assignment modal or starts a new assignment, stale preview data must not persist — clear on modal open (same reset pattern as 3-2's `set_show_assign_modal` open handler).
  - [x] Ensure UI transitions from preview state → confirmed state without stale budget totals by triggering fresh preview fetch if modal remains open.
- [x] **Task 6: Add integration and regression tests for preview and budget impact paths** (AC: #1, #2, #3, #4)
  - [x] Add backend integration tests for `GET /api/v1/allocations/cost-preview` happy path and validation failures (missing CTC, invalid dates, unauthorized role).
  - [x] Add parity tests: for the same inputs, verify that preview validation accepts/rejects identically to `POST /api/v1/allocations` create validation.
  - [x] Add tests for monthly split correctness across month boundaries, weekends, holidays, and allocation percentages (e.g., 50% allocation, range spanning Feb-Mar with holidays).
  - [x] Add tests for budget impact: under-threshold (healthy), near-threshold (warning), over-threshold (critical), and no-budget-configured (null fallback).
  - [x] Add tests that confirm assignment write path changes budget utilization: create assignment → re-query preview → verify `current_committed_idr` increased by expected amount.
  - [x] Keep Story 3.2's 18 assignment tests green — run full `assignment_tests.rs` suite.
## Dev Notes

### Known Limitations

- **M4: All-or-Nothing Budget Period Matching** — When an assignment spans multiple months, budget impact is only computed if `department_budgets` rows exist for ALL months in the range. If one month is missing, `budget_impact` returns `null` for the entire preview. This is an intentional conservative design: partial budget data could produce misleading utilization percentages. Future improvement: allow partial budget matching with explicit "budget data incomplete" indicator.

### Developer Context (Critical)

- Story 3.2 assignment workflow is implemented and stable; Story 3.3 must extend that flow without replacing it.
- Existing assignment creation validates role, project bounds, date ordering, allocation bounds (0 < % ≤ 100), and CTC existence in `src/backend/src/routes/allocation.rs`. Seven handlers exist: `get_allocations`, `get_allocations_by_project`, `get_allocations_by_resource`, `create_allocation`, `update_allocation`, `delete_allocation`, plus `allocation_routes()`. **No cost logic exists yet** — preview is entirely new.
- Team assignment UI exists in `src/frontend/src/pages/team.rs` with 10 reactive signals for assignment state (`show_assign_modal`, `assign_resource_id`, `assign_project_id`, `assign_start_date`, `assign_end_date`, `assign_pct`, `assign_error`, `assign_success`, `assign_submitting`, `assignable_projects`). Preview signals extend this set.
- `src/backend/src/services/ctc_calculator.rs` (from Story 2.1) contains daily rate computation and BigDecimal best practices — **reuse these patterns** for cost calculation. Do not create duplicate computation logic.
- **No department-budget schema exists** in current migrations or backend routes. Task 3 introduces a minimal `department_budgets` table scoped to this story's needs only.

### Dev Guardrails

**RBAC & Auth:**
- Extract user identity via `user_claims_from_headers(&headers)?` (returns full JWT Claims with role) — defined in `src/backend/src/services/audit_log.rs:164`. Use this for role-based access control on preview endpoint.
- `user_id_from_headers(&headers)?` (returns just UUID) also exists at `audit_log.rs:189` — use when only user ID needed (e.g., audit logging).
- Reuse `ensure_allocation_access()` from allocation.rs for preview endpoint access control — do not duplicate role checking logic.

**SQL & Data Safety:**
- Keep SQL parameterized with `sqlx` query macros; no dynamic SQL string concatenation.
- Use integer `i64` for all IDR money values in API payloads and database columns (`BIGINT`). Do not persist money as `f64`.
- **BigDecimal precision**: use string parsing (`"0.04".parse::<BigDecimal>()`) not `BigDecimal::try_from(f64)`. Use `bigdecimal_to_f64()` and `f64_to_bigdecimal()` helpers from project conventions. For percentage multiplication: convert allocation% to BigDecimal via string, multiply, then truncate result to `i64` via `bd.to_string().split('.').next()`.
- Explicit type casts in SQL for non-standard types: `SELECT date::TEXT as "date!" FROM holidays`.

**Architecture:**
- Keep route handlers thin; place cost calculation in service/helper layer (new `cost_preview.rs` service or extend `ctc_calculator.rs`).
- Register preview route within existing `allocation_routes()` in `routes/allocation.rs` — no new route module needed.
- Follow existing error envelope (`AppError` mapping) and deterministic validation messaging.
- Do not bypass RLS/session constraints already used by team and allocation endpoints.

**Audit & Logging:**
- Audit log on: assignment confirmation (existing), access denied on preview (reuse existing), budget `critical` threshold trigger (new).
- Preview reads of daily rate are acceptable without individual audit entries (daily rate is already exposed via team endpoint in 3.1); do NOT add per-preview-request audit logging as it would generate excessive noise.

**Rate Cache:**
- Architecture.md defines a `RateCache` (DashMap, 24h TTL). For MVP preview, querying CTC daily_rate directly is acceptable (<200ms target). If latency becomes an issue, consider caching daily rates — but do not implement the cache in this story unless preview response exceeds 200ms in testing.

### API Response Types

**`CostPreviewResponse`** — returned by `GET /api/v1/allocations/cost-preview`:
```rust
#[derive(Debug, Serialize)]
pub struct CostPreviewResponse {
    pub daily_rate_idr: i64,
    pub working_days: i32,
    pub allocation_percentage: f64,
    pub total_cost_idr: i64,
    pub monthly_breakdown: Vec<MonthlyBucket>,
    pub budget_impact: Option<BudgetImpact>,  // None when no budget configured
    pub warning: Option<String>,               // Human-readable warning copy
    pub requires_approval: bool,
}

#[derive(Debug, Serialize)]
pub struct MonthlyBucket {
    pub month: String,          // "YYYY-MM" format
    pub working_days: i32,
    pub cost_idr: i64,
}

#[derive(Debug, Serialize)]
pub struct BudgetImpact {
    pub department_budget_total_idr: i64,
    pub current_committed_idr: i64,
    pub projected_committed_idr: i64,
    pub remaining_after_assignment_idr: i64,
    pub utilization_percentage: f64,
    pub budget_health: String,  // "healthy" | "warning" | "critical"
}
```

### Library / Framework Requirements

- Backend: Rust 1.75+, Axum 0.7, sqlx 0.7, PostgreSQL 15+, chrono 0.4, serde 1.0, bigdecimal 0.4.
- Frontend: Leptos 0.6 with `spawn_local`/reactive signal pattern currently used in Team page. Use `gloo-timers` (or `web_sys::set_timeout`) for debounce timer.
- Keep API contracts compatible with existing frontend authenticated helper functions.

### File Structure Requirements

- Backend touchpoints:
  - `src/backend/src/routes/allocation.rs` — add `cost_preview` handler and route
  - `src/backend/src/services/cost_preview.rs` (new) — pure cost computation and monthly bucketing logic
  - `src/backend/src/services/mod.rs` — export `cost_preview` module
  - `migrations/YYYYMMDDHHMMSS_add_department_budgets.up.sql` (new)
  - `migrations/YYYYMMDDHHMMSS_add_department_budgets.down.sql` (new)
- Frontend touchpoints:
  - `src/frontend/src/pages/team.rs` — preview signals, debounced effect, preview panel UI, enhanced confirmation
- Tests:
  - `src/backend/tests/cost_preview_tests.rs` (new) — preview-specific integration tests
  - `src/backend/tests/assignment_tests.rs` — verify existing 18 tests remain green

### Testing Requirements

- Use backend integration pattern `#[sqlx::test(migrations = "../../migrations")]`.
- Add parity tests asserting preview validation equals create validation for same inputs.
- Test monthly-breakdown splits for cross-month ranges and holiday/weekend behavior.
- Test budget-impact warning: healthy (<50%), warning (50-80%), critical (>80%), and null (no budget configured).
- Test that confirmed assignment updates utilization: create → re-preview → verify committed increased.
- Test approval-required flag behavior under warn vs block policy config.

### Previous Story Intelligence (3.2)

- Reuse existing `POST /api/v1/allocations` and do not introduce parallel assignment write paths.
- Keep assignable project filtering rules from `GET /api/v1/projects/assignable` unchanged.
- Keep CTC-missing guard and deterministic assignment error copy already established.
- Team page modal has 10 signals (listed above in Developer Context); preview extends with 3 new signals.
- 3-2 submit handler pattern: on success → fetch fresh team data → clear form → show success toast. Story 3.3 follows the same pattern plus clears preview state.
- Known follow-up from 3.2: timeline bar overlap highlighting (M2) is still open and should NOT be mixed into this story.

### Git Intelligence Summary

- Recent commits show feature-focused increments across stories (`feat:` style) with backend + tests + frontend continuity.
- Story sequence confirms Epic 3 implementation trend: extend existing modules (`team.rs`, `allocation.rs`, `project.rs`) instead of introducing new parallel subsystems.
- Story 3.1 and 3.2 established assignment and blended-rate contracts that Story 3.3 should build on directly.

### Latest Technical Information

- Axum 0.7: extractor-driven handlers with `State`, `Path`, `Query`, `Json`. For the preview endpoint, use `Query<CostPreviewQuery>` extractor to parse query parameters (not Path or Json, since this is a GET).
- Leptos: `spawn_local` + signals for async UI updates (current Team page pattern). For debounce, use `set_timeout` with a stored timeout handle that gets cleared on each signal change.
- Continue compile-time-safe `sqlx` usage and deterministic response shapes.

### Project Structure Notes

- No structural conflicts detected with existing module layout.
- Story 3.3 remains a direct extension of Epic 3 assignment workflow.
- **Scope fence**: Do not leak into Epic 4 P&L territory. Budget impact here is per-assignment preview only, not department-wide P&L reporting.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.3: Cost Impact Preview]
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 3: Department Resource Assignment]
- [Source: _bmad-output/planning-artifacts/prd.md#FR11 — Cost impact preview before confirming assignments]
- [Source: _bmad-output/planning-artifacts/prd.md#FR13 — Department budget utilization real-time view]
- [Source: _bmad-output/planning-artifacts/prd.md#FR22 — Budget overrun prevention (configurable warn/block)]
- [Source: _bmad-output/planning-artifacts/architecture.md#Calculation Engine Decisions]
- [Source: _bmad-output/planning-artifacts/architecture.md#API Design Decisions]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Experience Mechanics — Steps 1-4, color thresholds, confirmation modal]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Effortless Interactions — Cost Impact Visibility, Calculation Transparency]
- [Source: _bmad-output/project-context.md#Critical Implementation Rules — BigDecimal, error handling, audit]
- [Source: _bmad-output/implementation-artifacts/3-2-resource-assignment-interface.md]
- [Source: src/backend/src/routes/allocation.rs — 7 handlers, ensure_allocation_access(), capacity logic]
- [Source: src/backend/src/services/ctc_calculator.rs — BigDecimal patterns, daily rate computation]
- [Source: src/backend/src/services/audit_log.rs — user_claims_from_headers (line 164), user_id_from_headers (line 189)]
- [Source: src/frontend/src/pages/team.rs — assignment modal, 10 signals, format_idr() helper]

### Story Creation Completion Note

- Ultimate context engine analysis completed — comprehensive developer guide created
- Validated via `validate-create-story` checklist: 5 critical issues fixed (C1-C5), 5 enhancements applied (E1-E5), 3 optimizations added (O1-O3), 3 LLM optimizations applied (L1-L3)

## Dev Agent Record

### Agent Model Used

anthropic/claude-opus-4-6

### Debug Log References

- Workflow source: `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`
- Workflow instructions: `_bmad/bmm/workflows/4-implementation/dev-story/instructions.xml`
- Validation checklist: `_bmad/bmm/workflows/4-implementation/dev-story/checklist.md`
- Sprint tracking source: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Create-story workflow source: `_bmad/bmm/workflows/4-implementation/create-story/workflow.yaml`
### Completion Notes List

- Story target provided by user input as `3-3`; output file created directly for that key.
- Core context assembled from epic + architecture + PRD + UX + project context + previous story + current codebase.
- Reuse guidance emphasizes extending `allocation.rs` and Team modal flow rather than introducing parallel assignment systems.
- Department budget persistence introduced as minimal `department_budgets` table with computed committed amounts (no denormalized drift risk).
- Validation review applied all 16 improvement categories from checklist analysis.
- Concrete Rust response types provided to eliminate dev agent ambiguity.
- Budget health thresholds, formula disambiguation, debounce interval, confirmation modal spec all explicit.
- Scope boundary defined: excludes what-if comparison, P&L, batch preview, timeline M2 fix.
- Backend implementation: migration, service layer (`cost_preview.rs`), endpoint handler with CTC decryption, budget impact computation, audit logging for critical budget threshold.
- Frontend implementation: 3 preview signals, 300ms debounced effect via `gloo_timers::callback::Timeout`, preview panel with total cost + formula tooltip + monthly breakdown table + budget impact progress bar + warning banner, assignment summary in confirmation area, state cleanup on modal open/success.
- Integration tests: 5 tests covering happy path, validation parity, monthly split across boundaries, budget impact null fallback, and unauthorized role rejection. All 18 existing assignment tests remain green.
- Code-review fixes applied: H1 (confirmation summary now shows resource name, daily rate, project name, start/end dates, allocation%), H2 (3 integration tests added for budget health thresholds, committed increase, BUDGET_OVERRUN_POLICY=block), M2 (crypto service created once before loop in compute_budget_impact), M3 (summary moved above button container), M4 (documented as known limitation), M5 (is_weekend exported from cost_preview.rs, removed duplicate in allocation.rs).
### Validation Review Applied

| Code | Category | Summary |
|------|----------|---------|
| C1 | Critical | Added concrete `department_budgets` migration schema |
| C2 | Critical | Disambiguated `working_days` formula with codebase reference |
| C3 | Critical | Added UX color-coded budget thresholds (green/yellow/red) |
| C4 | Critical | Referenced `ctc_calculator.rs` and BigDecimal precision patterns |
| C5 | Critical | Documented architecture route deviation (draft preview vs `:id`) |
| E1 | Enhancement | Added confirmation modal cost summary specification |
| E2 | Enhancement | Added hover formula tooltip requirement |
| E3 | Enhancement | Defined `MonthlyBucket` response shape |
| E4 | Enhancement | Specified 300ms debounce interval |
| E5 | Enhancement | Added explicit scope boundary section |
| O1 | Optimization | Added progress bar budget visualization |
| O2 | Optimization | Addressed rate cache usage decision |
| O3 | Optimization | Clarified audit logging scope for preview reads |
| L1 | LLM Optimization | Consolidated Technical Requirements + Architecture into Dev Guardrails |
| L2 | LLM Optimization | Added concrete Rust type definitions |
| L3 | LLM Optimization | Specified Task 5 signal refresh pattern |

### File List

**Created:**
- `_bmad-output/implementation-artifacts/3-3-cost-impact-preview.md`
- `migrations/20260302100000_add_department_budgets.up.sql`
- `migrations/20260302100000_add_department_budgets.down.sql`
- `src/backend/src/services/cost_preview.rs`
- `src/backend/tests/cost_preview_tests.rs`

**Modified:**
- `src/backend/src/routes/allocation.rs` — added CostPreviewQuery, CostPreviewResponse, BudgetImpact structs, cost_preview handler, budget impact computation, route registration; (code-review) refactored crypto service instantiation out of loop (M2), replaced local `is_weekend` with import from `cost_preview` service (M5)
- `src/backend/src/services/mod.rs` — added cost_preview module export
- `src/frontend/src/pages/team.rs` — added preview signals, debounced effect, CostPreviewResponse struct, fetch_cost_preview fn, preview panel UI, confirmation summary, budget health helpers, state cleanup; (code-review) enhanced confirmation summary with resource name, daily rate, project name, dates, allocation% (H1); moved summary above button flex container (M3)
- `src/backend/src/services/cost_preview.rs` — (code-review) made `is_weekend` public for reuse (M5)
- `src/backend/tests/cost_preview_tests.rs` — (code-review) added 3 integration tests: budget health thresholds, committed increase after allocation, BUDGET_OVERRUN_POLICY=block requires_approval (H2)

**Not in Story scope (pre-existing change from Story 3.2, noted for traceability):**
- `src/backend/src/routes/project.rs` — adds `get_assignable_projects` handler (+73 lines, from Story 3.2, uncommitted) (M1)
### Change Log

| Date | Change |
|------|--------|
| 2026-03-01 | Story context created via create-story workflow |
| 2026-03-02 | Validation review applied — 16 improvements (C1-C5, E1-E5, O1-O3, L1-L3) |
| 2026-03-02 | Backend Tasks 1-3, 6 implemented: migration, cost_preview service, endpoint handler with CTC decryption + budget impact, 5 integration tests passing, 18 assignment tests green |
| 2026-03-02 | Frontend Tasks 4-5 implemented: preview panel with debounced effect, monthly breakdown, budget impact progress bar, formula tooltip, warning banner, confirmation summary, state cleanup |
| 2026-03-02 | Story status → review; all ACs satisfied, all tasks checked |
| 2026-03-02 | Code-review fixes: H1 confirmation summary enriched, H2 three budget integration tests added, M2 crypto per-row fix, M3 layout fix, M4 documented as limitation, M5 is_weekend shared |
| 2026-03-02 | Story status → done; all HIGH/MEDIUM issues resolved, all ACs satisfied |
