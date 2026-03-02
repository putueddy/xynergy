# Story 3.4: Overallocation Warnings

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **Department Head**,
I want **to receive warnings when assignments would exceed 100% capacity**,
so that **I avoid overcommitting my team members**.

## Acceptance Criteria

1. **Given** an employee has existing allocations totaling 80% **when** I attempt to add a 30% allocation **then** the system shows a warning: `Total allocation would be 110% - confirm over-allocation?`.
2. **Given** an employee has allocations exceeding 100% **when** I view the team dashboard **then** the employee is highlighted with `Overallocated` status and total allocation shown in red.
3. **Given** I view the department capacity report **when** I select a date range **then** I see utilization percentage per employee over time and overallocation periods are visually highlighted.

## Scope Boundary

- **In scope**: Warning-first over-allocation flow, explicit user confirmation path, overallocated dashboard state, date-range utilization view for department capacity.
- **NOT in scope**: Auto-rebalancing suggestions, optimizer recommendations, cross-department staffing simulations, or Epic 4 budget/P&L forecasting logic.

## Tasks / Subtasks

- [x] **Task 1: Extend assignment creation with warning-first overallocation validation** (AC: #1)
  - [x] In `src/backend/src/routes/allocation.rs`, extend the existing `create_allocation` handler to compute the resource's total allocation percentage for the proposed date range BEFORE persisting.
  - [x] Compute `current_allocation_sum` by querying all active allocations for the resource where date ranges overlap with the proposed `(start_date, end_date)`. Use date-aware overlap logic: `WHERE resource_id = $1 AND start_date < $2 AND end_date > $3` (where $2 = proposed end_date, $3 = proposed start_date).
  - [x] Add `#[serde(default)] pub confirm_overallocation: bool` to the existing `CreateAllocationRequest` struct for backward compatibility.
  - [x] Validation rule: if `current_allocation_sum + requested_percentage > 100.0` and `confirm_overallocation == false`, return `CreateAllocationResult::OverallocationWarning(...)` with projected totals. If `confirm_overallocation == true`, allow creation and log overallocation confirmation audit event.
  - [x] Preserve ALL existing validation guards: `ensure_allocation_access()`, assignable-project checks, date-order checks, project date bounds, and CTC presence checks from Stories 3.2/3.3. These must run BEFORE the overallocation check.
  - [x] Change handler return type from `Result<Json<AllocationResponse>>` to `Result<Json<CreateAllocationResult>>` (see [API Response Types](#api-response-types)).

- [x] **Task 2: Add deterministic API contract for warning and confirmation flow** (AC: #1)
  - [x] Define `OverallocationWarningResponse` DTO in `allocation.rs` with fields: `resource_id`, `resource_name`, `current_allocation_percentage`, `requested_allocation_percentage`, `projected_allocation_percentage`, `warning_message`, `requires_confirmation` (see [API Response Types](#api-response-types) for exact struct).
  - [x] Define `CreateAllocationResult` enum with `#[serde(tag = "status")]` discriminator so frontend can branch on `response.status` field.
  - [x] Warning message format: `"Total allocation would be {projected}% — confirm over-allocation?"` — deterministic, UX-aligned.
  - [x] Keep error/warning envelope consistent with existing `AppError` and JSON response conventions. Overallocation warning is NOT an `AppError`; it is a successful HTTP 200 response with `status: "overallocation_warning"`.

- [x] **Task 3: Surface overallocated state in team dashboard** (AC: #2)
  - [x] Extend team query in `src/backend/src/routes/team.rs` and/or `src/backend/src/services/team_service.rs` to compute `current_allocation_percentage` per employee.
  - [x] Allocation sum query must be date-aware: sum `allocation_percentage` for all allocations where `start_date <= CURRENT_DATE AND end_date >= CURRENT_DATE` (current period overlap).
  - [x] Add `current_allocation_percentage: f64` and `is_overallocated: bool` fields to the team member response DTO.
  - [x] `is_overallocated = current_allocation_percentage > 100.0`.
  - [x] Never expose CTC component fields (`base_salary`, `hra_allowance`, `bpjs_*`, `thr_*`) in team responses — only `blended_daily_rate` (already exposed since Story 3.1).
  - [x] Use a single SQL query with `LEFT JOIN` and `SUM` + `GROUP BY` on allocations to avoid N+1 per employee.

- [x] **Task 4: Add department capacity report endpoint with date-range utilization** (AC: #3)
  - [x] Add `GET /api/v1/team/capacity-report?start_date=YYYY-MM-DD&end_date=YYYY-MM-DD` endpoint in `team.rs` or `team_service.rs`, scoped to the requesting user's department via `user_claims_from_headers(&headers)?`.
  - [x] Use `Query<CapacityReportQuery>` extractor with `start_date: NaiveDate` and `end_date: NaiveDate` query parameters.
  - [x] Return `CapacityReportResponse` with per-employee, per-period (monthly, keyed as `YYYY-MM`) utilization data (see [API Response Types](#api-response-types)).
  - [x] For each employee-period bucket, compute `total_allocation_percentage` by summing allocation percentages with date-range overlap in that month. Set `is_overallocated = total_allocation_percentage > 100.0`.
  - [x] Performance: Rust-side month bucketing with SQL date-overlap query. Avoid N+1 queries per employee per month.
  - [x] Reuse holiday/weekend handling conventions from allocation/cost-preview logic for consistent working-day treatment if utilization needs working-day normalization.
  - [x] Register route within existing `team_routes()` — no new route module needed.

- [x] **Task 5: Update Team UI with warning confirm flow and overallocated dashboard styling** (AC: #1, #2, #3)
  - [x] In `src/frontend/src/pages/team.rs`, add reactive signals for overallocation state:
    ```rust
    let (overallocation_warning, set_overallocation_warning) = create_signal(None::<OverallocationWarningResponse>);
    let (show_confirm_overallocation, set_show_confirm_overallocation) = create_signal(false);
    let (confirm_submitting, set_confirm_submitting) = create_signal(false);
    ```
  - [x] Modify assignment submit flow: on POST response, check `status` field:
    - If `"created"` → existing success flow (refresh team data, clear form, show toast).
    - If `"overallocation_warning"` → store warning data in `set_overallocation_warning`, show confirmation modal via `set_show_confirm_overallocation(true)`.
  - [x] Confirmation modal shows: current allocation %, requested %, projected %, and the `warning_message` from response. Two buttons: "Confirm Over-Allocation" (re-submits POST with `confirm_overallocation: true`) and "Cancel" (closes modal, returns to form).
  - [x] Render `Overallocated` status badge for employees where `is_overallocated == true` in team list. Badge: red background (`bg-red-100 text-red-700`), text "Overallocated". Show `current_allocation_percentage` in red (`text-red-600`) using JetBrains Mono font.
  - [x] Add capacity report UI section below team list with date-range filter, results table with color-coded cells (green ≤80%, yellow 80-100%, red >100%).
  - [x] Fetch from `GET /api/v1/team/capacity-report` on filter change (debounce 300ms, same pattern as 3.3 preview debounce).
  - [x] Clear overallocation modal state on modal open (same reset pattern as existing `set_show_assign_modal` open handler).

- [x] **Task 6: Add integration and regression tests** (AC: #1, #2, #3)
  - [x] Backend tests: POST with allocation exceeding 100% and `confirm_overallocation=false` returns `status: "overallocation_warning"` with correct projected percentage.
  - [x] Backend tests: POST with `confirm_overallocation=true` creates the allocation even when >100%, returns `status: "created"`.
  - [x] Backend tests: POST with allocation ≤100% creates normally regardless of `confirm_overallocation` flag.
  - [x] Backend tests: team list endpoint returns `is_overallocated=true` and correct `current_allocation_percentage` for resources with overlapping allocations summing >100%.
  - [x] Backend tests: capacity report returns per-employee per-period utilization with correct `is_overallocated` flags for overallocated periods.
  - [x] Backend tests: verify date-aware overlap — employee with 60% allocation Jan-Mar and 50% allocation Feb-Apr shows 110% in Feb-Mar but 60% in Jan and 50% in Apr.
  - [x] Frontend behavior tests/manual verification: warning modal appears on overallocation, confirm creates assignment, cancel aborts, dashboard highlights update.
  - [x] Keep all prior suites green: `assignment_tests.rs`, `cost_preview_tests.rs`, `team_tests.rs`.

## Dev Notes

### Developer Context (Critical)

- Story 3.3 implemented cost preview, department budget impact indicators, and confirmation summary patterns. Story 3.4 extends the assignment flow with an overallocation check that runs in the same `create_allocation` handler — do not introduce a parallel assignment endpoint.
- Story 3.2/3.3 established assignment create parity and deterministic validation messaging. The existing `CreateAllocationRequest` struct gets one new field (`confirm_overallocation`); all other fields remain unchanged.
- `src/backend/src/routes/allocation.rs` has 7 handlers: `get_allocations`, `get_allocations_by_project`, `get_allocations_by_resource`, `create_allocation`, `update_allocation`, `delete_allocation`, plus `allocation_routes()`. The `ensure_allocation_access()` guard runs on all mutation handlers. **No overallocation logic exists yet** — warning-first flow is entirely new.
- `src/frontend/src/pages/team.rs` has 10+ reactive signals for assignment state (`show_assign_modal`, `assign_resource_id`, `assign_project_id`, `assign_start_date`, `assign_end_date`, `assign_pct`, `assign_error`, `assign_success`, `assign_submitting`, `assignable_projects`) plus 3 preview signals from Story 3.3 (`preview_data`, `preview_loading`, `preview_error`). Story 3.4 adds 3 overallocation signals within this established pattern.
- `src/backend/src/services/cost_preview.rs` (from Story 3.3) contains working-day computation and monthly bucketing — reuse the `is_weekend()` function and date iteration pattern for capacity report if working-day normalization is needed.
- `department_budgets` table (from Story 3.3 migration) stores budget caps per department-period. Budget impact from 3.3 is separate from allocation capacity in 3.4 — capacity is about percentage utilization, budget is about cost. Do not conflate them.

### Dev Guardrails

**RBAC & Auth:**
- Extract user claims via `user_claims_from_headers(&headers)?` (returns full JWT Claims with role) — defined in `src/backend/src/services/audit_log.rs:164`. Use for role-based access on capacity report endpoint.
- Extract just user ID via `user_id_from_headers(&headers)?` — defined at `audit_log.rs:189`. Use for audit logging.
- Reuse `ensure_allocation_access()` from `allocation.rs` for all allocation mutation access control. Do not duplicate role-checking logic.
- Capacity report auto-scopes to user's department from JWT claims. Do not accept `department_id` as query param — derive from auth context.

**SQL & Data Safety:**
- Keep SQL parameterized with `sqlx` query macros; no dynamic SQL string concatenation.
- Allocation percentage stored as `f64` in Rust / `DECIMAL` or `DOUBLE PRECISION` in SQL. Use `BigDecimal` patterns from project conventions only for precise IDR money calculations — percentage comparisons can use `f64` safely.
- Date-aware overlap query pattern: `WHERE resource_id = $1 AND start_date < $2 AND end_date > $3` (proposed_end, proposed_start). This correctly finds all allocations that overlap with the proposed date range.
- Explicit type casts for non-standard types: `SELECT date::TEXT as "date!" FROM holidays`.
- Use `fetch_optional` + `ok_or_else` for single-record lookups. Use `fetch_all` for list queries.

**Architecture:**
- Keep route handlers thin; place allocation-sum and capacity computations in `team_service.rs` or a helper function, not inline in the handler.
- Register new routes within existing `allocation_routes()` and `team_routes()` functions — no new route modules.
- Follow existing error envelope (`AppError` mapping) for hard errors. Overallocation warning is a successful (HTTP 200) tagged response, not an `AppError`.
- Follow existing DTO conventions: `Create...Request` for inputs, `...Response` for outputs, `#[derive(Debug, Serialize)]` on responses, `#[derive(Debug, Deserialize)]` on requests.

**Audit & Logging:**
- Log overallocation confirmation events: when `confirm_overallocation=true` and allocation >100%, create audit entry with `entity_type="allocation"`, `action="overallocation_confirmed"`, `changes={"current_total": X, "requested": Y, "projected_total": Z, "confirmed_by": user_id}`.
- Standard audit logging for assignment creation (existing) continues to apply.
- Do not add per-request audit logging for team list or capacity report reads — these are read-only and already covered by general API access logging.

### API Response Types

**`CreateAllocationResult`** — returned by `POST /api/v1/allocations`:
```rust
#[derive(Debug, Serialize)]
#[serde(tag = "status")]
pub enum CreateAllocationResult {
    #[serde(rename = "created")]
    Created {
        allocation: AllocationResponse,  // existing type from 3.2
    },
    #[serde(rename = "overallocation_warning")]
    OverallocationWarning(OverallocationWarningResponse),
}
```

**`OverallocationWarningResponse`** — warning payload when confirmation needed:
```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OverallocationWarningResponse {
    pub resource_id: Uuid,
    pub resource_name: String,
    pub current_allocation_percentage: f64,
    pub requested_allocation_percentage: f64,
    pub projected_allocation_percentage: f64,
    pub warning_message: String,      // "Total allocation would be 110% — confirm over-allocation?"
    pub requires_confirmation: bool,  // always true when this variant is returned
}
```

**Team member response extension** — add to existing team member DTO:
```rust
// Add these fields to existing team member response struct
pub current_allocation_percentage: f64,  // sum of overlapping allocations for current date
pub is_overallocated: bool,              // current_allocation_percentage > 100.0
```

**`CapacityReportResponse`** — returned by `GET /api/v1/team/capacity-report`:
```rust
#[derive(Debug, Serialize)]
pub struct CapacityReportResponse {
    pub start_date: String,             // "YYYY-MM-DD"
    pub end_date: String,               // "YYYY-MM-DD"
    pub employees: Vec<EmployeeCapacity>,
}

#[derive(Debug, Serialize)]
pub struct EmployeeCapacity {
    pub resource_id: Uuid,
    pub resource_name: String,
    pub periods: Vec<CapacityPeriod>,
}

#[derive(Debug, Serialize)]
pub struct CapacityPeriod {
    pub period: String,                    // "YYYY-MM"
    pub total_allocation_percentage: f64,
    pub is_overallocated: bool,            // total_allocation_percentage > 100.0
    pub allocation_count: i32,             // number of active allocations in this period
}
```

**`CapacityReportQuery`** — query params for capacity report:
```rust
#[derive(Debug, Deserialize)]
pub struct CapacityReportQuery {
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
}
```

### Library / Framework Requirements

- Use currently pinned project versions:
  - `axum = 0.7`
  - `sqlx = 0.7`
  - `leptos = 0.6`
  - `bigdecimal = 0.4`
  - `chrono = 0.4`
  - `serde = 1.0` (with `derive` feature)
- Web research note: newer major releases are visible in ecosystem channels, but this story must stay on pinned versions to avoid cross-epic upgrade risk.
- Frontend: Leptos 0.6 with `spawn_local`/reactive signal pattern currently used in Team page. Use `gloo-timers` for debounce timer (already added in Story 3.3).

### File Structure Requirements

- Backend touchpoints:
  - `src/backend/src/routes/allocation.rs` — extend `CreateAllocationRequest`, `create_allocation` handler, add `CreateAllocationResult` enum, `OverallocationWarningResponse` struct
  - `src/backend/src/routes/team.rs` — extend team member response with `current_allocation_percentage` + `is_overallocated`, add `capacity_report` handler and route
  - `src/backend/src/services/team_service.rs` — add allocation-sum query helper and capacity report computation
  - `src/backend/src/services/mod.rs` (if new service module introduced)
  - `src/backend/tests/assignment_tests.rs` — existing tests must stay green
  - `src/backend/tests/team_tests.rs` — existing tests must stay green
  - `src/backend/tests/overallocation_tests.rs` (new, recommended)
- Frontend touchpoints:
  - `src/frontend/src/pages/team.rs` — overallocation signals, warning modal, overallocated badge, capacity report UI section
  - `src/frontend/src/components/` (if extracting reusable warning/capacity components)

### Testing Requirements

- Use backend integration pattern `#[sqlx::test(migrations = "../../migrations")]`.
- Overallocation warning tests: POST with >100% + confirm=false → warning response; POST with >100% + confirm=true → created; POST with ≤100% → created regardless of flag.
- Date-aware overlap tests: verify capacity sum considers only overlapping date ranges (e.g., 60% Jan-Mar + 50% Feb-Apr = 110% in Feb-Mar only).
- Team dashboard tests: verify `is_overallocated` computed correctly for current-date overlap.
- Capacity report tests: verify per-employee per-period utilization with correct overallocation flags across multi-month ranges.
- Regression: all prior suites green (`assignment_tests.rs`, `cost_preview_tests.rs`, `team_tests.rs`).

### Previous Story Intelligence (3.3)

- Story 3.3 introduced `cost_preview.rs` service, `department_budgets` table, budget health thresholds (green/yellow/red), and confirmation summary patterns. Extend those patterns — do not fork parallel flows.
- Story 3.3's cost preview debounce (300ms, `gloo_timers::callback::Timeout`) is the pattern to reuse for capacity report filter changes.
- Story 3.3's `CostPreviewResponse` and `BudgetImpact` structs are separate from 3.4's `OverallocationWarningResponse` — they solve different problems (cost/budget vs capacity/utilization).
- 3.3 code-review fixes: crypto service created once before loop (M2), `is_weekend` exported from `cost_preview.rs` for reuse (M5), confirmation summary with resource name + rate + project + dates (H1). Follow these established patterns.
- Known limitation from 3.3 (M4): all-or-nothing budget period matching — 3.4's capacity report is separate and does not depend on budget data.

### Project Structure Notes

- Continue extending existing Epic 3 modules (`allocation.rs`, `team.rs`, `team_service.rs`) instead of adding parallel subsystems.
- No schema changes required — overallocation is computed from existing `allocations` table data. No new migration needed for this story.
- **Scope fence**: Do not leak into Epic 4 P&L territory. Capacity utilization here is about allocation percentage, not cost/budget utilization (which is Story 3.3/3.5 territory).

### References

- [Source: `_bmad-output/planning-artifacts/epics.md#Story 3.4: Overallocation Warnings`]
- [Source: `_bmad-output/planning-artifacts/epics.md#Epic 3: Department Resource Assignment`]
- [Source: `_bmad-output/planning-artifacts/prd.md#FR12 — overallocation warnings`]
- [Source: `_bmad-output/planning-artifacts/prd.md#FR47 — team utilization dashboard`]
- [Source: `_bmad-output/planning-artifacts/architecture.md#API Design Decisions`]
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md#Experience Mechanics — color thresholds, confirmation modal`]
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md#Visual Design Foundation — semantic colors, badge patterns`]
- [Source: `_bmad-output/project-context.md#Critical Implementation Rules`]
- [Source: `_bmad-output/implementation-artifacts/3-3-cost-impact-preview.md`]
- [Source: `src/backend/src/routes/allocation.rs` — 7 handlers, ensure_allocation_access(), CreateAllocationRequest]
- [Source: `src/backend/src/services/audit_log.rs` — user_claims_from_headers (line 164), user_id_from_headers (line 189)]
- [Source: `src/backend/src/services/cost_preview.rs` — is_weekend(), working-day computation]
- [Source: `src/frontend/src/pages/team.rs` — assignment modal, 13+ signals, format_idr() helper]

### Story Creation Completion Note

- Ultimate context engine analysis completed — comprehensive developer guide created with concrete Rust types, date-aware overlap specifications, and LLM-optimized structure.

## Dev Agent Record

### Agent Model Used

anthropic/claude-opus-4-6

### Debug Log References

- Workflow source: `_bmad/bmm/workflows/4-implementation/create-story/workflow.yaml`
- Workflow instructions: `_bmad/bmm/workflows/4-implementation/create-story/instructions.xml`
- Validation checklist: `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- Sprint tracking source: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Implementation workflow: `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`

### Completion Notes List

- Story auto-selected from sprint backlog order: `3-4-overallocation-warnings`.
- Core context synthesized from epics, PRD, architecture, UX, project context, previous story (3.3), and recent git history.
- Story keeps strict scope fence to Epic 3 warning/capacity behavior and avoids Epic 4 financial forecasting scope.
- Concrete Rust response types provided (`CreateAllocationResult`, `OverallocationWarningResponse`, `CapacityReportResponse`, `EmployeeCapacity`, `CapacityPeriod`) to eliminate dev agent ambiguity.
- Date-aware overlap logic specified with exact SQL WHERE clause pattern for allocation percentage summation.
- Auth function references documented with file locations (`audit_log.rs:164`, `:189`) and `ensure_allocation_access()` reuse requirement.
- Capacity report endpoint fully specified: route, query params, response shape, performance guidance.
- Frontend signals explicitly named; warning modal flow and capacity report UI coloring patterns defined.
- Dev Notes consolidated into Dev Guardrails structure matching Story 3.3's validated pattern.
- All 6 tasks implemented and tested. 43/43 story-relevant backend tests pass (7 overallocation + 18 assignment + 10 team + 8 cost_preview).
- Pre-existing failure in `audit_tests.rs:177` (`test_ctc_view_and_mutation_audit`) confirmed unrelated to Story 3.4 — last modified in Story 2.4 (commit `9e6f561`), no changes made by this story.
- Team query date filtering uses `start_date <= CURRENT_DATE AND end_date >= CURRENT_DATE` — tests use `current_spanning_allocation_dates()` helper to ensure allocations span today.
- Capacity report uses Rust-side month bucketing (not SQL `generate_series`) — allocations fetched by date overlap, then iterated across month buckets in `team_service.rs`.
- `CreateAllocationResult` enum with `#[serde(tag = "status")]` discriminator replaces previous `Result<Json<AllocationResponse>>` return type.
- `cargo fmt` applied; all formatting clean.
- **Code-review fixes applied**: H1 frontend compilation error (i32→u32 cast in `current_month_range()`), M1 removed guardrail-violating audit logs from read-only team/capacity-report endpoints, M2 added `overallocation_confirmation_creates_audit_entry` test, L1 removed unreachable branch in `allocation_color()`.

### File List

- `_bmad-output/implementation-artifacts/3-4-overallocation-warnings.md` — this story file
- `src/backend/src/routes/allocation.rs` — extended with `confirm_overallocation` field, `CreateAllocationResult` enum, `OverallocationWarningResponse` DTO, overallocation warning logic in `create_allocation` handler, `format_percentage()`, `get_resource_name()`, `get_current_allocation_percentage()` helpers
- `src/backend/src/routes/team.rs` — added `GET /team/capacity-report` endpoint with `CapacityReportQuery`, role checks, department scoping (audit logging removed per story guardrail)
- `src/backend/src/services/team_service.rs` — extended `TeamMemberResponse` with `current_allocation_percentage` and `is_overallocated` fields; added `CapacityReportResponse`, `EmployeeCapacity`, `CapacityPeriod` DTOs; added `get_capacity_report_in_transaction()` with month bucketing
- `src/frontend/src/pages/team.rs` — added overallocation warning modal, confirm-overallocation flow, `Overallocated` badge with red styling, capacity report section with date filters and color-coded cells, `OverallocationWarning`/`CapacityReportResponse`/`EmployeeCapacity`/`CapacityPeriod` models
- `src/backend/tests/overallocation_tests.rs` — **NEW**: 7 integration tests covering warning flow, confirmation, audit entry verification, team overallocation state, capacity report, under-100% normal flow, and date-aware overlap accuracy
- `src/backend/tests/assignment_tests.rs` — updated `capacity_over_100_percent_returns_warning` and `dept_head_can_create_assignment` for new `CreateAllocationResult` response shape

### Change Log

| Date | Change | Files |
|------|--------|-------|
| 2026-03-02 | Task 1-2: Warning-first overallocation validation and API contract | `allocation.rs` |
| 2026-03-02 | Task 3: Team dashboard `is_overallocated` and `current_allocation_percentage` | `team_service.rs` |
| 2026-03-02 | Task 4: Department capacity report endpoint | `team.rs`, `team_service.rs` |
| 2026-03-02 | Task 5: Frontend overallocation UX — modal, badges, capacity report | `pages/team.rs` |
| 2026-03-02 | Task 6: Integration tests (6 new) + regression test updates | `overallocation_tests.rs`, `assignment_tests.rs` |
| 2026-03-02 | Story finalization — `cargo fmt`, story file updates, status → review | `3-4-overallocation-warnings.md`, `sprint-status.yaml` |
| 2026-03-02 | Code-review fixes — H1 frontend type error, M1 audit log removal, M2 audit test, L1 dead code | `team.rs`, `pages/team.rs`, `overallocation_tests.rs` |
