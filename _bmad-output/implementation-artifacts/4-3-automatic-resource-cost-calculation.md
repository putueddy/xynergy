# Story 4.3: Automatic Resource Cost Calculation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **Project Manager**,
I want **resource costs to be calculated automatically from allocations**,
so that **I don't need to manually compute costs**.

## Acceptance Criteria

1. **Given** resources are assigned to my project **when** I view the project dashboard **then** I see a "Resource Costs" section with: Employee, Daily Rate, Days Allocated, Total Cost.
2. **Given** an assignment spans multiple months **when** the system calculates costs **then** it prorates by working days in each month.
3. **Given** an assignment allocation is less than 100% **when** costs are calculated **then** the amount is `(daily_rate x days x allocation%)`.
4. **Given** a resource's CTC changes mid-project **when** costs are recalculated **then** the system applies pro-rata for the change period **and** displays a note about the rate change.

## Scope Boundary

- **In scope**: project-level automatic resource cost aggregation from allocations, per-employee cost rows, monthly pro-rata breakdown, allocation-percentage-aware math, CTC mid-period rate-change handling, project dashboard integration, and budget rollup integration for `spent_to_date_idr`.
- **Not in scope**: manual revenue entry (Story 4.4), full P&L dashboard and charts (Story 4.5), forecasting (Story 4.6), department budget features (Epic 3), cash-flow reporting (Epic 5), custom cost category administration, multi-currency support.
- **No new migration required** — this story computes from existing tables: `allocations`, `ctc_records`, `ctc_revisions`, `holidays`, `project_expenses`, `projects`.

## Tasks / Subtasks

- [x] **Task 0: Make shared helper functions `pub`** (prerequisite for Tasks 1-2)
  - [x] In `src/backend/src/services/budget_service.rs`, change visibility to `pub` for:
    - `extract_daily_rate_from_allocation_row()` (line ~174)
    - `bigdecimal_to_i64_trunc()` (line ~143)
    - `parse_json_decimal()` (line ~153)
    - `load_holidays()` (line ~225)
  - [x] Verify existing callers still compile after visibility change.
  - [x] Run `cargo sqlx prepare` if any query changes are needed.

