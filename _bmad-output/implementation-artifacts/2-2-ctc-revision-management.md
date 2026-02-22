# Story 2.2: CTC Revision Management

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **HR Staff member**,
I want **to update CTC components with full revision tracking**,
so that **salary changes are documented with who made the change and why**.

## Acceptance Criteria

1. **Given** an employee has an existing CTC record **when** I click "Edit CTC" **then** I see the current values and a "Change Reason" field (required).
2. **Given** I modify the base salary **when** I enter a new value and provide a reason **then** the system creates a new revision record **and** preserves the previous version in history.
3. **Given** I view CTC details **when** I click "View History" **then** I see a chronological list of all changes with: date, user, field changed, old value, new value, reason.
4. **Given** a mid-month CTC change **when** I apply the update **then** the system applies pro-rata calculation for the current month (configurable: pro-rata OR effective-first-of-month).

## Tasks / Subtasks

- [x] **Task 1: Introduce durable CTC revision persistence model** (AC: #2, #3)
  - [x] Add a dedicated append-only revisions table (e.g., `ctc_revisions`) keyed by `id` with `resource_id`, revision metadata, encrypted payloads, `changed_by`, `reason`, and timestamps.
  - [x] Keep `ctc_records` as current-snapshot table for fast reads and blended-rate consumers, but source historical timeline from `ctc_revisions`.
  - [x] Add migration indexes for revision timeline queries (`resource_id`, `created_at DESC`, optional `revision_number`).

- [x] **Task 2: Preserve Story 2.0 encryption guarantees in revision writes** (AC: #2, #3)
  - [x] Store revision values as ciphertext (`encrypted_components`, `encrypted_daily_rate`) plus encryption metadata (`key_version`, `encryption_version`, `encryption_algorithm`, `encrypted_at`).
  - [x] Do not persist sensitive component plaintext in revision history rows.
  - [x] Ensure DB-admin table reads cannot recover component/daily-rate plaintext from revisions.

- [x] **Task 3: Implement HR-only update flow that appends revision + updates snapshot** (AC: #1, #2, #4)
  - [x] Extend `PUT /api/v1/ctc/:resource_id/components` to require non-empty `reason`, append a revision row, and atomically update current snapshot.
  - [x] Keep strict HR-only RBAC and RLS checks in all CTC mutation paths.
  - [x] Compute and persist derived values (`total_monthly_ctc`, encrypted `daily_rate`) consistently with existing CTC calculator logic.

- [x] **Task 4: Implement revision history endpoint with secure diff payload** (AC: #3)
  - [x] Add `GET /api/v1/ctc/:resource_id/history` (HR-only) returning chronological entries.
  - [x] For each entry, include date, actor, reason, and changed-field diff (`field`, `old_value`, `new_value`) from decrypted snapshots in service layer only.
  - [x] Do not expose raw encrypted blobs in client response except optional metadata for troubleshooting.

- [x] **Task 5: Add configurable mid-month policy handling** (AC: #4)
  - [x] Introduce configurable CTC effective-date policy (`pro_rata` vs `effective_first_of_month`) from `global_settings` or equivalent config source.
  - [x] On update, apply selected policy deterministically and record policy/effective-date in revision metadata.
  - [x] Include policy behavior in API response/history so HR can audit why values changed.

- [x] **Task 6: Extend frontend CTC page for edit + history workflows** (AC: #1, #3, #4)
  - [x] Add explicit edit workflow with required "Change Reason" field and role-safe validation errors.
  - [x] Add "View History" panel/table with chronological revisions and field-level before/after display.
  - [x] Display effective-date policy outcome (pro-rata vs first-of-month) in update confirmation/history UI.

- [x] **Task 7: Add regression-safe tests for revisions, history, and policy** (AC: #1, #2, #3, #4)
  - [x] Integration test: update creates new revision row and preserves prior version.
  - [x] Integration test: history endpoint returns chronological entries with expected diff fields and reasons.
  - [x] Integration test: non-HR cannot access revision mutation/history endpoints.
  - [x] Integration test: encrypted revision storage contains no sensitive plaintext (`components`, legacy numeric fields, `daily_rate`).
  - [x] Policy tests: pro-rata and effective-first-of-month produce expected effective-date/result semantics.

## Dev Notes

### Developer Context (Critical)

- Story 2.1 and 2.0 are done; Story 2.2 must **extend** those implementations (no redesign of auth/encryption foundations).
- Current CTC update endpoint exists at `src/backend/src/routes/ctc.rs`; evolve it into revision-aware behavior instead of replacing API contracts unnecessarily.
- Sensitive CTC data now includes component values **and daily rate**; both are encrypted at-rest in current design and must remain so.

### Technical Requirements

- HR-only edit/update/history for CTC component details.
- "Change Reason" is mandatory for every mutation.
- Revision timeline must preserve prior state and support field-level before/after diffs.
- Mid-month rule must be configurable (`pro_rata` or `effective_first_of_month`) and auditable.
- IDR rules remain: no decimals for component amounts; deterministic BigDecimal-based calculations.

### Architecture Compliance

- Maintain existing Axum route composition and module exports.
- Keep handlers thin; put diffing/revision business logic in services.
- Continue `AppError` mapping, no production `unwrap`/`expect`.
- Preserve audit hash-chain behavior for view/mutation events.
- Preserve encryption fail-closed behavior for sensitive CTC reads/writes.

### Library / Framework Requirements

- Rust 1.75+, Axum 0.7, sqlx 0.7, PostgreSQL 15+, serde 1.0, validator 0.16, bigdecimal 0.4.
- Reuse existing crypto stack from Story 2.0 (`aes-gcm`, base64, env-backed key provider abstraction).
- Reuse existing integration-test style with `#[sqlx::test(migrations = "../../migrations")]`.

### File Structure Requirements

- Backend likely touchpoints:
  - `src/backend/src/routes/ctc.rs`
  - `src/backend/src/services/ctc_crypto.rs`
  - `src/backend/src/services/mod.rs`
  - `src/backend/src/services/*` (new revision/history service module if needed)
  - `src/backend/src/routes/mod.rs` (if new route registration needed)
- Database/migrations:
  - `migrations/*` for revision table and indexes
- Frontend likely touchpoints:
  - `src/frontend/src/pages/ctc.rs`
- Tests:
  - `src/backend/tests/ctc_tests.rs`
  - `src/backend/tests/ctc_encryption_tests.rs`
  - `src/backend/tests/rbac_tests.rs` (if history RBAC coverage added)

### Testing Requirements

- Add integration tests for revision creation, history retrieval, and RBAC enforcement.
- Add storage-level assertions that revision payloads and daily-rate values are not stored in plaintext.
- Add policy behavior tests for both effective-date modes.
- Keep existing Story 2.0/2.1 regression tests green; do not weaken current CTC endpoint guarantees.

### Previous Story Intelligence (2.1)

- Existing CTC create/calculate/update flows and frontend page are in place; build on these paths for minimal churn.
- Story 2.1 already enforced HR-only create/calculate and frontend route/menu RBAC; revision/history work must preserve that posture.
- Story 2.1 established validation and BPJS calculation patterns that should be reused rather than duplicated.

### Git Intelligence Summary

- Most recent work heavily touched `src/backend/src/routes/ctc.rs`, `src/backend/src/services/ctc_crypto.rs`, migrations, and CTC-focused tests.
- Team patterns favor focused changes with migration + integration test proof in the same story.
- Recent commits also show doc/status synchronization in `_bmad-output/implementation-artifacts/*`; keep that discipline.

### Latest Technical Information

- Axum 0.7 extractor guidance: keep body-consuming extractors last and centralize rejection/error handling for consistent API behavior.
- sqlx 0.7 testing guidance: continue `#[sqlx::test]` isolation and migration-backed integration tests.
- For this story, enforce transactionally consistent revision append + snapshot update to avoid split-brain history/current-state records.

### Project Structure Notes

- No structural conflicts detected.
- Story 2.2 should be an incremental extension of the existing CTC route/service/test architecture.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.2: CTC Revision Management]
- [Source: _bmad-output/planning-artifacts/prd.md#FR3]
- [Source: _bmad-output/planning-artifacts/prd.md#FR4]
- [Source: _bmad-output/planning-artifacts/prd.md#FR5]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR9]
- [Source: _bmad-output/planning-artifacts/architecture.md#Decision: CTC Data Protection (MVP)]
- [Source: _bmad-output/planning-artifacts/architecture.md#Decision: Daily Rate Calculation Strategy]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Core User Experience]
- [Source: _bmad-output/project-context.md#Critical Implementation Rules]
- [Source: _bmad-output/implementation-artifacts/2-1-employee-ctc-record-creation.md]
- [Source: _bmad-output/implementation-artifacts/2-0-ctc-encryption-foundation.md]
- [Source: https://docs.rs/axum/0.7.4/axum/extract/index.html]
- [Source: https://docs.rs/sqlx/latest/sqlx/attr.test.html]

### Story Creation Completion Note

- Ultimate context engine analysis completed - comprehensive developer guide created.

## Dev Agent Record

### Agent Model Used

openai/gpt-5.3-codex

### Debug Log References

- create-story workflow execution log (this session)

### Completion Notes List

- Story created with encryption-aware revision-management guardrails.
- Dependencies from Story 2.0 and Story 2.1 incorporated.
- Implemented CTC revisions table and API logic for history and diff generation.
- Integrated frontend UI with History panel, policy toggles, and diff displays.
- Review fixes: effective-date policy now resolves from `global_settings` and is applied deterministically.
- Review fixes: history/diff and frontend date rendering panic edges removed.
- Review fixes: expanded integration coverage for non-HR denial, ciphertext-at-rest assertions, and policy behavior.

### Change Log

- Date: 2026-02-22
- Changes: Implemented CTC revision persistence, history endpoint with diffs, frontend update UI.
- Date: 2026-02-22
- Changes: Applied adversarial code-review fixes (policy configuration/application, safety hardening, and expanded revision tests).

### File List

- _bmad-output/implementation-artifacts/2-2-ctc-revision-management.md
- migrations/20260222123816_ctc_revisions.up.sql
- migrations/20260222123816_ctc_revisions.down.sql
- src/backend/src/routes/ctc.rs
- src/backend/tests/ctc_revision_tests.rs
- src/frontend/src/pages/ctc.rs

## Senior Developer Review (AI)

### Review Date

- 2026-02-22

### Outcome

- Approve

### Findings Addressed

- [x] [HIGH] Implement configurable and applied effective-date policy behavior for revision updates.
- [x] [HIGH] Add missing integration evidence for non-HR denial, ciphertext storage, and policy semantics.
- [x] [HIGH] Reconcile story File List with actual migration filenames in git reality.
- [x] [MEDIUM] Remove panic-prone unwrap paths in backend history diff generation.
- [x] [MEDIUM] Remove panic-prone frontend date slicing in revision timeline rendering.
- [x] [MEDIUM] Synchronize review transparency with updated story record and test evidence.
