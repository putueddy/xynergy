# Story 4.1: Project Budget Setup

Status: review

## Story

As a **Project Manager**,
I want **to create project budgets with category breakdown**,
so that **I can track spending against planned allocations**.

## Acceptance Criteria

1. **Given** I am logged in as Project Manager **when** I navigate to "Create Project" **then** I see a form with: Project Name, Client, Start/End Date, Budget Categories.
2. **Given** I set up budget categories **when** I enter amounts for HR, Software, Hardware, Overhead **then** the system validates the total equals the project budget **and** displays category percentages.
3. **Given** I complete project setup **when** I click "Save Project" **then** the project is created with status="Active" **and** I am set as the Project Manager.
4. **Given** I view my projects **when** I select a project **then** I see the budget summary with: Total, Spent, Remaining per category.

## Scope Boundary

- **In scope**: project-level budget setup and retrieval, fixed MVP categories (HR/Software/Hardware/Overhead), strict IDR whole-number validation, PM ownership checks, project budget summary for selected project, `client` text field on projects table.
- **Not in scope**: non-resource expense ledger entry (Story 4.2), automatic allocation-to-project cost rollup (Story 4.3), revenue entry (Story 4.4), P&L dashboard (Story 4.5), forecasting (Story 4.6), multi-currency support, custom category administration, budget overrun warn/block (FR22 — deferred to Story 4.2+), architecture doc's extra categories (`materials`, `subcontractors` — deferred beyond MVP).

## Tasks / Subtasks

