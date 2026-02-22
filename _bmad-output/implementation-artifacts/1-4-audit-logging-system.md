# Story 1.4: Audit Logging System

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **Finance Controller**,
I want **every CTC view and mutation to be logged with complete details**,
so that **I can demonstrate compliance during audits and detect unauthorized access**.

## Acceptance Criteria

1. **Given** a user views any CTC record **when** the view action completes **then** an audit log entry is created with: user ID, timestamp, employee ID, action="VIEW".
2. **Given** a user modifies a CTC record **when** the change is saved **then** an audit log entry includes: before values, after values, change reason **and** the log entry hash is computed for tamper detection.
3. **Given** I navigate to Audit Reports **when** I filter by date range and action type **then** I see all matching audit entries **and** I can export the report (subject to four-eyes approval).
4. **Given** an audit log entry exists **when** I verify the hash chain **then** any tampering with the log is immediately detectable.

## Tasks / Subtasks

- [x] **Task 1: Extend audit storage model for tamper-evident chain** (AC: #2, #4)
  - [x] Add migration(s) to extend `audit_logs` with hash-chain fields (e.g., `previous_hash`, `entry_hash`, optional sequence/index metadata).
  - [x] Ensure backward-compatible handling for existing audit rows.
  - [x] Add DB constraints/indexes required for deterministic chain traversal and reporting performance.
- [x] **Task 2: Implement deterministic hash-chain generation in audit service** (AC: #2, #4)
  - [x] Extend `src/backend/src/services/audit_log.rs` to compute `entry_hash` using canonicalized payload + previous hash.
  - [x] Enforce deterministic serialization for hash input (RFC8785-style canonical JSON or equivalent deterministic serializer).
  - [x] Persist chain links atomically when new audit entries are inserted.
- [x] **Task 3: Ensure complete CTC view and mutation audit capture** (AC: #1, #2)
  - [x] Audit all CTC `VIEW` actions with actor, target employee/resource, timestamp, action metadata.
  - [x] Audit CTC mutation actions with before/after snapshots and required reason field.
  - [x] Keep action/entity naming consistent with current audit conventions.
- [x] **Task 4: Add audit report/filter/export endpoints and access controls** (AC: #3)
  - [x] Create/extend backend audit-report routes supporting filters: date range, action type, actor, entity.
  - [x] Return paginated/sorted results for operational query performance.
  - [x] Add export endpoint contract and approval-state fields/hooks required for four-eyes workflow handoff.
  - [x] Restrict audit-report access to appropriate roles (Finance/Admin) and log report/export access.
- [x] **Task 5: Add tamper verification and integration tests** (AC: #4)
  - [x] Implement chain verification routine/API that detects altered or broken hash links.
  - [x] Add integration tests for CTC view/mutation audit creation, including before/after and reason capture.
  - [x] Add tests for report filtering and export eligibility behavior.
  - [x] Add tests proving tamper detection triggers when historical audit record content/hash is altered.
- [x] **Task 6: Harden append-only behavior for audit integrity** (AC: #4)
  - [x] Add DB-level controls to block normal `UPDATE/DELETE` on `audit_logs` (trigger/policy-based append-only behavior).
  - [x] Document operational exception path (if any) for privileged maintenance.

## Dev Notes

### Developer Context (Critical)

- Story 1.1 introduced JWT + security error patterns; Story 1.2/1.3 already expanded `log_audit` usage. Story 1.4 should extend existing audit infrastructure, not introduce a parallel logging system.
- Current audit helper in `src/backend/src/services/audit_log.rs` writes basic rows but does not yet implement full hash-chain integrity logic.
- Existing CTC access routes are in `src/backend/src/routes/ctc.rs`; this is the primary integration point for required `VIEW` and mutation auditing.

### Technical Requirements

- Hash chain must be deterministic and reproducible for verification.
- Canonical JSON serialization is required for hash input stability (avoid key-order nondeterminism).
- Audit logs should behave append-only in normal operation.
- Keep security-safe error responses; do not leak sensitive internals in verification/report failures.

### Architecture Compliance

- Keep route composition in `src/backend/src/routes/mod.rs` and API composition in `src/backend/src/lib.rs`.
- Keep service logic in `src/backend/src/services/*`; handlers remain thin.
- Use `AppError` and sqlx parameterized queries.
- Avoid `unwrap`/`expect` in production paths.

### Library / Framework Requirements

- Rust `1.75+`, Axum `0.7`, sqlx `0.7`, PostgreSQL `15+`, serde `1.0`, sha2 `0.10`.
- Use deterministic canonical JSON approach for hash input (RFC8785-style crate/pattern).
- No framework changes required.

### File Structure Requirements

- Expected backend touchpoints:
  - `src/backend/src/services/audit_log.rs`
  - `src/backend/src/routes/ctc.rs`
  - `src/backend/src/routes/audit_log.rs`
  - `src/backend/src/routes/mod.rs`
  - `src/backend/src/lib.rs`
- DB migrations:
  - `migrations/*` for audit schema extension + append-only protections
- Tests:
  - `src/backend/tests/*` (new/extended integration tests for audit and tamper detection)

### Testing Requirements

- Add integration tests covering:
  - CTC `VIEW` action audit creation with required fields
  - CTC mutation audit includes before/after + reason + hash
  - audit report filtering by date range/action
  - export endpoint access/eligibility behavior
  - chain verification detects tampering
- Keep sqlx integration pattern (`#[sqlx::test(migrations = "../../migrations")]`).

### Previous Story Intelligence (1.1 + 1.2 + 1.3)

- Reuse claims extraction helpers (`user_claims_from_headers`, `user_id_from_headers`) and existing action naming conventions.
- Preserve strict security posture: generic denial/error responses and no production `unwrap`/`expect`.
- Follow recent route/service-centric implementation and endpoint-level test practices.

### Git Intelligence Summary

- Recent commits indicate stable implementation pattern: small route/service enhancements + migration updates + focused integration tests.
- Story 1.4 should follow that cadence (audit service enhancement + route/report additions + test coverage).

### Latest Technical Information

- Canonical JSON is required for cryptographic log hashing stability; standard JSON serialization is not sufficient due to object-key-order variance.
- Hash-chain verification should iterate in insertion order and compare both recomputed entry hash and `previous_hash` linkage.
- DB-level append-only controls (trigger/policy) are recommended to complement hash-chain tamper detection.

### Project Structure Notes

- No structural conflict with existing architecture.
- Story is a security/compliance hardening increment that deepens existing audit features.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.4: Audit Logging System]
- [Source: _bmad-output/planning-artifacts/prd.md#FR52]
- [Source: _bmad-output/planning-artifacts/prd.md#FR57]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR14]
- [Source: _bmad-output/planning-artifacts/architecture.md#Security Architecture Decisions]
- [Source: _bmad-output/project-context.md#Critical Implementation Rules]
- [Source: _bmad-output/project-context.md#Development Workflow Rules]
- [Source: _bmad-output/implementation-artifacts/1-1-user-authentication-system.md]
- [Source: _bmad-output/implementation-artifacts/1-2-role-based-access-control.md]
- [Source: _bmad-output/implementation-artifacts/1-3-row-level-security.md]

### Story Creation Completion Note

- Ultimate context engine analysis completed - comprehensive developer guide created.

## Dev Agent Record

### Agent Model Used

### Agent Model Used

Antigravity

### Debug Log References

- Relied on serde_json object keys serialization which uses BTreeMap implicitly (providing deterministic outputs natively).
- Used explicit advisory lock `pg_advisory_xact_lock` to serialize concurrent audit logging efficiently.

### Completion Notes List

- Database migrated with `previous_hash`, `entry_hash`, append-only trigger, and additional persistence tables for export workflow and CTC component storage.
- Hash-chain generation and verification now use deterministic canonical JSON serialization, shared by writer and verifier.
- `/audit-logs/export` now persists pending approval requests in `audit_export_requests` for four-eyes workflow handoff.
- CTC mutation path now persists updates and records true before/after snapshots with required reason.
- Audit/report routes now fail on critical audit write errors instead of silently swallowing failures.
- Integration coverage expanded for export persistence and tamper-detection verification.

### File List

- `migrations/20260222133000_audit_hash_chain.up.sql`
- `migrations/20260222133000_audit_hash_chain.down.sql`
- `migrations/20260222134500_audit_exports_and_ctc_records.up.sql`
- `migrations/20260222134500_audit_exports_and_ctc_records.down.sql`
- `src/backend/src/services/audit_log.rs`
- `src/backend/src/routes/audit_log.rs`
- `src/backend/src/routes/ctc.rs`
- `src/backend/src/services/mod.rs`
- `src/backend/tests/audit_tests.rs`

### Senior Developer Review (AI)

- Reviewer: Amelia (Developer Agent)
- Date: 2026-02-22
- Outcome: High/Medium findings remediated
- Resolution summary:
  - Removed compile-time DB mismatch risk by replacing new hash-chain SQLx macro queries with runtime-bound queries
  - Implemented deterministic canonical hashing path and shared recomputation for chain verification
  - Added persistence for export request approval handoff (`audit_export_requests`)
  - Replaced placeholder CTC mutation behavior with persisted before/after state in `ctc_records`
  - Removed silent error swallowing for critical audit/report operations and tightened role access to Finance/Admin

### Change Log

- 2026-02-22: Applied code-review remediation for Story 1.4 (deterministic hash-chain integrity, export-request persistence, CTC mutation persistence, stricter error handling, extended integration tests)
