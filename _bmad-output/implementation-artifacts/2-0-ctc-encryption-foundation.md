# Story 2.0: CTC Encryption Foundation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **Security-focused Engineering Team**,
I want **CTC sensitive fields encrypted with application-controlled keys before storing in PostgreSQL**,
so that **direct database reads by privileged DB admins do not reveal plaintext compensation data**.

## Acceptance Criteria

1. **Given** CTC data is persisted **when** salary/allowance/BPJS/THR values are written **then** sensitive fields are stored as ciphertext **and** plaintext values are not queryable from raw SQL table reads.
2. **Given** the application reads CTC for authorized HR workflows **when** the requester is HR and passes RBAC checks **then** data is decrypted in application/service layer only **and** decryption is denied for non-HR workflows.
3. **Given** key management is configured **when** encryption/decryption occurs **then** keys are sourced from centralized key management (Vault/KMS compatible) **and** key version metadata is stored for rotation support.
4. **Given** encryption migration is executed **when** existing plaintext CTC records are migrated **then** migration completes with integrity checks and rollback plan **and** no sensitive plaintext columns remain active after cutover.

## Tasks / Subtasks

- [x] **Task 1: Define cryptography and key-management contract** (AC: #2, #3)
  - [x] Add `CtcCryptoService` interface with explicit `encrypt_components` and `decrypt_components` methods.
  - [x] Introduce key provider abstraction (`KeyProvider`) for Vault/KMS compatibility; local dev fallback via env var only.
  - [x] Define encryption metadata contract: `encryption_version`, `key_version`, `algorithm`, `encrypted_at`.

- [x] **Task 2: Add encrypted storage schema for CTC sensitive fields** (AC: #1, #3)
  - [x] Create migration to add encrypted columns (bytea/text as appropriate) and encryption metadata columns in `ctc_records`.
  - [x] Preserve non-sensitive operational columns needed for joins/filtering and status management.
  - [x] Add constraints to ensure encrypted data and metadata are present for active records.

- [x] **Task 3: Implement encrypt-on-write/decrypt-on-read in backend CTC flows** (AC: #1, #2, #3)
  - [x] Update CTC create/update paths to persist only ciphertext for sensitive fields.
  - [x] Update CTC read/components/history endpoints to decrypt only for HR-authorized requests.
  - [x] Ensure non-HR paths (blended-rate consumers) do not invoke decryption for component fields.

- [x] **Task 4: Add migration/backfill and cutover safety** (AC: #4)
  - [x] Implement one-time backfill logic to encrypt any existing plaintext CTC records.
  - [x] Add dual-read validation window (read encrypted first, compare with legacy where present) and telemetry.
  - [x] Finalize cutover plan to disable plaintext columns and remove legacy reads after validation.

- [x] **Task 5: Enforce operational security controls and auditability** (AC: #2, #3, #4)
  - [x] Ensure no plaintext CTC values are logged in app logs, SQL logs, or audit payloads.
  - [x] Add audit trail entries for key events (key version used, decrypt attempts, migration batches).
  - [x] Implement fail-closed behavior when key retrieval/decryption fails on sensitive endpoints.

- [x] **Task 6: Add security-focused tests and verification evidence** (AC: #1, #2, #3, #4)
  - [x] Integration tests: direct DB select returns ciphertext/non-readable values for sensitive fields.
  - [x] Integration tests: HR can read decrypted components; non-HR gets forbidden/no component plaintext.
  - [x] Integration tests: key-rotation compatibility with old/new `key_version`.
  - [x] Migration tests: plaintext-to-ciphertext backfill completes and preserves business-calculation equivalence.

## Dev Notes

### Developer Context (Critical)

- Story 2.0 is now the security gate for Epic 2. Do not start Story 2.2+ until this is complete.
- Story 2.1 already shipped CTC CRUD + BPJS logic with plaintext numeric storage; this story must harden that storage without breaking existing CTC behavior.
- Requirement target is explicit DBA-read protection: `SELECT` from CTC table must not expose plaintext compensation fields.

### Technical Requirements

- Keep all existing RBAC guarantees: CTC components remain HR-only.
- Encryption must be application-controlled (key not stored in DB).
- Preserve deterministic business behavior for BPJS/THR/daily rate outputs at API level.
- Keep IDR whole-number semantics and current validation behavior from Story 2.1.
- Key rotation readiness is required at schema + service level (store and honor `key_version`).

### Architecture Compliance

- Follow Axum route composition patterns in `routes/mod.rs` and `lib.rs`.
- Keep handlers thin; place crypto logic in service layer (new module under `src/backend/src/services/`).
- Continue `AppError` mapping; no production `unwrap`/`expect`.
- Continue audit hash-chain integration for mutation and security-relevant operations.

### Library / Framework Requirements

- Stack baseline: Rust 1.75+, Axum 0.7, sqlx 0.7, PostgreSQL 15+, bigdecimal 0.4.
- Prefer AEAD encryption primitives and authenticated ciphertext format.
- Key management must support Vault/KMS compatibility; local development can use env-backed key provider.
- Do not rely on DB-side decryption functions for app responses in this story.

### File Structure Requirements

- Primary backend touchpoints:
  - `src/backend/src/routes/ctc.rs`
  - `src/backend/src/services/mod.rs`
  - `src/backend/src/services/ctc_calculator.rs` (ensure compatibility only)
  - `src/backend/src/services/ctc_crypto.rs` (new)
  - `src/backend/src/services/key_provider.rs` (new or equivalent)
  - `src/backend/tests/ctc_tests.rs`
- Database/migrations:
  - `migrations/*` new migration for encrypted CTC columns + metadata + backfill support
- Config/environment:
  - backend env handling for key provider configuration

### Testing Requirements

- Use `#[sqlx::test(migrations = "../../migrations")]` for integration coverage.
- Add explicit assertions that DB column reads are ciphertext/non-plaintext for sensitive fields.
- Cover RBAC edge cases for decryption endpoints (HR allowed, admin/others denied where required).
- Add migration test path for existing records created by Story 2.1 schema.

### Previous Story Intelligence (2.1)

- Reuse existing CTC API surface where possible to avoid client contract churn.
- Preserve strict HR-only create/calculate behavior and existing route guard semantics.
- Preserve decimal-safe daily-rate behavior and current BPJS calculation service.
- Keep 2.1 test patterns and extend them with encryption assertions rather than replacing wholesale.

### Git Intelligence Summary

- Current codebase follows route/service/migration/test incremental changes.
- Continue with focused, auditable changes; avoid sweeping refactors in unrelated modules.

### Latest Technical Information

- PostgreSQL docs distinguish TDE/storage encryption vs column-level protections for insider/admin-read threats.
- OWASP guidance requires threat-model-based crypto placement plus lifecycle key management (generation, rotation, revocation, compromise response).
- pgcrypto can encrypt columns, but key exposure model must still be controlled; this story keeps decryption decisions in app service layer under RBAC.

### Project Structure Notes

- No structural conflicts detected.
- This story should be implemented as a hardening layer over existing Story 2.1 CTC flows.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.0: CTC Encryption Foundation]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.1: Employee CTC Record Creation]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR9]
- [Source: _bmad-output/planning-artifacts/architecture.md#Decision: CTC Data Protection (MVP)]
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-2026-02-22.md]
- [Source: _bmad-output/project-context.md#Critical Implementation Rules]
- [Source: _bmad-output/implementation-artifacts/2-1-employee-ctc-record-creation.md]
- [Source: https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html]
- [Source: https://cheatsheetseries.owasp.org/cheatsheets/Key_Management_Cheat_Sheet.html]
- [Source: https://www.postgresql.org/docs/15/encryption-options.html]
- [Source: https://www.postgresql.org/docs/15/pgcrypto.html]

### Story Creation Completion Note

- Ultimate context engine analysis completed - comprehensive developer guide created.

## Dev Agent Record

### Agent Model Used

Antigravity

### Debug Log References

- Handled AES-256-GCM cryptography securely with a 96-bit random nonce encoded alongside the ciphertext via base64 for direct text persistence at scale.
- Solved missing `rand::RngCore` dependency trait resolution explicitly through imports.
- Tightened RBAC/decryption boundaries so non-HR workflows cannot trigger CTC component decryption or mutation.

### Completion Notes List

- Designed and implemented `KeyProvider` interface (via env abstraction `EnvKeyProvider`).
- Integrated `CtcCryptoService` with fully authenticated DB storage using `encrypted_components`, and preserved operational indexes (`daily_rate`).
- Removed plaintext writes from endpoint payloads entirely in favor of strict ciphertext writes.
- Cleaned up audit log payloads so raw CTC components aren't accidentally exposed in plaintext audit streams. 
- Added migration integrity constraint (`chk_ctc_encryption_metadata_consistent`) and shifted to non-destructive cutover-safe migration.
- Added key-version rotation compatibility integration test and validated full backend test suite.

### File List

- `src/backend/Cargo.toml` (AES primitives base64)
- `src/backend/src/services/ctc_crypto.rs`
- `src/backend/src/services/key_provider.rs`
- `src/backend/src/services/mod.rs`
- `src/backend/src/routes/ctc.rs`
- `src/backend/tests/ctc_encryption_tests.rs`
- `src/backend/tests/ctc_tests.rs`
- `src/backend/tests/audit_tests.rs`
- `migrations/20260222160000_enc_ctc_columns.up.sql`
- `migrations/20260222160000_enc_ctc_columns.down.sql`