- [ ] **Task 1: Database migration — budget columns + client field** (AC: #1, #2, #4)
  - [ ] Create `migrations/<timestamp>_add_project_budget_columns.up.sql` with the following schema additions:
    ```sql
    ALTER TABLE projects
      ADD COLUMN IF NOT EXISTS client TEXT,
      ADD COLUMN IF NOT EXISTS total_budget_idr BIGINT NOT NULL DEFAULT 0,
      ADD COLUMN IF NOT EXISTS budget_hr_idr BIGINT NOT NULL DEFAULT 0,
      ADD COLUMN IF NOT EXISTS budget_software_idr BIGINT NOT NULL DEFAULT 0,
      ADD COLUMN IF NOT EXISTS budget_hardware_idr BIGINT NOT NULL DEFAULT 0,
      ADD COLUMN IF NOT EXISTS budget_overhead_idr BIGINT NOT NULL DEFAULT 0;

    -- Constraint: categories must sum to total
    ALTER TABLE projects ADD CONSTRAINT chk_budget_sum
      CHECK (budget_hr_idr + budget_software_idr + budget_hardware_idr + budget_overhead_idr = total_budget_idr);

    -- All budget values non-negative
    ALTER TABLE projects ADD CONSTRAINT chk_budget_nonneg
      CHECK (total_budget_idr >= 0 AND budget_hr_idr >= 0 AND budget_software_idr >= 0
             AND budget_hardware_idr >= 0 AND budget_overhead_idr >= 0);
    ```
  - [ ] Create matching `.down.sql` rollback that drops the constraints and columns.
  - [ ] Keep existing `projects` fields and API backward-compatible for all existing screens.
  - [ ] **Architecture deviation note**: architecture doc (`architecture.md` line 205) recommends `budget_settings JSONB`. This story intentionally uses typed BIGINT columns instead because: (a) database-level CHECK constraints enforce sum/non-negative invariants that JSONB cannot, (b) sqlx compile-time checking works with typed columns, (c) MVP has fixed categories so JSONB flexibility is unnecessary. This is an approved deviation.

- [ ] **Task 2: Budget API endpoints** (AC: #2, #4)
  - [ ] Add `POST /api/v1/projects/:id/budget` and `GET /api/v1/projects/:id/budget` in `src/backend/src/routes/project.rs`.
  - [ ] Request DTO:
    ```rust
    #[derive(Debug, Deserialize)]
    pub struct SetProjectBudgetRequest {
        pub total_budget_idr: i64,
        pub budget_hr_idr: i64,
        pub budget_software_idr: i64,
        pub budget_hardware_idr: i64,
        pub budget_overhead_idr: i64,
    }
    ```
  - [ ] Response DTO (exact shape — the dev agent frontend will consume this):
    ```json
    {
      "project_id": "uuid",
      "project_name": "string",
      "client": "string | null",
      "total_budget_idr": 100000000,
      "budget_hr_idr": 50000000,
      "budget_software_idr": 20000000,
      "budget_hardware_idr": 15000000,
      "budget_overhead_idr": 15000000,
      "hr_pct": 50.0,
      "software_pct": 20.0,
      "hardware_pct": 15.0,
      "overhead_pct": 15.0,
      "spent_to_date_idr": 0,
      "remaining_idr": 100000000
    }
    ```
  - [ ] Percentage fields: compute server-side as `(category_idr as f64 / total_budget_idr as f64) * 100.0`. These are display-only, never persisted.
  - [ ] `spent_to_date_idr`: hardcode to `0` for this story. Stories 4.2 (expenses) and 4.3 (resource costs) will wire this. `remaining_idr = total_budget_idr - spent_to_date_idr`.
  - [ ] Return validation errors via existing `AppError::Validation` envelope.

- [ ] **Task 3: Authorization and ownership** (AC: #1, #3, #4)
  - [ ] **Budget write** (`POST`): restrict to PM who owns the project (`is_project_manager()` from `services/rbac.rs`) OR `admin` role.
  - [ ] **Budget read** (`GET`): PM sees only own projects; `admin` sees any project (no ownership check). This matches existing `get_projects()` behavior in `routes/project.rs`.
  - [ ] Use `user_claims_from_headers()` + `can_access_project()` pattern from `services/rbac.rs`.
  - [ ] Audit-log denied access attempts with `ACCESS_DENIED` entity type.

- [ ] **Task 4: Budget validation logic** (AC: #2)
  - [ ] All values must be whole non-negative integers (`>= 0`). Reject any decimal or negative input.
  - [ ] `total_budget_idr` must be `> 0`.
  - [ ] `budget_hr_idr + budget_software_idr + budget_hardware_idr + budget_overhead_idr == total_budget_idr`. Reject with clear error message if sum doesn't match.
  - [ ] Place validation in a service helper function, not inline in the handler (thin-handler rule).

- [ ] **Task 5: Extend project form for budget setup** (AC: #1, #2, #3)
  - [ ] Extend `src/frontend/src/components/project_form.rs`:
    - Add `client` text input field (optional).
    - Add budget input fields: Total Budget, HR, Software, Hardware, Overhead.
    - Add live category percentage display (recompute on every input change).
    - Add client-side validation matching backend rules: integer-only, non-negative, sum check.
  - [ ] On "Save Project": set `project_manager_id` to current PM user ID from auth context.
  - [ ] Extend `CreateProjectRequest` and `UpdateProjectRequest` DTOs to include `client` and budget fields.
  - [ ] Update `ProjectResponse` to include `client` and budget columns.

- [ ] **Task 6: Project budget summary view** (AC: #4)
  - [ ] Extend `src/frontend/src/pages/projects.rs` with a budget summary panel for selected project.
  - [ ] Display: Total Budget, Spent (shows 0 for MVP), Remaining, and per-category breakdown with percentages.
  - [ ] PM sees only their projects; admin sees all. No CTC data leakage.

- [ ] **Task 7: Integration and regression tests** (AC: #1, #2, #3, #4)
  - [ ] Create `src/backend/tests/project_budget_tests.rs` following `budget_tests.rs` patterns:
    - PM sets budget on own project → 200 OK
    - PM sets budget on non-owned project → 403 Forbidden
    - Non-PM role attempts budget write → 403 Forbidden
    - Admin sets budget on any project → 200 OK
    - Invalid category sum (doesn't match total) → 400 Validation Error
    - Negative budget value → 400 Validation Error
    - Decimal budget value → 400 Validation Error
    - GET budget returns correct structure with `spent_to_date_idr: 0`
    - Audit log emitted on successful set/update
  - [ ] Regression: existing project CRUD routes still pass.
  - [ ] Regression: existing team/department budget endpoints unaffected.

## Dev Notes — MUST FOLLOW

### Status Casing (CRITICAL — read before writing any SQL or test)

The canonical project status value is **`'Active'`** (Title case, capital A). Evidence:
- All backend queries use `WHERE status = 'Active'` (`routes/project.rs` line 135, 147; `allocation.rs`; all test files).
- All test fixtures insert `'Active'` (7+ test files).
- Seed data uses `'planning'` (lowercase) for a different status, which is fine.
- **BUG**: `src/frontend/src/pages/dashboard.rs` line 70 filters `p.status == "active"` (lowercase) — this is wrong. Fix it to `"Active"` as part of this story's regression fix.
- When creating new projects in this story, always persist `status = 'Active'`.

### Budget Categories — Intentional MVP Scope

- MVP uses exactly 4 fixed categories: `hr`, `software`, `hardware`, `overhead`.
- The architecture doc (`architecture.md`) defines 6 categories (adds `materials`, `subcontractors`). Those are intentionally deferred beyond MVP. Do NOT add them.

### Typed Columns vs. JSONB — Architecture Deviation

- Architecture doc recommends `budget_settings JSONB` on projects table.
- This story uses typed BIGINT columns instead. Reasons: DB-level CHECK constraints, sqlx compile-time safety, fixed MVP categories. This is approved.

### Spent-to-Date MVP Proxy

- `spent_to_date_idr` returns `0` for all categories in this story.
- Story 4.2 (Non-Resource Costs) and Story 4.3 (Resource Costs) will provide actual spend data.
- Do NOT attempt to compute actual spend in this story.

### Money Handling

- All monetary values: `i64` in Rust, `BIGINT` in SQL. Whole IDR, no decimals, no floats.
- For percentage display only: `f64` is acceptable (display-only, never persisted).
- Never use `f64` for monetary persistence. Prior CTC work found BigDecimal/f64 conversion pitfalls.

### Architecture Compliance

- Thin handlers: validation/computation logic goes in service helpers, not inline in route handlers.
- Audit logging: `log_audit()` on every successful budget create/update with before/after payloads.
- Use `sqlx::query_as!` compile-time checked queries. Use `fetch_optional` + `ok_or_else` for single-row lookups.
- Reuse `user_claims_from_headers()` and `is_project_manager()` / `can_access_project()` from `services/rbac.rs`.
- Use currently pinned stack: Axum 0.7, sqlx 0.7, Leptos 0.6, BigDecimal 0.4, Rust 1.75+. No upgrades.

## Dev Notes — REFERENCE FILES

Read these before coding (priority order):
1. `src/backend/src/routes/project.rs` — existing project CRUD, extend with budget endpoints
2. `src/backend/src/services/rbac.rs` — ownership checks, reuse as-is
3. `src/backend/src/services/budget_service.rs` — department budget patterns (follow conventions, don't duplicate)
4. `src/backend/tests/budget_tests.rs` — test structure template
5. `src/frontend/src/components/project_form.rs` — extend with budget fields
6. `src/frontend/src/pages/projects.rs` — extend with budget summary

Planning artifacts (consult if ambiguous):
- `_bmad-output/planning-artifacts/epics.md` — Epic 4 story definitions
- `_bmad-output/planning-artifacts/prd.md` — FR16-22 requirements
- `_bmad-output/planning-artifacts/architecture.md` — architecture decisions (see deviations noted above)

## Dev Agent Record

### Completion Notes

- Story created for Epic 4 Story 1, validated via `validate-create-story` checklist.
- All critical issues (status casing, category scope, JSONB deviation) addressed.
- Story set to `ready-for-dev` with comprehensive implementation guardrails.

### File List

- `_bmad-output/implementation-artifacts/4-1-project-budget-setup.md`
