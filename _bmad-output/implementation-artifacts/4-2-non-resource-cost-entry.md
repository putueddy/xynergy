# Story 4.2: Non-Resource Cost Entry

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **Project Manager**,
I want **to enter non-resource costs (expenses, vendor payments)**,
so that **the total project cost includes all expenditures**.

## Acceptance Criteria

1. **Given** I am viewing a project **when** I click "Add Expense" **then** I see a form with: Category, Description, Amount, Date, Vendor (optional).
2. **Given** I enter an expense **when** I select the category **then** the dropdown shows: HR, Software, Hardware, Overhead.
3. **Given** I save an expense **when** the entry is created **then** it appears in project cost history **and** budget utilization updates immediately.
4. **Given** I need to edit an expense **when** I click "Edit" **then** I can modify details with an "Edit Reason" field **and** the change is logged for audit.

## Scope Boundary

- **In scope**: project-scoped non-resource expense CRUD, category-constrained expense entry, IDR whole-number validation, project-manager ownership checks, budget impact rollup for non-resource spend, expense edit with required reason, audit logs on create/update/delete.
- **Not in scope**: resource-cost auto-calculation logic (Story 4.3), revenue entry (Story 4.4), full P&L dashboard and forecasting (Stories 4.5/4.6), custom category administration, multi-currency support, recurring expense automation, bulk import/export.

## Tasks / Subtasks

