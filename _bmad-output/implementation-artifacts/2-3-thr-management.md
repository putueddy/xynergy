# Story 2.3: THR Management

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **HR Staff member**,
I want **the system to track THR (Tunjangan Hari Raya) religious holiday allowance**,
so that **compliance with Indonesian labor law is maintained**.

## Acceptance Criteria

1. **Given** I create or edit a CTC record **when** I navigate to the THR section **then** I can set THR eligibility and calculation basis.
2. **Given** THR is configured **when** the monthly accrual runs **then** the system accrues 1/12 of annual THR entitlement **and** displays accrued amount in the CTC summary.
3. **Given** it is THR payment month (typically before Eid) **when** I generate the THR report **then** the system shows total THR due per employee **and** includes calculation basis (1 month salary or prorated).

## Tasks / Subtasks

- [x] **Task 1: Add THR domain model and persistence fields** (AC: #1, #2)
  - [x] Extend CTC/revision schema for THR configuration: eligibility flag, calculation basis, service-month basis, accrual balance, last accrual period, payout period metadata.
  - [x] Keep sensitive THR amounts under existing CTC encryption model (ciphertext-at-rest with key metadata).
  - [x] Add migration indexes/constraints for period-based accrual and report queries.

- [x] **Task 2: Implement THR eligibility and calculation-basis logic** (AC: #1, #3)
  - [x] Support basis options: full (1 month salary + fixed allowances) and prorated (`service_months / 12 * basis_amount`).
  - [x] Compute service-months deterministically from employment start date and effective period cut-off.
  - [x] Validate basis inputs and reject impossible combinations.

- [x] **Task 3: Implement monthly THR accrual routine** (AC: #2)
  - [x] Add idempotent monthly accrual execution path (manual trigger for MVP, scheduler-ready interface).
  - [x] Accrue `annual_entitlement / 12` for eligible employees and write auditable accrual entries.
  - [x] Expose accrued amount in CTC summary endpoint without leaking raw component plaintext to non-HR roles.

- [x] **Task 4: Add THR payout report endpoint** (AC: #3)
  - [x] Add HR/Finance-authorized report endpoint for selected payment month.
  - [x] Return per-employee THR due, accrual-to-date, remaining/top-up, and basis explanation.
  - [x] Ensure filtering and pagination for operational scale.

- [x] **Task 5: Extend CTC UI for THR settings and visibility** (AC: #1, #2, #3)
  - [x] Add THR section to CTC create/edit form: eligibility toggle, basis selector, preview values.
  - [x] Show accrued THR in CTC summary panel.
  - [x] Add THR report UI entry with month selector and export-safe table display.

- [x] **Task 6: Preserve RBAC, audit, and encryption guardrails** (AC: #1, #2, #3)
  - [x] Keep THR component details HR-only; non-HR can only consume blended/allowed outputs.
  - [x] Log all THR configuration mutations, accrual runs, and report views in audit chain.
  - [x] Ensure no THR-sensitive plaintext is persisted in DB columns that are intended to be encrypted.

- [x] **Task 7: Add integration and calculation tests** (AC: #1, #2, #3)
  - [x] Integration tests for THR config create/update validation and RBAC denial paths.
  - [x] Accrual tests for monthly idempotency, 1/12 logic, and period roll-forward.
  - [x] Report tests for payout-month output and basis explanation correctness.
  - [x] Storage-level tests confirming ciphertext-at-rest for THR-sensitive values.

## Dev Notes

### Developer Context (Critical)

- Story 2.0 and 2.2 are now done and already introduced encryption + revision/history behavior in CTC flows.
- Story 2.3 must build directly on existing CTC route/service patterns rather than introducing parallel pipelines.
- Daily-rate and CTC components are already encrypted; THR-sensitive values must follow the same model.
- **Design correction (post-review):** `employment_start_date` lives on `resources` table (not `ctc_records`). It is a property of the person, not their CTC package. THR queries JOIN resources to read it. The `configure_thr` endpoint updates `resources.employment_start_date` separately from CTC config fields.

### Technical Requirements

- THR basis must be configurable per employee CTC context.
- Monthly accrual is mandatory and must be idempotent per employee+period.
- THR report must clearly explain basis and due amount for compliance reviews.
- IDR whole-number semantics remain mandatory for CTC component amounts.

### Architecture Compliance

- Keep Axum route conventions and `/api/v1` nesting.
- Keep handlers thin; move THR computations to service layer.
- Use `AppError` mappings; no production unwrap/expect.
- Keep audit hash-chain integration for THR mutations/reporting.
- Preserve defense-in-depth for sensitive CTC/THR data (field-level encryption + RBAC).

### Library / Framework Requirements

- Rust 1.75+, Axum 0.7, sqlx 0.7, PostgreSQL 15+, bigdecimal 0.4, validator 0.16.
- Continue existing CTC crypto stack (`aes-gcm`, base64, env-backed key provider abstraction).
- Keep `#[sqlx::test(migrations = "../../migrations")]` integration style.

### File Structure Requirements

- Backend likely touchpoints:
  - `src/backend/src/routes/ctc.rs`
  - `src/backend/src/services/ctc_crypto.rs`
  - `src/backend/src/services/ctc_calculator.rs` (or dedicated `thr_calculator.rs`)
  - `src/backend/src/services/mod.rs`
  - `src/backend/src/routes/mod.rs` (if route registration changes)
- Database/migrations:
  - `migrations/*` for THR config/accrual/report support fields/tables
- Frontend likely touchpoints:
  - `src/frontend/src/pages/ctc.rs`
- Tests:
  - `src/backend/tests/ctc_tests.rs`
  - `src/backend/tests/ctc_encryption_tests.rs`
  - `src/backend/tests/ctc_revision_tests.rs`
  - `src/backend/tests/*thr*.rs` (new)

### Testing Requirements

- Add deterministic THR formula tests for full and prorated eligibility.
- Add monthly accrual idempotency tests (rerun same period = no duplicate accrual).
- Add report output tests with basis explanations and payout totals.
- Add RBAC tests for HR/Finance visibility and non-HR denial.
- Add storage assertions ensuring THR-sensitive values are not plaintext at rest.

### Previous Story Intelligence (2.2)

- CTC revision history now exists and is fetched on demand in frontend; new THR changes should integrate with this history path.
- Effective-date policy handling was hardened and moved to `global_settings`; reuse this pattern for any THR policy toggles.
- Existing implementation used transactional snapshot+revision writes; THR updates should remain atomic in same transaction boundaries.

### Git Intelligence Summary

- Recent epic-2 changes are concentrated in `ctc.rs`, CTC migrations, and CTC integration tests.
- Team convention is to pair schema changes with integration tests and story artifact updates.
- Security hardening and RBAC fixes were validated through full backend test runs; preserve that bar.

### Latest Technical Information

- THR legal references commonly cite Indonesia Manpower Regulation No. 6/2016 with practical rule: full one-month wage for >=12 months service, prorated formula for <12 months.
- Industry guidance also emphasizes payout deadline near religious holiday period (commonly no later than H-7); encode this as report/context metadata and configurable business rule where possible.
- Continue BigDecimal/string-safe conversions and avoid float persistence for payroll amounts.

### Project Structure Notes

- No structural conflicts detected.
- Story 2.3 should be implemented as incremental extension of existing CTC encryption/revision infrastructure.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.3: THR Management]
- [Source: _bmad-output/planning-artifacts/prd.md#FR6]
- [Source: _bmad-output/planning-artifacts/prd.md#FR57]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR9]
- [Source: _bmad-output/planning-artifacts/architecture.md#Decision: CTC Data Protection (MVP)]
- [Source: _bmad-output/planning-artifacts/architecture.md#Decision: CTC Data Model]
- [Source: _bmad-output/project-context.md#Critical Implementation Rules]
- [Source: _bmad-output/implementation-artifacts/2-2-ctc-revision-management.md]
- [Source: _bmad-output/implementation-artifacts/2-0-ctc-encryption-foundation.md]
- [Source: https://www.aseanbriefing.com/news/religious-holiday-allowances-in-indonesia-obligations-for-businesses/]
- [Source: https://mapresourcesindonesia.com/understanding-the-religious-holiday-allowance-in-indonesia-a-guide/]

### Story Creation Completion Note

- Ultimate context engine analysis completed - comprehensive developer guide created.

## Dev Agent Record

### Agent Model Used

anthropic/claude-opus-4-6 (orchestrator) + delegated sub-agents

### Debug Log References

- create-story workflow execution log (original session)
- dev-story implementation session (this session)

### Completion Notes List

- Story 2.3 context assembled with Epic 2 continuity (2.0 encryption + 2.2 revision patterns).
- THR accrual/report requirements translated into concrete backend/frontend/test tasks.
- THR domain model (thr_calculator.rs) with 15 unit tests covering full/prorated calculation, service months, eligibility.
- THR routes (thr.rs, 779 lines) with 5 endpoints: configure, get config, run accrual, accrual history, payout report.
- All endpoints enforce HR-only RBAC, audit hash-chain logging, and AES-256-GCM encryption at rest.
- THR accrual idempotency via UNIQUE constraint + application-level skip + DB duplicate-key handling.
- Frontend THR Management page (thr.rs, 903 lines) with config, accrual, and report sections.
- 10 integration tests (thr_tests.rs) covering RBAC, config, accrual idempotency, report, and encryption-at-rest verification.
- Full regression suite: 43 tests pass (15 unit + 6 CTC + 8 RBAC + 4 RLS + 10 THR).
- Frontend compiles clean (only pre-existing warning in auth.rs).
- Code review completed: 11 findings (3 CRITICAL, 3 HIGH, 3 MEDIUM, 2 LOW). All CRITICAL and HIGH fixed.
- C1-C3 FIXED: Frontend JSON key mismatches in accrual history ("history"→"accruals") and payout report ("rows"→"entries", "basis"→"calculation_basis", "basis_explanation"→"calculation_basis_explanation").
- H1 FIXED: Added THR Monthly Accrual line to CTC Calculation Preview panel in ctc.rs.
- H2 FIXED: Wrapped accrual routine in database transaction (pool.begin/tx.commit) for atomicity.
- H3 FIXED: Migration default thr_eligible changed from TRUE to FALSE for safety.
- ALL MEDIUM/LOW FIXED: N+1 queries eliminated (M1, M2), amount assertions added to tests (M3), bd_to_i64 returns Result (L1), ThrCalculationBasis implements FromStr (L2).
- Full regression: 62 tests pass (19 unit + 5 audit + 2 auth + 3 encryption + 5 revision + 6 CTC + 8 RBAC + 4 RLS + 10 THR).
- Frontend and backend compile clean.
- Moved employment_start_date from ctc_records to resources table (design correction — it's a property of the person, not the CTC record).
- New migration 20260301090000_move_employment_start_date adds column to resources, migrates data, drops old column.
- Backend queries now JOIN resources for employment_start_date. N+1 queries replaced with batch pre-fetch (HashSet/HashMap).
- Frontend updated: employment_start_date signal renamed, response parsing uses new key.
- Tests updated: amount assertions verify exact THR calculations (16M basis, 1.33M accrual, 14.67M remaining top-up).

### File List

- _bmad-output/implementation-artifacts/2-3-thr-management.md
- migrations/20260301080000_thr_management.up.sql
- migrations/20260301080000_thr_management.down.sql
- src/backend/src/services/thr_calculator.rs
- src/backend/src/routes/thr.rs
- src/backend/src/services/mod.rs (modified)
- src/backend/src/routes/mod.rs (modified)
- src/backend/src/lib.rs (modified)
- src/backend/tests/thr_tests.rs
- src/frontend/src/pages/thr.rs
- src/frontend/src/pages/mod.rs (modified)
- src/frontend/src/lib.rs (modified)
- src/frontend/src/components/mod.rs (modified)
- _bmad-output/implementation-artifacts/sprint-status.yaml (modified)
- src/frontend/src/pages/ctc.rs (modified — H1: added THR accrual to CTC preview)
- migrations/20260301090000_move_employment_start_date.up.sql (new — adds employment_start_date to resources)
- migrations/20260301090000_move_employment_start_date.down.sql (new — rollback)

### Change Log

| Change | File(s) | Reason |
|--------|---------|--------|
| Added THR migration (up + down) | migrations/20260301080000_thr_management.* | Task 1: THR config columns on ctc_records + thr_accruals table |
| Created THR calculator service | src/backend/src/services/thr_calculator.rs | Task 1-2: Domain model, eligibility, full/prorated calculation, monthly accrual |
| Created THR route handlers | src/backend/src/routes/thr.rs | Task 2-4, 6: 5 endpoints with RBAC, audit, encryption |
| Wired THR routes into app | routes/mod.rs, lib.rs, services/mod.rs | Task 2-4: Module exports and route registration |
| Created THR frontend page | src/frontend/src/pages/thr.rs | Task 5: Config, accrual, report UI sections |
| Wired THR page into app | pages/mod.rs, lib.rs, components/mod.rs | Task 5: Route, module export, HR-only nav link |
| Created THR integration tests | src/backend/tests/thr_tests.rs | Task 7: 10 tests covering all endpoints and encryption |
| Updated story status | 2-3-thr-management.md, sprint-status.yaml | Step 9: Mark all tasks complete, status → review |
| Fixed frontend JSON key mismatches | src/frontend/src/pages/thr.rs | Code review C1-C3: accrual history, payout report, field names |
| Added THR to CTC preview panel | src/frontend/src/pages/ctc.rs | Code review H1: show accrued THR in CTC summary |
| Wrapped accrual in transaction | src/backend/src/routes/thr.rs | Code review H2: atomicity for accrual routine |
| Fixed migration default | migrations/20260301080000_thr_management.up.sql | Code review H3: thr_eligible defaults to false |
| Updated story status to done | 2-3-thr-management.md, sprint-status.yaml | Code review Step 5: all CRITICAL/HIGH resolved |
| Moved employment_start_date to resources | migrations/20260301090000_*, routes/thr.rs, thr_tests.rs, frontend/thr.rs | Design fix: employment_start_date belongs on person, not CTC record |
| Eliminated N+1 queries | src/backend/src/routes/thr.rs | M1/M2: batch pre-fetch with HashSet/HashMap |
| Added amount assertions to tests | src/backend/tests/thr_tests.rs | M3: verify exact THR calculations |
| bd_to_i64 returns Result | src/backend/src/routes/thr.rs | L1: proper error propagation instead of silent 0 |
| Implemented FromStr for ThrCalculationBasis | src/backend/src/services/thr_calculator.rs | L2: idiomatic Rust trait instead of ad-hoc method |
