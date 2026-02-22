# Story 1.2: Role-Based Access Control

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **System Administrator**,
I want **to assign roles to users (HR, Department Head, Project Manager, Finance, Admin)**,
so that **users can only access features appropriate to their responsibilities**.

## Acceptance Criteria

1. **Given** I am logged in as Admin **when** I navigate to User Management **then** I can view all users and their current roles.
2. **Given** I select a user to edit **when** I change their role to "HR" **then** the user gains access to CTC management features **and** the change is logged with timestamp and admin ID.
3. **Given** a user has the "Department Head" role **when** they attempt to access CTC component details **then** access is denied with "Insufficient permissions" message **and** the access attempt is logged for audit.
4. **Given** a user has the "Project Manager" role **when** they access resource assignment **then** they see only blended rates, never CTC components **and** they can only view projects assigned to them.

## Tasks / Subtasks

- [x] **Task 1: Enforce role-aware user management access and role updates** (AC: #1, #2)
  - [x] Add/verify backend authorization checks so only Admin can create/update/delete users and roles in `src/backend/src/routes/user.rs`.
  - [x] Keep role update pathway in `PUT /api/v1/users/:id` and ensure role field updates remain explicit and validated.
  - [x] Return consistent forbidden response (`Insufficient permissions`) for non-admin attempts.
- [x] **Task 2: Strengthen audit logging for role changes and denied attempts** (AC: #2, #3)
  - [x] Ensure role-change audit payload includes before/after role, acting admin ID, timestamp, and target user ID.
  - [x] Log denied role-management and denied CTC component access attempts via existing `log_audit` service.
  - [x] Keep audit entity/action naming consistent with current audit conventions.
- [x] **Task 3: Implement role-based data exposure rules** (AC: #3, #4)
  - [x] Add/verify backend endpoint-level checks for CTC detail access (Dept Head denied detailed CTC components).
  - [x] Ensure Project Manager views only blended rates (no salary/BPJS/THR component fields in response DTOs).
  - [x] Restrict Project Manager project visibility to assigned projects only (query filtering, not frontend-only filtering).
- [x] **Task 4: Align frontend user management and authorization UX** (AC: #1, #2, #3)
  - [x] Ensure User Management page reflects current roles from backend and role updates via existing edit flow.
  - [x] Handle forbidden responses with clear "Insufficient permissions" UI message.
  - [x] Prevent exposing role-management controls to non-admin users in UI, while preserving backend as source of truth.
- [x] **Task 5: Add integration tests for RBAC behavior** (AC: #1, #2, #3, #4)
  - [x] Add backend integration tests for admin role-change success and audit log creation.
  - [x] Add tests for non-admin denial on role-management routes.
  - [x] Add tests confirming Department Head cannot access CTC component details.
  - [x] Add tests confirming Project Manager receives blended rates only and only assigned projects.

## Dev Notes

### Developer Context (Critical)

- Story 1.1 already established JWT login, `/auth/me`, refresh-token rotation, and generic auth error behavior; reuse these patterns instead of creating parallel auth flows.
- Existing user management endpoints already exist in `src/backend/src/routes/user.rs`; implement RBAC by extending these handlers/middleware, not by creating duplicate user admin APIs.
- Audit infrastructure already exists in `src/backend/src/services/audit_log.rs`; use `log_audit`, `audit_payload`, and `user_id_from_headers` consistently.
- Role string values in current code use snake_case (`admin`, `hr`, `department_head`, `project_manager`, `finance`); keep API/UI role values aligned with this format.

### Technical Requirements

- Enforce role checks server-side for all sensitive endpoints (user role changes, CTC details, assignment visibility).
- Keep response contracts role-safe: only HR/Admin can get CTC component-level fields; PM/Dept Head must get blended rates only.
- Use project error style (`AppError`) and avoid information leakage in permission failures.
- Preserve existing auth token extraction pattern via headers and JWT claims.

### Architecture Compliance

- Follow Axum route composition through `routes/mod.rs` and merged API router in `src/backend/src/lib.rs`.
- Keep handler signatures and error propagation consistent with project conventions (`Result<Json<_>>`, `.map_err(...)?`).
- Keep SQL in `sqlx` parameterized queries; for row/filter constraints use SQL-side filtering.
- Audit all security-relevant mutations and denied attempts before returning responses.

### Library / Framework Requirements

- Rust `1.75+`, Axum `0.7`, sqlx `0.7`, PostgreSQL `15+`, jsonwebtoken `9`, Argon2 `0.5`.
- No dependency upgrades required for this story.
- Use current middleware/service patterns; avoid introducing new auth frameworks unless absolutely required.

### File Structure Requirements

- Backend implementation targets:
  - `src/backend/src/routes/user.rs`
  - `src/backend/src/routes/*` files where CTC/resource-assignment authorization is enforced
  - `src/backend/src/services/audit_log.rs` (reuse, minimal extension only)
  - `src/backend/src/middleware/auth.rs` (if centralized role checks are introduced)
  - `src/backend/src/models/*` for DTO shaping if needed
- Frontend implementation targets:
  - `src/frontend/src/pages/users/users_content.rs`
  - Role-sensitive pages/components for CTC/resource assignment visibility
- Tests:
  - `src/backend/tests/*` integration tests for RBAC scenarios

### Testing Requirements

- Add integration tests for:
  - Admin can view users and change role.
  - Role change is audited with admin ID and before/after role.
  - Non-admin receives forbidden on user-role management endpoints.
  - Department Head denied CTC component detail access and denial is audited.
  - Project Manager gets blended-rate-only payload and assigned-project filtering.
- Keep test style aligned with Story 1.1 auth tests (`#[sqlx::test(...)]`, API-level request/response assertions).

### Previous Story Intelligence (1.1)

- Keep generic security-facing error behavior; do not leak sensitive authorization internals.
- Reuse `/api/v1/auth/me` for frontend role-awareness and route gating.
- Maintain production rule: no `unwrap()`/`expect()` in production paths.
- Integration tests should hit real endpoints, not only DB mutations.

### Git Intelligence Summary

- Recent commits show route/service-centric backend changes and page/component-centric frontend changes.
- User management is already in place (`src/backend/src/routes/user.rs`, `src/frontend/src/pages/users/users_content.rs`), so Story 1.2 should be an enhancement pass, not greenfield.
- Existing audit logging is actively used across routes; maintain naming and payload shape consistency.

### Latest Technical Information

- Current best practice remains: server-enforced RBAC layered with middleware/handler checks, short-lived JWT with strict validation, and auditable authorization denials.
- Keep permission checks centralized where practical to avoid drift across handlers.

### Project Structure Notes

- Align with existing module layout (`routes`, `services`, `models`, `pages`, `components`) and snake_case file names.
- Use DTO-driven API responses to avoid accidental exposure of sensitive DB fields.
- No structural conflicts detected with current architecture; story is a focused security hardening/feature-completion increment.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 1: Security & RBAC Foundation]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.2: Role-Based Access Control]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions]
- [Source: _bmad-output/planning-artifacts/architecture.md#API Design Decisions]
- [Source: _bmad-output/project-context.md#Critical Implementation Rules]
- [Source: _bmad-output/project-context.md#Framework-Specific Rules (Axum + Leptos)]
- [Source: _bmad-output/implementation-artifacts/1-1-user-authentication-system.md]
- [Source: src/backend/src/routes/user.rs]
- [Source: src/backend/src/services/audit_log.rs]
- [Source: src/frontend/src/pages/users/users_content.rs]

### Story Creation Completion Note

- Ultimate context engine analysis completed - comprehensive developer guide created.

### Senior Developer Review (AI)

- Reviewer: Amelia (Developer Agent)
- Date: 2026-02-22
- Outcome: Changes Requested resolved
- Resolution summary:
  - Enforced admin-only access for user listing/detail endpoints
  - Added allocation mutation authorization checks and PM scoping for resource allocation listing
  - Added CTC component access-control endpoint to enforce and audit Department Head denial path
  - Replaced newly introduced `expect`/`unwrap` hot spots in reviewed RBAC paths
  - Expanded RBAC integration tests to cover denied/allowed scenarios from story ACs

### Change Log

- 2026-02-22: Remediated code-review findings for Story 1.2 (admin guards for user reads, allocation RBAC hardening, CTC denial audit endpoint, expanded RBAC integration test coverage)

## Dev Agent Record

### Agent Model Used

TBD

### Debug Log References

TBD

### Completion Notes List

- RBAC review issues fixed and validated via integration tests.
- AC1: Admin-only user-management access enforced for list/detail/mutations.
- AC3: Department Head access to CTC component details now denied with "Insufficient permissions" and audit logging.
- AC4: Project Manager allocation visibility narrowed to assigned projects in resource-allocation endpoint.

### File List

- `src/backend/src/routes/user.rs`
- `src/backend/src/routes/allocation.rs`
- `src/backend/src/routes/ctc.rs`
- `src/backend/src/routes/mod.rs`
- `src/backend/src/lib.rs`
- `src/backend/tests/rbac_tests.rs`