- [ ] **Task 1: Database migration for project non-resource costs** (AC: #1, #2, #3, #4)
  - [ ] Create `migrations/<timestamp>_add_project_expenses.up.sql` with a new `project_expenses` table:
    - [ ] `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
    - [ ] `project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE`
    - [ ] `category TEXT NOT NULL` constrained to `('hr','software','hardware','overhead')`
    - [ ] `description TEXT NOT NULL`
    - [ ] `amount_idr BIGINT NOT NULL` with `CHECK (amount_idr > 0)`
    - [ ] `expense_date DATE NOT NULL`
    - [ ] `vendor TEXT NULL`
    - [ ] `created_by UUID REFERENCES users(id)`
    - [ ] `created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP`
    - [ ] `updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP`
    - [ ] Note: `edit_reason` is NOT stored on this table. It is captured only in the audit log `details` JSONB on update operations.
  - [ ] Add indexes: `(project_id)`, `(project_id, expense_date DESC)`, `(project_id, category)`.
  - [ ] Create matching `.down.sql` rollback dropping indexes then table.
  - [ ] Keep migration idempotency and environment safety (`IF NOT EXISTS` where appropriate).

- [ ] **Task 2: Backend DTOs, validation, and service helper** (AC: #1, #2, #4)
  - [ ] Add request/response DTOs in `src/backend/src/routes/project.rs` or a dedicated project-expense route module:
    - [ ] `CreateProjectExpenseRequest`
    - [ ] `UpdateProjectExpenseRequest`
    - [ ] `ProjectExpenseResponse`
  - [ ] Add service validation helper in `src/backend/src/services/project_service.rs` (or `expense_service.rs`):
    - [ ] Validate `amount_idr` is whole-number positive integer
    - [ ] Validate category is one of the 4 fixed budget categories
    - [ ] Validate `expense_date` is present/valid
    - [ ] `edit_reason` is `String` (non-optional) on `UpdateProjectExpenseRequest`; serde rejects missing field. Validate it is also non-empty.
  - [ ] Reuse existing `AppError::Validation` response envelope.

- [ ] **Task 3: Backend endpoints for expense CRUD + history** (AC: #1, #3, #4)
  - [ ] Add endpoints:
    - [ ] `POST /api/v1/projects/:id/expenses` (create)
    - [ ] `GET /api/v1/projects/:id/expenses` (list history)
    - [ ] `PUT /api/v1/projects/:id/expenses/:expense_id` (edit with reason)
    - [ ] `DELETE /api/v1/projects/:id/expenses/:expense_id` — hard SQL `DELETE` for MVP (no soft-delete). Log `expense_deleted` audit entry with expense snapshot before deletion. No reason required for delete (unlike edit).
  - [ ] Response list should be deterministic (newest-first by `expense_date`, then `created_at`).
  - [ ] Ensure `expense_id` belongs to `project_id` on update/delete.

- [ ] **Task 4: Authorization and RBAC ownership enforcement** (AC: #1, #3, #4)
  - [ ] Mirror Story 4.1 project-budget access rules:
    - [ ] `project_manager` can mutate/read only projects they manage (`is_project_manager()`)
    - [ ] `admin` can mutate/read any project
    - [ ] other roles receive `403 Forbidden`
  - [ ] Use `user_claims_from_headers()` + `Uuid::parse_str(&claims.sub)` patterns.
  - [ ] Log denied access attempts using `log_audit(..., "ACCESS_DENIED", "project_expense", ...)`.

- [ ] **Task 5: Budget utilization integration for non-resource costs** (AC: #3)
  - [ ] Update `get_project_budget()` and `set_project_budget()` handlers in `src/backend/src/routes/project.rs`:
    - [ ] Replace the hardcoded `let spent_to_date_idr: i64 = 0;` (appears at ~line 589 and ~line 722) with a SQL subquery:
      ```sql
      SELECT COALESCE(SUM(amount_idr), 0) FROM project_expenses WHERE project_id = $1
      ```
    - [ ] `remaining_idr = total_budget_idr - spent_to_date_idr` (unchanged formula)
  - [ ] Keep category percentages based on configured budget buckets (not on spend).
  - [ ] Maintain backward compatibility with Story 4.1 response shape — only `spent_to_date_idr` and `remaining_idr` values change.

- [ ] **Task 6: Frontend expense entry + expense history UI** (AC: #1, #2, #3, #4)
  - [ ] Extend `src/frontend/src/pages/projects.rs` and/or `src/frontend/src/components/project_list.rs` with an "Add Expense" action from project context.
  - [ ] Add expense form UI (modal/side panel consistent with current page patterns):
    - [ ] Category select (HR/Software/Hardware/Overhead)
    - [ ] Description input
    - [ ] Amount input (whole-number-only)
    - [ ] Date input
    - [ ] Vendor optional input
  - [ ] Add expense history list with edit affordance.
  - [ ] On edit, require `Edit Reason` before submit.
  - [ ] Refresh project budget summary and expense history after create/update/delete.

- [ ] **Task 7: Integration and regression tests** (AC: #1, #2, #3, #4)
  - [ ] Create `src/backend/tests/project_expense_tests.rs` following `project_budget_tests.rs` patterns:
    - [ ] PM creates expense on own project -> `200 OK`
    - [ ] PM create on non-owned project -> `403 Forbidden`
    - [ ] Admin can create on any project -> `200 OK`
    - [ ] Non-PM/non-admin role create -> `403 Forbidden`
    - [ ] Invalid category -> `400 Validation`
    - [ ] Negative/zero amount -> `400 Validation`
    - [ ] Decimal amount payload rejected -> `400/422`
    - [ ] Edit requires reason -> `400 Validation`
    - [ ] Expense appears in list after create
    - [ ] Budget summary `spent_to_date_idr` reflects expense sum
    - [ ] Audit log created for create/update/delete and ACCESS_DENIED
  - [ ] Regression: existing `project_budget_tests.rs` remain green.

## Dev Notes

### Developer Context (Critical)

- Story 4.1 established fixed project budget categories and project budget endpoints in `src/backend/src/routes/project.rs`. Story 4.2 must reuse those exact categories and ownership checks.
- `GET /api/v1/projects/:id/budget` currently hardcodes `spent_to_date_idr = 0`; Story 4.2 is the first story that must replace that with real non-resource expense aggregation.
- Frontend project management currently lives in `src/frontend/src/pages/projects.rs` with `ProjectForm` and budget summary display. Add expense UX to this existing flow rather than introducing parallel project pages.
- Money representation in this codebase for budgets is `i64` + SQL `BIGINT` (whole IDR). Keep that pattern for expenses.
- Audit logging infrastructure with hash-chain already exists; story must call `log_audit()` for all mutations and denied access events.

### Dev Guardrails

**Money and Validation**
- Use `i64` + SQL `BIGINT` for expense amounts.
- Reject decimals and negative values at API boundary.
- Keep category set fixed to `hr/software/hardware/overhead` (MVP alignment with Story 4.1).

**Authorization (CRITICAL — do NOT use `can_access_project()`)**
- `can_access_project()` grants `finance`/`department_head` read access — NOT wanted for expense mutations.
- Use the inline role-check pattern from Story 4.1's budget handlers (exact pattern from `project.rs`):
  ```rust
  // 1. Extract claims and user_id
  let claims = user_claims_from_headers(&headers)?
      .ok_or_else(|| AppError::Authentication("Missing token".into()))?;
  let user_id = Uuid::parse_str(&claims.sub)
      .map_err(|_| AppError::Authentication("Invalid user ID".into()))?;
  // 2. PM must own the project; admin bypasses
  if claims.role == "project_manager" {
      let mut conn = pool.acquire().await.map_err(|e| AppError::Database(e.to_string()))?;
      let is_pm = crate::services::rbac::is_project_manager(&mut conn, user_id, project_id).await?;
      if !is_pm {
          log_audit(&pool, Some(user_id), "ACCESS_DENIED", "project_expense", project_id,
              serde_json::json!({"reason":"not_project_manager","action":"<endpoint_name>"})).await.ok();
          return Err(AppError::Forbidden("Insufficient permissions".into()));
      }
  } else if claims.role != "admin" {
      return Err(AppError::Forbidden("Insufficient permissions".into()));
  }
  ```
- Apply this pattern to all 4 expense endpoints (POST, GET, PUT, DELETE).

**Database and Query Patterns**
- Use `sqlx::query!` / `query_as!` with compile-time checked SQL.
- Use `fetch_optional` + `ok_or_else` for single-row lookups.
- Ensure project-expense relation is enforced (expense cannot be edited through wrong project id path).

**Frontend UX**
- Follow current signal-based form pattern from `ProjectForm`.
- Keep inline validation and deterministic error feedback.
- Keep keyboard-accessible controls and visible focus states.

### API Response Types (Story 4.2)

```rust
#[derive(Debug, Deserialize)]
pub struct CreateProjectExpenseRequest {
    pub category: String,
    pub description: String,
    pub amount_idr: i64,
    pub expense_date: chrono::NaiveDate,
    pub vendor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectExpenseRequest {
    pub category: Option<String>,
    pub description: Option<String>,
    pub amount_idr: Option<i64>,
    pub expense_date: Option<chrono::NaiveDate>,
    pub vendor: Option<String>,
    pub edit_reason: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectExpenseResponse {
    pub id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub category: String,
    pub description: String,
    pub amount_idr: i64,
    pub expense_date: chrono::NaiveDate,
    pub vendor: Option<String>,
    pub created_by: Option<uuid::Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

### File Structure Requirements

- Backend:
  - `src/backend/src/routes/project.rs` (extend with expense endpoints) **or** `src/backend/src/routes/project_expense.rs` (new module + route nesting)
  - `src/backend/src/services/project_service.rs` (extend validation) **or** `src/backend/src/services/expense_service.rs` (new)
  - `src/backend/src/services/mod.rs` (export new module if created)
  - `migrations/<timestamp>_add_project_expenses.up.sql`
  - `migrations/<timestamp>_add_project_expenses.down.sql`
- Frontend:
  - `src/frontend/src/pages/projects.rs`
  - optionally `src/frontend/src/components/project_expense_form.rs`
  - optionally `src/frontend/src/components/project_expense_list.rs`
- Tests:
  - `src/backend/tests/project_expense_tests.rs`

### Testing Requirements

- Use `#[sqlx::test(migrations = "../../migrations")]` integration test style.
- Cover both positive and negative authorization paths.
- Cover validation edge cases (invalid category, negative, decimal payload).
- Verify expense write immediately affects budget summary API values.
- Verify audit logs for mutation + access denied flows.
- Reuse these helpers from `project_budget_tests.rs` — do NOT reimplement:
  - `create_test_user_with_role(pool, role)` — creates user with specified role
  - `get_auth_token(pool, user_id)` — generates JWT for test user
  - `create_test_project_with_pm(pool, pm_user_id)` — creates project with PM assignment

### Previous Story Intelligence (4.1)

- Story 4.1 introduced fixed project budgets and ownership-guarded budget endpoints.
- Story 4.1 intentionally deferred non-resource expenses and `spent_to_date` population; Story 4.2 is the intended continuation.
- Story 4.1 uses `project_service::validate_project_budget()` and thin route handlers; follow same separation-of-concerns standard.
- Story 4.1 tests (`project_budget_tests.rs`) provide reusable auth/setup helper patterns for Story 4.2 tests.

### Git Intelligence Summary

- Recent commit shows Story 4.1 implementation touched:
  - `src/backend/src/routes/project.rs`
  - `src/backend/src/services/project_service.rs`
  - `src/backend/tests/project_budget_tests.rs`
  - `src/frontend/src/components/project_form.rs`
  - `src/frontend/src/pages/projects.rs`
  - budget migration files under `migrations/`
- Story 4.2 should extend these same files/modules before adding new abstraction layers.

### Latest Technical Information

- Pinned stack in project context remains: Rust 1.75+, Axum 0.7, sqlx 0.7, Leptos 0.6.
- Do not introduce framework upgrades as part of this story.
- Keep Tailwind utility usage consistent with existing frontend patterns.

### Project Structure Notes

- This is a brownfield extension. Prefer incremental changes in existing project management routes/pages over net-new vertical slices.
- Preserve current API envelope and response structures to avoid downstream regressions in ongoing Epic 4 work.

### References (Priority Order)

Read before coding:
1. `src/backend/src/routes/project.rs` — Budget endpoint auth + handler patterns to mirror
2. `src/backend/src/services/rbac.rs` — `is_project_manager()` ownership check
3. `src/backend/src/services/audit_log.rs` — `log_audit()` + `user_claims_from_headers()`
4. `src/backend/tests/project_budget_tests.rs` — Test structure + reusable helpers
5. `src/frontend/src/components/project_form.rs` — Form component pattern
6. `src/frontend/src/pages/projects.rs` — Page to extend with expense UI

Planning artifacts (consult if ambiguous):
- `_bmad-output/planning-artifacts/epics.md` — Epic 4 story definitions
- `_bmad-output/planning-artifacts/prd.md` — FR18-FR22 requirements
- `_bmad-output/implementation-artifacts/4-1-project-budget-setup.md` — Predecessor story

## Dev Agent Record

### Agent Model Used

anthropic/claude-opus-4-6

### Debug Log References

- Workflow source: `_bmad/bmm/workflows/4-implementation/create-story/workflow.yaml`
- Workflow instructions: `_bmad/bmm/workflows/4-implementation/create-story/instructions.xml`
- Validation checklist: `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- Sprint tracking source: `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Completion Notes List

- Story created from explicit user target `4-2` (no auto-discovery needed).
- Context synthesized from epics, PRD, architecture, UX spec, project context, Story 4.1 implementation artifact, and current codebase route/form/test patterns.
- Story status set to `ready-for-dev` with concrete backend/frontend/testing guardrails and compatibility constraints.
- Validated via `validate-create-story` checklist. Applied 3 critical fixes (edit_reason typing, auth pattern clarification, migration schema note), 4 enhancements (auth snippet, SQL specificity, delete behavior, test helpers), and 2 optimizations (references consolidation, agent model correction).

### File List

- `_bmad-output/implementation-artifacts/4-2-non-resource-cost-entry.md`
