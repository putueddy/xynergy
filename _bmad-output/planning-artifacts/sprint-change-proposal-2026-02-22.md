# Sprint Change Proposal

Date: 2026-02-22
Workflow: correct-course
Trigger Story: Epic 2 / Story 2.1 (Employee CTC Record Creation)
Mode: Batch

## 1) Issue Summary

### Problem Statement
The current design protects CTC data with PostgreSQL TDE (data-at-rest) and RBAC/RLS, but this does not prevent plaintext visibility to privileged database administrators querying tables directly.

The newly confirmed requirement is to enforce database-level protection such that direct table reads do not reveal plaintext salary/allowance/BPJS/THR values.

### Why This Change Is Needed Now
- CTC is highly sensitive compensation data.
- Compliance posture (ISO 27001 control family for cryptography and key management) requires stronger, auditable cryptographic controls than disk-level encryption alone.
- This must be in place before continuing the next Epic 2 story to avoid rework and security debt.

### Evidence / External Best-Practice Inputs (Exa Research)
- OWASP Cryptographic Storage Cheat Sheet: encryption layer choice must follow threat model; TDE/hardware encryption does not protect from privileged server/database compromise.
- OWASP Key Management Cheat Sheet: centralized key lifecycle management, rotation, storage segregation, and compromise handling are mandatory.
- PostgreSQL Encryption Options docs: PostgreSQL explicitly distinguishes encryption options for threats including "unscrupulous administrators" and column-specific encryption needs.
- PostgreSQL pgcrypto docs: supports column encryption primitives; but key exposure model must be handled carefully.

## 2) Impact Analysis

### Epic Impact
- Epic 2 (CTC Management): materially impacted; CTC create/read/update/history must use encrypted storage for sensitive fields.
- Epic 1 (Security & RBAC Foundation): impacted by adding key management controls and operational security controls.
- Epic 3+ indirect: blended-rate consumers must remain unaffected (they should never need to decrypt CTC components).

### Story Impact
- Story 2.1 must include encrypted persistence acceptance criteria (not only RBAC).
- Story 2.2 (revision management) must preserve encrypted before/after data handling.
- New foundational story is required before any further Epic 2 functional expansion:
  - Story 2.0 (new): CTC cryptography foundation (keys, encrypted columns, migration, envelope scheme).

### Artifact Conflicts
- PRD currently states TDE-only for CTC at-rest security (insufficient for DBA plaintext threat).
- Architecture currently defers app-level encryption to Post-MVP (conflicts with new requirement).
- UX has minimal impact (no major flow change), but should include explicit role-safe messaging and non-disclosure behavior for sensitive fields.

### Technical Impact
- Database schema: add encrypted bytea/text columns for CTC sensitive components.
- Service layer: encrypt on write/decrypt on read for HR-only paths.
- Key management: introduce KMS/Vault-backed DEK/KEK model (or env-backed interim for local dev only).
- Migration: transform existing plaintext CTC data to encrypted form with reversible rollback strategy.
- Observability: prevent sensitive plaintext from logs/traces and query debugging.

## 3) Recommended Approach

### Selected Path
Hybrid: Direct Adjustment + MVP Security Scope Update

- Direct Adjustment: add a security foundation story and revise existing Epic 2 stories/criteria.
- MVP Scope Update: move field-level CTC encryption from Post-MVP into current sprint gating for Epic 2 continuation.

### Rationale
- Meets the explicit DBA threat model.
- Prevents security architecture drift and expensive retrofitting.
- Preserves existing business logic (BPJS/THR) while hardening storage.

### Effort / Risk / Timeline
- Effort: Medium-High
- Risk: Medium (crypto/key management complexity, migration correctness)
- Timeline Impact: +3 to +5 development days before next Epic 2 feature story

## 4) Detailed Change Proposals

### A) Epics/Stories Updates

#### Proposal A1 - Add New Story Before Remaining Epic 2 Stories

Story: Epic 2 - new Story 2.0 "CTC Encryption Foundation"
Section: Story list and sequence

OLD:
- Story 2.1 starts CTC implementation sequence

NEW:
- Story 2.0: CTC Encryption Foundation (must complete first)
- Story 2.1+ proceed only after 2.0 acceptance is met

Rationale:
- Enforces security baseline before additional CTC feature expansion.

#### Proposal A2 - Story 2.1 Acceptance Criteria Extension

Story: 2.1 Employee CTC Record Creation
Section: Acceptance Criteria

OLD:
- Record created with status=Active, daily rate calculated, audit logged