- [x] **Task 1: Add project resource cost aggregation service** (AC: #1, #2, #3, #4)
  - [x] Create `src/backend/src/services/project_cost_service.rs` with entry point:
    ```rust
    pub async fn compute_project_resource_costs(
        pool: &PgPool,
        project_id: Uuid,
    ) -> Result<ProjectResourceCostResult>
    ```
  - [x] Query allocations for the project using this adapted pattern from `budget_service.rs:273-299`:
    ```sql
    SELECT a.resource_id, a.start_date, a.end_date,
           a.allocation_percentage, a.include_weekend,
           r.name AS resource_name,
           c.daily_rate, c.encrypted_daily_rate, c.key_version,
           c.encryption_version, c.encryption_algorithm, c.encrypted_at
    FROM allocations a
    JOIN resources r ON r.id = a.resource_id
    LEFT JOIN LATERAL (
       SELECT daily_rate, encrypted_daily_rate, key_version,
              encryption_version, encryption_algorithm, encrypted_at
       FROM ctc_records c
       WHERE c.resource_id = a.resource_id AND c.status = 'Active'
       ORDER BY c.effective_date DESC, c.updated_at DESC
       LIMIT 1
    ) c ON TRUE
    WHERE a.project_id = $1
    ```
  - [x] Load holidays via `pub load_holidays()` from `budget_service.rs` (made `pub` in Task 0).
  - [x] Extract daily rate via `pub extract_daily_rate_from_allocation_row()` from `budget_service.rs`.
  - [x] Extract `include_weekend: bool` from each allocation row — required by `calculate_cost_preview()`.
  - [x] Reuse `calculate_cost_preview()` from `cost_preview.rs` for canonical day counting and monthly bucketing; do not reimplement formula logic.
  - [x] Aggregate per employee rows with: `resource_id`, `resource_name`, `daily_rate_idr`, `days_allocated`, `allocation_percentage`, `total_cost_idr`.
  - [x] Return monthly breakdown (`YYYY-MM`) and project total resource cost.
  - [x] **Missing CTC handling**: if a resource has no CTC record (encrypted or plaintext), include the employee row in the response with `daily_rate_idr: None`, `total_cost_idr: 0`, and `missing_rate: true`. Do NOT skip silently, do NOT fail the entire request. The frontend should render a "Rate unavailable" indicator for that row.

- [x] **Task 2: Implement CTC effective-date pro-rata logic for mid-project rate changes** (AC: #4)
  - [x] **CRITICAL SCHEMA CONTEXT**:
    - `ctc_records` has `resource_id UUID PRIMARY KEY` — stores only the **latest** rate per resource (1:1).
    - `ctc_revisions` is the **append-only rate timeline** (1:many per resource) with `revision_number`, `effective_date`, `encrypted_daily_rate` (nullable), and `encrypted_components` (TEXT, always present).
    - For rate history, query `ctc_revisions` — NOT `ctc_records`.
  - [x] Query rate timeline for each resource:
    ```sql
    SELECT revision_number, effective_date, encrypted_daily_rate,
           encrypted_components, key_version, encryption_version,
           encryption_algorithm, encrypted_at
    FROM ctc_revisions
    WHERE resource_id = $1
    ORDER BY effective_date ASC
    ```
  - [x] Build rate windows from the revision timeline:
    1. For each revision, the rate is effective from `effective_date` until the next revision's `effective_date - 1` (or allocation end).
    2. Extract daily rate: try `encrypted_daily_rate` first; if NULL, decrypt `encrypted_components` JSON blob and extract `daily_rate` field from it.
    3. Use the same `DefaultCtcCryptoService` + `EnvKeyProvider` pattern plus `parse_json_decimal()` and `bigdecimal_to_i64_trunc()` from `budget_service.rs`.
  - [x] For each allocation, split into time segments at CTC revision boundaries and month boundaries, then call `calculate_cost_preview()` per segment with that segment's rate.
  - [x] Add `rate_change_note`/`has_rate_change` metadata when multiple rates are applied within the allocation period.
  - [x] **Fallback**: if a resource has zero revisions in `ctc_revisions`, fall back to the single-rate path using `ctc_records` (the default `LATERAL JOIN` from Task 1). This handles resources that predate the revision system.

- [x] **Task 3: Expose project resource cost API endpoint** (AC: #1, #2, #3, #4)
  - [x] Add `GET /api/v1/projects/:id/resource-costs` in `src/backend/src/routes/project.rs`.
  - [x] Reuse `enforce_expense_access()` (line ~792 in `project.rs`) for auth — it already enforces PM-owns-project or admin. If the function name is too expense-specific, rename it to `enforce_project_mutation_access()` and update expense callers.
  - [x] Audit-log access denials with `ACCESS_DENIED` entity type `"project_resource_costs"` and include project ID/action payload.
  - [x] Response DTOs (add in `project.rs` or dedicated module):
    ```rust
    #[derive(Debug, Serialize)]
    pub struct ProjectResourceCostResponse {
        pub project_id: Uuid,
        pub total_resource_cost_idr: i64,
        pub employees: Vec<EmployeeResourceCost>,
        pub monthly_breakdown: Vec<MonthlyResourceCost>,
    }

    #[derive(Debug, Serialize)]
    pub struct EmployeeResourceCost {
        pub resource_id: Uuid,
        pub resource_name: String,
        pub daily_rate_idr: Option<i64>,
        pub days_allocated: i32,
        pub allocation_percentage: f64,
        pub total_cost_idr: i64,
        pub has_rate_change: bool,
        pub rate_change_note: Option<String>,
        pub missing_rate: bool,
    }

    #[derive(Debug, Serialize)]
    pub struct MonthlyResourceCost {
        pub month: String,      // "YYYY-MM"
        pub working_days: i32,
        pub cost_idr: i64,
    }
    ```

- [x] **Task 4: Integrate resource costs into project budget `spent_to_date_idr`** (AC: #1, #3)
  - [x] Update `get_project_budget()` (~line 570 in `project.rs`) and `set_project_budget()` (~line 650):
    - Keep the existing expense SQL: `COALESCE(SUM(amount_idr), 0) FROM project_expenses WHERE project_id = $1` → assign to `expense_total_idr`.
    - Call `compute_project_resource_costs(pool, project_id).await?` → extract `.total_resource_cost_idr` → assign to `resource_total_idr`.
    - Set `spent_to_date_idr = expense_total_idr + resource_total_idr`.
  - [x] This is the **compute-on-the-fly** approach (no caching table), consistent with department budget utilization pattern in `budget_service.rs`. If performance becomes a concern for large projects, caching can be added in a future story.
  - [x] Keep response backward-compatible; only `spent_to_date_idr` and `remaining_idr` values change. Do not remove/rename existing keys.
  - [x] **Double-count prevention**: resource costs come exclusively from allocations; non-resource costs come exclusively from `project_expenses`. These are disjoint data sources. Verify by test.

- [x] **Task 5: Add frontend "Resource Costs" section to Projects page** (AC: #1, #2, #3, #4)
  - [x] Extend `src/frontend/src/pages/projects.rs` to fetch `GET /api/v1/projects/:id/resource-costs`.
  - [x] Render table columns exactly per AC: Employee, Daily Rate, Days Allocated, Total Cost.
  - [x] Render monthly breakdown for cross-month allocations.
  - [x] Render "Rate unavailable" indicator for employees with `missing_rate: true`.
  - [x] Render a visible note/badge when `has_rate_change: true` for an employee.
  - [x] Refresh this section after allocation edits and relevant project interactions.

- [x] **Task 6: Add integration tests for resource-cost correctness and auth** (AC: #1, #2, #3, #4)
  - [x] Create `src/backend/tests/project_resource_cost_tests.rs`.
  - [x] Test base formula at 100% and <100% allocation.
  - [x] Test cross-month allocation prorating by working days.
  - [x] Test that `include_weekend: true` allocations correctly include weekend days.
  - [x] Test CTC mid-period revision causes segmented pro-rata and `has_rate_change: true` + note.
  - [x] Test resource with no CTC data returns `missing_rate: true`, `daily_rate_idr: null`, `total_cost_idr: 0`.
  - [x] Test PM ownership restrictions and admin access (reuse `enforce_expense_access` or renamed helper).
  - [x] Test `spent_to_date_idr` = expense sum + resource cost sum (no double-count).
  - [x] Regression: existing `project_budget_tests.rs`, `project_expense_tests.rs`, and `cost_preview_tests.rs` remain green.

### Review Follow-ups (AI)

- [x] [AI-Review][Critical] Fix compile regression in `src/backend/src/services/budget_service.rs:4` and `src/backend/src/services/budget_service.rs:247` (`PgPool` imported for `load_holidays_pool`; targeted backend tests now compile and pass).
- [x] [AI-Review][High] Add explicit project-existence check in `src/backend/src/routes/project.rs:1576` for `GET /projects/:id/resource-costs` (handler now returns `404` for unknown project IDs, aligned with other project endpoints).
- [x] [AI-Review][Medium] Align AC#1 table columns with story requirement in `src/frontend/src/pages/projects.rs:867` (removed extra `Allocation` column; table now shows Employee, Daily Rate, Days Allocated, Total Cost).
- [x] [AI-Review][Medium] Reconcile source-code change tracking: `migrations/20260305100000_add_project_expenses.up.sql` and `src/backend/tests/overallocation_tests.rs` are now included in the story File List.
- [x] [AI-Review][Medium] Scope cleanup for unrelated create-project behavior in `src/backend/src/routes/project.rs:482` and `src/frontend/src/components/project_form.rs:399` (removed forced `Active` + forced self-assignment behavior; restored explicit status/manager selection semantics).

## Dev Notes

### Developer Context

- Story 4.1 added project budget columns and budget endpoints in `src/backend/src/routes/project.rs`.
- Story 4.2 added `project_expenses` table, expense CRUD, and integrated non-resource costs into `spent_to_date_idr`. It introduced `enforce_expense_access()` (~line 792) as a reusable auth helper.
- Story 4.3 is the first story that must compute and surface **resource cost actuals** from allocations at project scope.
- Cost math primitives already exist in `src/backend/src/services/cost_preview.rs`; reusing them is mandatory to prevent formula drift.
- Resource costs are computed on-the-fly per request (no caching table). This mirrors the department budget utilization pattern in `budget_service.rs`.

### Dev Guardrails

**Calculation, Precision, and Data Sources**
- Use integer IDR (`i64`/`BIGINT`) for totals. Keep BigDecimal conversion pattern consistent (string parse/truncate, no float persistence).
- Do not create alternative cost formulas; use `calculate_cost_preview()` as source of truth.
- `calculate_cost_preview()` requires three parameters the story depends on:
  - `include_weekend: bool` — stored on the `allocations` table per allocation row; extract it.
  - `holidays: &[NaiveDate]` — load from `holidays` table via `budget_service::load_holidays()` (made `pub` in Task 0).
  - `allocation_percentage: f64` — stored as `DECIMAL(5,2)` on `allocations`; parse via `BigDecimal::to_string().parse::<f64>()` pattern.

**CTC Schema — CRITICAL DISAMBIGUATION**
- `ctc_records`: `resource_id` is **PRIMARY KEY** (1:1). Stores only the **latest active** rate per resource. Used for the default single-rate path.
- `ctc_revisions`: **append-only timeline** (1:many per resource). Has `revision_number`, `effective_date`, `encrypted_daily_rate` (nullable TEXT), `encrypted_components` (non-null TEXT). Used for AC #4 rate-change time-slicing.
- When `ctc_revisions.encrypted_daily_rate` is NULL for a revision, decrypt `encrypted_components` JSON blob and extract the `daily_rate` field from it.
- If a resource has zero revisions in `ctc_revisions`, fall back to single rate from the `ctc_records` LATERAL JOIN.

**Missing CTC Handling**
- If a resource has no CTC record at all (neither `ctc_records` nor `ctc_revisions`), do NOT fail the entire endpoint and do NOT silently return zero. Instead, include the employee in the response with `daily_rate_idr: None`, `total_cost_idr: 0`, `missing_rate: true`. The frontend renders "Rate unavailable" for that row.

**Authorization and Audit**
- Reuse `enforce_expense_access()` from `project.rs` (~line 792) for auth. If the name is too specific, rename to `enforce_project_mutation_access()` and update expense callers.
- Log `ACCESS_DENIED` on blocked access with entity type `"project_resource_costs"`.

**Authorization Pattern (from Story 4.2)**

Do NOT use `can_access_project()` — it grants finance/department_head read access. Use the PM-owns-project or admin check:
```rust
let user_id = enforce_expense_access(&pool, &headers, project_id, "get_resource_costs").await?;
```

**Performance**
- Avoid N+1 across allocations/resources/revisions; prefer bounded query sets and in-memory bucketing.
- The allocation query (Task 1) fetches all project allocations in one query. Rate-change time-slicing (Task 2) queries `ctc_revisions` once per resource that has multiple rates. This is O(resources) not O(allocations×months).
- Add indexes only if measured bottlenecks warrant it.

### Architecture Compliance

- Keep handlers thin; place aggregation and time-slicing logic in `project_cost_service.rs`.
- Use `sqlx::query!`/`query_as!` with compile-time checks where possible; use `sqlx::query()` dynamic for the LATERAL JOIN pattern (consistent with `budget_service.rs`).
- Keep API DTOs explicit and separated from DB row structs.
- Register new service module in `src/backend/src/services/mod.rs`.

### Previous Story Intelligence (4.2)

- Story 4.2 introduced `enforce_expense_access()` — reuse it for consistent auth enforcement.
- Story 4.2 established expense edit requiring `edit_reason` as non-optional `String` on update DTOs — follow the same "mandatory metadata" approach for `rate_change_note`.
- Story 4.2 validation learned that `edit_reason` should be validated as non-empty at API boundary, not just present. Apply same rigor to any required string fields.
- Reuse test helper patterns from `project_expense_tests.rs` and `project_budget_tests.rs`: `create_test_user_with_role()`, `get_auth_token()`, `create_test_project_with_pm()`.

### References (Read Before Coding — Priority Order)

1. `src/backend/src/services/cost_preview.rs` — canonical cost formula, `MonthlyBucket`, `calculate_cost_preview()` signature
2. `src/backend/src/services/budget_service.rs` — allocation query with LATERAL JOIN (~line 273), `extract_daily_rate_from_allocation_row` (~line 174), `load_holidays` (~line 225), `bigdecimal_to_i64_trunc` (~line 143)
3. `src/backend/src/routes/project.rs` — `get_project_budget` (~line 570), `set_project_budget` (~line 650), `enforce_expense_access` (~line 792), `spent_to_date_idr` expense query (~line 624)
4. `src/backend/src/routes/allocation.rs` — cost-preview endpoint, `get_holidays_in_range` (~line 635)
5. `src/backend/src/routes/ctc.rs` — CTC revision writes, encryption metadata flow
6. `migrations/20260222123816_ctc_revisions.up.sql` — `ctc_revisions` schema (effective_date, encrypted_daily_rate nullable, encrypted_components)
7. `migrations/20260222140000_extend_ctc_records.up.sql` — `ctc_records` schema (resource_id PK, daily_rate, effective_date, status)
8. `src/frontend/src/pages/projects.rs` — current budget and expense UI to extend
9. `src/backend/tests/project_budget_tests.rs` and `src/backend/tests/project_expense_tests.rs` — auth/setup/testing conventions

Planning artifacts:
- `_bmad-output/planning-artifacts/epics.md` (Story 4.3 ACs, line ~731)
- `_bmad-output/planning-artifacts/prd.md` (FR19-FR22)
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/implementation-artifacts/4-2-non-resource-cost-entry.md`

## Dev Agent Record

### Agent Model Used

anthropic/claude-opus-4-6

### Debug Log References

- Workflow source: `_bmad/bmm/workflows/4-implementation/create-story/workflow.yaml`
- Workflow instructions: `_bmad/bmm/workflows/4-implementation/create-story/instructions.xml`
- Validation checklist: `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- Sprint tracking source: `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Completion Notes List

- Story created from explicit user target `4-3`; no auto-discovery needed.
- Context synthesized from epics/PRD/architecture/UX/project-context, Story 4.1 and 4.2 artifacts, current codebase patterns, and recent commit history.
- Story status set to `ready-for-dev` with explicit implementation guardrails for formula reuse, encryption-safe rate extraction, and pro-rata effective-date handling.
- Validated via `validate-create-story` checklist. Applied 8 critical fixes (CTC schema disambiguation, helper visibility, include_weekend/holidays data sources, spent_to_date integration approach, no-migration statement, missing-CTC error handling, agent model correction), 6 enhancements (DTO definitions, auth helper reuse, project-level query pattern, compute-on-the-fly decision, pub prerequisite task, rate-window pseudocode), and 3 optimizations (line-number references, Dev Notes consolidation, References deduplication).
- Implementation completed and code review performed. Review fixes applied: **H1** — budget response now includes `resource_cost_idr` as a separate line and per-category spend/remaining breakdowns (`spent_hr_idr`, `spent_software_idr`, `spent_hardware_idr`, `spent_overhead_idr`, `remaining_*_idr`); **M1** — moved `load_holidays_from_pool` into `budget_service.rs` as `load_holidays_pool` (single source of truth, eliminates duplication in `project_cost_service.rs`); **M3** — renamed `enforce_expense_access` → `enforce_project_mutation_access` across all callers for accurate semantics.

### File List

- `_bmad-output/implementation-artifacts/4-3-automatic-resource-cost-calculation.md`
- `src/backend/src/services/project_cost_service.rs` (new — Tasks 1+2: resource cost aggregation and CTC pro-rata)
- `src/backend/src/services/budget_service.rs` (modified — Task 0: added `load_holidays_pool`)
- `src/backend/src/routes/project.rs` (modified — Tasks 3+4: resource-cost endpoint, budget integration, category breakdowns, auth rename)
- `src/backend/tests/project_resource_cost_tests.rs` (new — Task 6: resource cost integration tests)
- `src/backend/tests/project_budget_tests.rs` (modified — Task 6: budget integration tests with resource costs)
- `src/frontend/src/pages/projects.rs` (modified — Task 5: Resource Costs UI section)
- `src/frontend/src/components/project_form.rs` (modified — Task 5: project form budget display)
- `migrations/20260305100000_add_project_expenses.up.sql` (modified — UUID default aligned to `gen_random_uuid()`)
- `src/backend/tests/overallocation_tests.rs` (modified — UTC date source stabilization)

## Senior Developer Review (AI)

### Reviewer

- Putu (AI)

### Date

- 2026-03-05

### Outcome

- Approved after fixes

### Findings

- **Critical (resolved)**: Backend integration test compile issue from missing `PgPool` import in `budget_service` helper additions was fixed (`src/backend/src/services/budget_service.rs:4`, `src/backend/src/services/budget_service.rs:247`).
- **High (resolved)**: `GET /projects/:id/resource-costs` now verifies project existence (`src/backend/src/routes/project.rs:1576`) and returns `404` for unknown project IDs, aligned with other project endpoints.
- **Medium (resolved)**: Frontend table now matches AC-required columns (`src/frontend/src/pages/projects.rs:867`).
- **Medium (resolved)**: Story File List now includes additional modified files (`migrations/20260305100000_add_project_expenses.up.sql`, `src/backend/tests/overallocation_tests.rs`).
- **Medium (resolved)**: Out-of-scope create-project behavior changes were cleaned up; explicit status/manager inputs are respected (`src/backend/src/routes/project.rs:482`, `src/frontend/src/components/project_form.rs:399`).

### Validation Performed

- Ran: `cargo test --package xynergy-backend --test project_resource_cost_tests --test project_budget_tests`
- Result (initial review): failed at compile stage due `PgPool` unresolved type in `budget_service`.
- Result (post-fix): compile succeeded; `project_budget_tests` (17/17) and `project_resource_cost_tests` (13/13, includes new missing-project `404` regression test) passed.
