# Story 2.1: Employee CTC Record Creation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **HR Staff member**,
I want **to create employee CTC records with full component breakdown**,
so that **the system has accurate cost data for project calculations**.

## Acceptance Criteria

1. **Given** I am logged in as HR **when** I navigate to CTC Management -> Add Employee **then** I see a form with fields: Employee ID, Name, Department, Base Salary, Allowances.
2. **Given** I enter CTC components **when** I input values in IDR **then** the system rejects any decimal places (whole numbers only) **and** displays validation errors for negative values.
3. **Given** I enter base salary and allowances **when** I click "Calculate BPJS" **then** the system calculates BPJS Kesehatan (4% employer, 1% employee) and BPJS Ketenagakerjaan (0.24-1.74% based on tier) **and** displays the calculated amounts for confirmation.
4. **Given** I complete the CTC form **when** I click "Save" **then** the record is created with `status="Active"` **and** the daily rate is automatically calculated (monthly CTC / 22 working days) **and** an audit log entry is created with all values.
5. **Given** I am logged in as a non-HR role **when** I view the app navigation or access `/ctc` directly **then** the CTC menu is hidden **and** direct access is blocked by RBAC (forbidden UX state + redirect to `/dashboard`).

## Tasks / Subtasks

- [x] **Task 1: Build backend create-CTC API contract and persistence model** (AC: #1, #4)
  - [x] Introduce/extend typed CTC request/response DTOs for component-level fields (employee/resource + department + salary + allowances + BPJS values + derived totals/rate + status).
  - [x] Implement `POST /api/v1/ctc` (or equivalent canonical route) in `src/backend/src/routes/ctc.rs` with HR-only authorization checks.
  - [x] Persist record as active with required metadata (`created_by`, `created_at`, `status`, effective date).
- [x] **Task 2: Enforce IDR input validation and safe arithmetic** (AC: #2)
  - [x] Validate all IDR monetary inputs as whole numbers (no decimal fraction accepted at API boundary).
  - [x] Reject negative values with field-specific validation messages.
  - [x] Keep BigDecimal/numeric-safe conversions and avoid float-based money persistence.
- [x] **Task 3: Implement BPJS calculation service for create flow** (AC: #3)
  - [x] Add/extend CTC calculation service to compute BPJS Kesehatan and BPJS Ketenagakerjaan components from entered salary/allowances.
  - [x] Keep payroll constants/rates configurable and centrally defined (do not hardcode deep in handlers).
  - [x] Return calculation preview payload used by UI confirmation step.
- [x] **Task 4: Compute daily rate and total monthly CTC automatically** (AC: #4)
  - [x] Derive total monthly CTC from component sum.
  - [x] Compute daily rate using `monthly_ctc / working_days` (default `22`, configurable).
  - [x] Ensure derived values are stored and returned consistently.
- [x] **Task 5: Integrate audit logging for CTC creation** (AC: #4)
  - [x] Emit audit event on successful create with actor, target employee/resource, timestamp, and full value snapshot.
  - [x] Keep action/entity naming aligned with established audit conventions.
  - [x] Ensure hash-chain audit integration remains intact with Story 1.4 infrastructure.
- [x] **Task 6: Add frontend CTC Add Employee workflow (HR-only)** (AC: #1, #2, #3, #4, #5)
  - [x] Implement/extend CTC Management form with required fields and inline validation feedback.
  - [x] Add BPJS calculation preview action and confirmation display.
  - [x] Submit create request and show success/error states with role-safe messaging.
  - [x] Hide CTC top-nav item for non-HR users and enforce RBAC guard on direct `/ctc` access (forbidden UX state + redirect).
- [x] **Task 7: Add integration and validation tests** (AC: #1, #2, #3, #4)
  - [x] Backend integration test: HR can create CTC record with active status, derived totals, daily rate, and audit entry.
  - [x] Backend integration test: decimal and negative inputs are rejected.
  - [x] Backend integration test: non-HR role denied create access.
  - [x] Calculation test coverage for BPJS formula and tier handling.

## Dev Notes

### Developer Context (Critical)

- Epic 1 stories are completed; Story 2.1 must reuse existing security and audit foundations instead of introducing parallel mechanisms.
- Existing CTC route/file already exists at `src/backend/src/routes/ctc.rs` and currently persists JSON component snapshots via `ctc_records`; extend this path to support full create workflow and derived fields.
- Existing audit chain and report/export infrastructure from Story 1.4 must remain source-of-truth for CTC create logging.

### Technical Requirements

- CTC create flow is HR-only and must align with role checks already established in prior stories.
- IDR values must be whole numbers; reject decimals and negatives at validation layer.
- BPJS calculations must be deterministic and transparent, with rates/config maintained in one place.
- Daily rate calculation defaults to `22` working days but should remain configurable.
- Creation must result in active record state and complete audit logging.

### Architecture Compliance

- Keep route/module structure aligned with Axum conventions (`routes/mod.rs` export + router merge in `lib.rs`).
- Keep handlers thin and extract reusable calculation logic into service/module code.
- Use `AppError` and parameterized SQL (`sqlx`) with explicit error mapping.
- No production `unwrap`/`expect` in new code paths.

### Library / Framework Requirements

- Rust `1.75+`, Axum `0.7`, sqlx `0.7`, PostgreSQL `15+`, serde `1.0`, validator `0.16`, bigdecimal `0.4`.
- Use numeric-safe money handling (BigDecimal/integer semantics) for persistence and computation.
- For request validation, follow project validator patterns (typed DTO + explicit validation errors).

### File Structure Requirements

- Backend likely touchpoints:
  - `src/backend/src/routes/ctc.rs`
  - `src/backend/src/services/*` (new/extended CTC calculation helper)
  - `src/backend/src/services/audit_log.rs` (reuse only; minimal extension if needed)
  - `src/backend/src/routes/mod.rs`
  - `src/backend/src/lib.rs`
- Database/migrations:
  - `migrations/*` for any CTC schema alignment needed for full component-level create flow
- Frontend likely touchpoints:
  - `src/frontend/src/pages/*` CTC management form/page modules
- Tests:
  - `src/backend/tests/*` integration coverage for create + validation + authorization

### Testing Requirements

- Add API-level integration tests with sqlx harness (`#[sqlx::test(migrations = "../../migrations")]`).
- Validate field-level failures for decimals/negatives and authorization failures for non-HR.
- Verify audit event insertion for successful create and ensure hash-chain fields populate via existing log service.
- Include deterministic BPJS calculation tests with tier/cap edge cases in unit or integration scope.

### Previous Story Intelligence (1.4)

- Reuse deterministic audit-hash path (`log_audit` + chain fields) and avoid bypass logging.
- CTC routes currently support RLS-aware view/update and persist component JSON in `ctc_records`; Story 2.1 should evolve this into first-class create flow rather than replacing it.
- Keep export/audit/report improvements intact while adding create-specific events.

### Git Intelligence Summary

- Recent implementation style favors small, route/service-focused backend changes with explicit migration + integration test updates.
- Continue with that cadence for Story 2.1: focused CTC create route/service/migration changes and high-signal integration tests.

### Latest Technical Information

- BPJS rates/caps can evolve; keep calculation constants configurable and documented rather than deeply hardcoded.
- Decimal-safe monetary handling remains mandatory; avoid float persistence/math for payroll components.
- Axum validated extractors / validator patterns are current best practice for strict API input contracts.

### Project Structure Notes

- No structural conflicts detected.
- Story 2.1 should be implemented as incremental extension of existing CTC/audit/security stack.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.1: Employee CTC Record Creation]
- [Source: _bmad-output/planning-artifacts/prd.md#FR1]
- [Source: _bmad-output/planning-artifacts/prd.md#FR5]
- [Source: _bmad-output/planning-artifacts/prd.md#FR6]
- [Source: _bmad-output/planning-artifacts/prd.md#FR7]
- [Source: _bmad-output/planning-artifacts/prd.md#FR8]
- [Source: _bmad-output/planning-artifacts/prd.md#FR42]
- [Source: _bmad-output/planning-artifacts/architecture.md#Decision: CTC Data Model]
- [Source: _bmad-output/planning-artifacts/architecture.md#Calculation Engine Decisions]
- [Source: _bmad-output/project-context.md#Critical Implementation Rules]
- [Source: _bmad-output/project-context.md#Framework-Specific Rules (Axum + Leptos)]
- [Source: _bmad-output/implementation-artifacts/1-4-audit-logging-system.md]
- [Source: src/backend/src/routes/ctc.rs]

### Story Creation Completion Note

- Ultimate context engine analysis completed - comprehensive developer guide created.

## Dev Agent Record

### Agent Model Used

TBD

### Debug Log References

TBD

### Completion Notes List

- Story 2.1 backend and frontend implementation complete
- AC #1 now implemented with frontend CTC Add Employee workflow at `/ctc`
- New endpoints added: POST /api/v1/ctc (create), POST /api/v1/ctc/calculate (preview)
- Strict HR-only authorization enforced on create/calculate endpoints (admin excluded)
- IDR validation: whole numbers enforced via i64 types and range validators
- BPJS calculations: configurable rates via BpjsConfig, supports risk tiers 1-4
- Daily rate auto-calculated: monthly_ctc / working_days (default 22) and persisted as numeric with decimals
- Audit logging: CREATE events with full value snapshot via hash-chain infrastructure
- Database migration: extended ctc_records with component columns and derived fields
- Unit tests: 3 tests for BPJS, daily rate, THR calculations
- Integration tests: 6 tests covering create, auth (including admin-denied), validation, duplicates, preview
- ✅ Review fix: Added frontend CTC form/page with BPJS preview and save flow
- ✅ Review fix: Removed production unwrap paths in CTC create/calculate risk-tier and daily-rate conversion paths
- ✅ Review fix: Updated story documentation and file list to match git reality
- ✅ UX/security alignment: CTC navigation visibility is HR-only; non-HR direct `/ctc` access is blocked by frontend RBAC guard and redirected to dashboard

### File List

- migrations/20260222140000_extend_ctc_records.up.sql
- migrations/20260222140000_extend_ctc_records.down.sql
- src/backend/src/routes/ctc.rs
- src/backend/src/services/mod.rs
- src/backend/src/services/ctc_calculator.rs
- src/backend/tests/ctc_tests.rs
- src/frontend/src/components/mod.rs
- src/frontend/src/lib.rs
- src/frontend/src/pages/ctc.rs
- src/frontend/src/pages/mod.rs
- _bmad-output/implementation-artifacts/2-1-employee-ctc-record-creation.md
- _bmad-output/implementation-artifacts/sprint-status.yaml

## Senior Developer Review (AI)

### Review Date

- 2026-02-22

### Outcome

- Approve

### Findings Addressed

- [x] [HIGH] Implement missing frontend CTC Add Employee workflow to satisfy AC #1.
- [x] [HIGH] Preserve decimal daily rate in persistence layer (`NUMERIC(12,2)`) without truncating to integer.
- [x] [HIGH] Align story claims/status with actual implementation completion.
- [x] [MEDIUM] Enforce strict HR-only authorization (admin denied) for CTC create/calculate.
- [x] [MEDIUM] Remove unwrap-dependent production paths in CTC route for risk-tier rate and daily-rate conversion.
- [x] [MEDIUM] Reconcile story File List with actual changed files.

## Change Log

- 2026-02-22: Implemented Story 2.1 backend create/preview flow with BPJS calculation, audit logging, migration, and integration tests.
- 2026-02-22: Addressed adversarial code-review findings; added frontend CTC management page, fixed daily-rate precision persistence, enforced strict HR-only access, expanded tests, and synchronized story documentation/status.