NEW:
- Sensitive CTC components are encrypted before database persistence.
- Direct SQL read of encrypted columns does not expose plaintext values.
- Decryption is only performed in HR-authorized application paths.

Rationale:
- Aligns functional creation flow with confidentiality requirement.

### B) PRD Updates

#### Proposal B1 - Security NFR Clarification

Artifact: PRD
Section: NFR9 and Security section

OLD:
- NFR9: All CTC data encrypted at rest using PostgreSQL TDE

NEW:
- NFR9: CTC data protected with defense-in-depth:
  - storage encryption at rest (TDE/disk)
  - application-controlled field-level encryption for sensitive CTC columns
  - plaintext not readable via direct DB table queries by DB administrators
- Add explicit key-management NFRs:
  - centralized key custody (KMS/Vault)
  - key versioning and rotation
  - encryption context metadata and auditable decrypt access

Rationale:
- TDE protects media theft; field encryption addresses privileged DB read risk.

### C) Architecture Updates

#### Proposal C1 - Replace TDE-only CTC Decision

Artifact: Architecture
Section: "CTC Data Protection (MVP)"

OLD:
- MVP: PostgreSQL TDE only
- App-level encryption Post-MVP

NEW:
- MVP (now): TDE + application-controlled field-level encryption for CTC sensitive columns
- Encryption design:
  - Encrypt fields: base_salary, allowances, BPJS, THR, total_monthly_ctc, daily_rate (or daily_rate policy-based if required for non-HR views)
  - Store ciphertext + key_version + algorithm metadata
  - Compute blended rate for non-HR views from derived non-sensitive projection or controlled service output
- Key hierarchy:
  - KEK in Vault/KMS
  - per-environment DEK wrapped by KEK
  - no keys in database

Rationale:
- Aligns architecture with confirmed compliance/security threat model.

#### Proposal C2 - Migration and Runtime Model

Artifact: Architecture
Section: Data migration + operational controls

OLD:
- No explicit encrypted-column migration plan

NEW:
- Add migration runbook:
  1) add encrypted columns
  2) backfill ciphertext in batches
  3) dual-read validation
  4) cutover reads/writes to encrypted columns
  5) remove plaintext columns after validation window
- Add operational controls:
  - fail-closed if decryption key unavailable
  - block plaintext debug logging
  - auditable decrypt events

Rationale:
- Reduces outage and data-loss risk during crypto transition.

### D) UX Specification Updates

#### Proposal D1 - Security UX behavior note

Artifact: UX Spec
Section: Security/access patterns

OLD:
- HR-only visibility described; no explicit encrypted storage UX language

NEW:
- Add UX behavior note:
  - Non-HR users never see CTC component values.
  - HR flows may show values only through authorized app screens.
  - Error states for key unavailability: clear "sensitive data temporarily unavailable" messaging.

Rationale:
- Keeps UX aligned with security and operational behavior.

## 5) Implementation Handoff

### Scope Classification
Major (security architecture baseline change across PRD + architecture + epic sequencing)

### Handoff Recipients and Responsibilities
- Product Manager / Architect:
  - approve cryptography architecture shift (MVP scope update)
  - approve key management standard and migration policy
- Product Owner / Scrum Master:
  - insert Story 2.0 before remaining Epic 2 stories
  - resequence backlog and sprint status
- Development Team:
  - implement encryption/decryption service
  - schema migration/backfill/cutover
  - regression tests and performance checks
- QA / Security:
  - verify direct DB queries do not expose plaintext
  - verify HR-only decryption paths and audit records

### Success Criteria for Handoff Completion
- Direct DB SELECT on CTC sensitive columns returns ciphertext only.
- HR authorized APIs function normally; non-HR never receive sensitive fields.
- Key rotation tested with at least one version transition.
- Audit evidence exists for decrypt operations and migration cutover.

## 6) Compliance Validation Approach

- Control mapping:
  - ISO 27001 Annex A 8.24 (use of cryptography)
  - ISO 27001 Annex A 8.11 (data masking where applicable)
- Evidence artifacts:
  - approved cryptography policy excerpt for CTC data
  - key management SOP (generation, rotation, revocation)
  - migration report and validation checks
  - security test report showing DBA plaintext non-exposure

## 7) Proposed Backlog Actions (Immediate)

1. Add Story 2.0 CTC Encryption Foundation (backlog -> in-progress).
2. Update Story 2.1 acceptance criteria to require encrypted persistence and plaintext non-exposure.
3. Update PRD NFR9 + add key management NFR entries.
4. Update architecture decision from TDE-only to defense-in-depth (TDE + field encryption).
5. Gate next Epic 2 feature story on Story 2.0 completion.
