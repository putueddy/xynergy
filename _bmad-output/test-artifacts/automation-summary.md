---
story: '5-3-ctc-validation-reports'
stepsCompleted:
  - 'step-01-preflight-and-context'
  - 'step-02-identify-targets'
  - 'step-03-generate-tests (sequential mode)'
  - 'step-04-validate-and-summarize'
lastStep: 'step-04-validate-and-summarize'
lastSaved: '2026-05-20'
mode: 'BMad-Integrated'
executionMode: 'sequential (inline generation, no subagent dispatch)'
detectedStack: 'fullstack'
testStackType: 'auto -> fullstack (Rust + Leptos detected)'
inputDocuments:
  - '_bmad-output/implementation-artifacts/5-3-ctc-validation-reports.md'
  - '_bmad-output/implementation-artifacts/sprint-status.yaml'
  - '_bmad-output/project-context.md'
  - 'src/backend/src/services/ctc_validation_report.rs'
  - 'src/backend/src/routes/ctc.rs'
  - 'src/backend/tests/ctc_validation_report_tests.rs'
  - 'migrations/20260307100000_add_payroll_validation_staging.up.sql'
---

# Test Automation Expansion - Story 5.3 CTC Validation Reports

## 1. Preflight & Context

### Stack Detection

- `test_stack_type: auto` -> resolved to **fullstack**
- Backend manifest: `Cargo.toml` (workspace), backend tests at `src/backend/tests/`
- Frontend manifest: `src/frontend/package.json` (Tailwind 4.1.x), Leptos 0.6 components
- No `playwright.config.*` or browser-test indicators found in repo — **API/backend-only profile**

### Execution Mode

**BMad-Integrated.** Story artifact `5-3-ctc-validation-reports.md` is present with full ACs, file list, and dev-notes context.

### Test Framework Verified

- Pattern: `#[sqlx::test(migrations = "../../migrations")]` (existing convention from `ctc_validation_tests.rs`, `audit_tests.rs`, `cash_flow_tests.rs`)
- Axum router exercised via `tower::ServiceExt::oneshot` with raw `Request`/`Body` (no HTTP socket)
- JSON body assertions via `serde_json::Value`
- Service-level unit tests live in `#[cfg(test)] mod tests` inside the service file

### Knowledge Loaded (core tier only)

- `test-levels-framework.md` — chose API + unit; no UI/browser
- `test-priorities-matrix.md` — P0 = AC + auth + audit; P1 = pagination + determinism + boundary; P2 = unit edge cases
- `data-factories.md` — reuse existing `create_user`, `create_resource`, `create_ctc_record`, `insert_payroll_*` helpers from `ctc_validation_report_tests.rs`
- `selective-testing.md` — avoid duplicating coverage that already exists; expand only the gaps
- `ci-burn-in.md` — keep new tests deterministic, no clock dependence, no flaky randomness
- `test-quality.md` — assert intent and structure, not just `is_number()`/`is_array()`

Playwright Utils, Pact.js Utils, and Pact MCP fragments **skipped** — `tea_use_playwright_utils=true` is configured but no browser tests exist in this repo and the story is backend-API + Leptos CSR (no E2E suite).

## 2. Initial Coverage Audit (pre-expansion baseline)

At the start of test-automation expansion, `src/backend/tests/ctc_validation_report_tests.rs` had **12 integration tests + 4 service unit tests**:

| # | Test                                                          | AC      | Level       |
| - | ------------------------------------------------------------- | ------- | ----------- |
| 1 | `finance_can_fetch_validation_report`                         | 1, 2    | Integration |
| 2 | `admin_can_fetch_validation_report`                           | 1       | Integration |
| 3 | `hr_denied_validation_report`                                 | 1       | Integration |
| 4 | `department_head_denied_validation_report`                    | 1       | Integration |
| 5 | `project_manager_denied_validation_report`                    | 1       | Integration |
| 6 | `inverted_date_range_returns_400`                             | 1       | Integration |
| 7 | `missing_payroll_staging_returns_400`                         | 1       | Integration |
| 8 | `report_returns_summary_counts_and_deterministic_rows`        | 2, 3    | Integration |
| 9 | `excluded_records_are_counted_when_missing_payroll`           | 2       | Integration |
| 10 | `bpjs_validation_uses_current_regulation_formula`            | 4       | Integration |
| 11 | `report_generation_creates_audit_log`                        | 1-4     | Integration |
| 12 | `existing_compliance_report_endpoint_unchanged` (regression) | (regr.) | Integration |
| u1 | `match_rate_zero_total`                                      | (calc)  | Unit        |
| u2 | `match_rate_perfect`                                         | (calc)  | Unit        |
| u3 | `match_rate_partial`                                         | (calc)  | Unit        |
| u4 | `jkk_rate_invalid_tier_rejected`                             | (calc)  | Unit        |

## 3. Coverage Gaps Identified

| Gap                                                                                                                       | AC       | Risk                                                              | Level       | Priority |
| ------------------------------------------------------------------------------------------------------------------------- | -------- | ----------------------------------------------------------------- | ----------- | -------- |
| **G1** Pagination `limit`/`offset` query params (not exercised at all)                                                    | 2, 3     | Detail truncation/skip bug ships unnoticed; story Task 3 promised | Integration | **P0**   |
| **G2** Summary metrics remain stable when pagination is applied to mismatches                                             | 2        | Finance sees wrong totals on paginated views                      | Integration | **P1**   |
| **G3** Mismatch row shape contract (AC #3 explicit fields: employee, field, xynergy_value, payroll_value, variance)       | 3        | Frontend drill-down breaks silently if API drops a field          | Integration | **P0**   |
| **G4** Variance amount is **absolute** value (not signed)                                                                 | 3        | Negative variance leaks; finance compares wrong direction         | Integration | **P1**   |
| **G5** `BPJS_REGULATION_ERROR` status differentiated from plain `DISCREPANCY`                                             | 4        | AC #4 'flag any calculation errors' wording                       | Integration | **P0**   |
| **G6** Audit payload **shape** (not just row count) — lean fields only, no sensitive values                               | (sec)    | Story-mandated security rule; quietly regressable                 | Integration | **P0**   |
| **G7** Missing JWT / unauthenticated request returns 401 on the new endpoint                                              | 1        | Auth bypass; not currently exercised on this route                | Integration | **P1**   |
| **G8** Date-boundary inclusivity (records exactly on `start_date` and `end_date`)                                         | 1        | Off-by-one in BETWEEN clause silently drops edge records          | Integration | **P1**   |
| **G9** Multiple effective dates per resource — service selects latest within range                                        | 2        | Service has explicit logic for this, untested                     | Integration | **P1**   |
| **G10** Match rate **numeric value** correctness end-to-end (not just `is_number()`)                                      | 2        | Display says 100% when 75%, easy regression                       | Integration | **P1**   |
| **G11** Idempotency: full summary numbers identical across repeated runs (not just mismatch array)                        | 2        | Counts could drift if any ordering/decryption noise exists        | Integration | **P2**   |
| **G12** Service unit: `compare_record` plain-field discrepancy emits canonical row in canonical order                     | 3        | Pure logic; cheap; isolates from DB                               | Unit        | **P2**   |
| **G13** Service unit: `compare_record` BPJS path returns `BPJS_REGULATION_ERROR` with `bpjs_metadata` when expected drifts | 4        | Pure logic; isolates regulation comparator                        | Unit        | **P2**   |

Out of scope (intentionally excluded):

- Frontend page tests for `ctc_validation.rs` — Leptos CSR; repo has no frontend test harness today (per Story 5.2 precedent). Manual verification continues to be the documented path.
- Cash flow / audit regression suites — already invoked by Task 6, no new tests needed; they remain re-run targets in the test plan.
- Full property-based fuzzing — overkill for a finance reconciliation report; deterministic table-driven inputs suffice.

## 4. Test Levels Selected

- **API/Integration (Axum + sqlx::test):** 11 new tests for G1–G11 — exercise the real DB + handler stack to match the existing convention.
- **Unit (in-module `#[cfg(test)]`):** 2 new tests for G12–G13 — exercise `compare_record` directly with fixture data, no DB.
- **E2E:** none. No Playwright/browser harness exists in repo and Story 5.3 does not introduce one.

## 5. Priority Assignment

- **P0** (must-have before merge):
  - G1 — pagination params honored
  - G3 — mismatch row contract
  - G5 — BPJS regulation-error status
  - G6 — audit payload shape
- **P1** (high signal, low cost):
  - G2 — summary stability under pagination
  - G4 — variance is absolute
  - G7 — 401 on missing auth
  - G8 — date-boundary inclusivity
  - G9 — latest payroll wins per resource
  - G10 — match rate numeric correctness
- **P2** (cheap unit coverage):
  - G11 — full-summary idempotency
  - G12, G13 — `compare_record` unit tests

## 6. Justification

**Scope: critical-paths + selective.** The story already shipped baseline integration coverage; this expansion targets only the gaps that (a) were promised by the story tasks (pagination, deterministic ordering, audit payload leanness, BPJS regulation error flag), (b) protect the AC #3 drill-down contract that the new finance UI directly depends on, or (c) close pure-logic edge cases at the unit level without adding DB cost.

No frontend automation is added because the repo has no frontend test harness and Story 5.2 set the precedent of manual frontend verification for finance pages.

## 7. Tests Generated

### Integration suite (`src/backend/tests/ctc_validation_report_tests.rs`)

11 new `#[sqlx::test(migrations = "../../migrations")]` integration tests appended:

| Test                                                          | Gap   | AC    | Priority |
| ------------------------------------------------------------- | ----- | ----- | -------- |
| `pagination_limit_offset_returns_subset`                      | G1    | 2,3   | P0       |
| `summary_metrics_stable_under_pagination`                     | G2    | 2     | P1       |
| `mismatch_row_contract_includes_canonical_fields`             | G3    | 3     | P0       |
| `variance_amount_is_absolute`                                 | G4    | 3     | P1       |
| `bpjs_regulation_error_status_distinct_from_discrepancy`      | G5    | 4     | P0       |
| `audit_log_payload_contains_only_lean_metadata`               | G6    | (sec) | P0       |
| `missing_auth_token_returns_401`                              | G7    | 1     | P1       |
| `date_boundary_records_are_included`                          | G8    | 1     | P1       |
| `latest_payroll_row_per_resource_wins`                        | G9    | 2     | P1       |
| `match_rate_percentage_is_accurate`                           | G10   | 2     | P1       |
| `full_report_is_idempotent_across_runs`                       | G11   | 2     | P2       |

Plus two new helpers reused by the new tests:

- `insert_payroll_with_higher_base()` — payroll value larger than xynergy (exercises absolute-variance invariant)
- `fetch_validation_report_paged()` — adds `limit`/`offset` query params

### Service unit suite (`src/backend/src/services/ctc_validation_report.rs::tests`)

2 new pure-logic unit tests appended:

| Test                                                          | Gap   | Priority |
| ------------------------------------------------------------- | ----- | -------- |
| `compare_record_emits_discrepancy_for_plain_field`            | G12   | P2       |
| `compare_record_flags_bpjs_regulation_error_with_metadata`    | G13   | P2       |

Both unit tests **passed** locally via `cargo test -p xynergy-backend --lib services::ctc_validation_report` (6/6 ok including the 4 pre-existing at generation time).

After subsequent Story 5.3 review patches, the final checked-in suite contains **36 integration tests** and **9 service unit tests**. The additional review-patch tests cover sampling validation/audit metadata, stale selected payroll baselines, payroll coverage, legacy encryption-metadata exclusions, payroll-basis BPJS checks, invalid risk-tier preservation, and missing BPJS field counting.

## 8. Validation Checklist

- [x] Framework readiness — `#[sqlx::test]` convention matches existing repo style
- [x] Coverage mapping — every new test maps to a gap (G1–G13) and to an AC or story task
- [x] Test quality — assertions are intent-based (status string, exact values, payload key-set), not `is_number()`/`is_array()` stubs
- [x] Fixtures/factories — reuse existing `create_user`, `create_resource`, `create_ctc_record`, `insert_payroll_*` helpers; only two narrow new helpers added
- [x] No CLI session orphans — sequential mode, no browser/Playwright used
- [x] Test artifacts in `_bmad-output/test-artifacts/` — automation summary plus traceability/gate files
- [x] Unit tests pass — final service unit suite verified locally after backend review patches with `cargo test -p xynergy-backend ctc_validation_report::tests --lib` (9/9)
- [x] Integration tests compiled — final `ctc_validation_report_tests` suite verified with `cargo test -p xynergy-backend --test ctc_validation_report_tests --no-run`
- [ ] Final integration execution — pending because `DATABASE_URL` was unavailable in the current review environment; earlier pre-review-patch execution passed 26/26 against live PostgreSQL
- [x] Story regression suites previously passed against live PostgreSQL: `ctc_validation_tests` (15/15), `audit_tests` (5/5), `cash_flow_tests` (31/31)
- [x] Production code changes accounted for — review patches intentionally changed Story 5.3 service, route, and page behavior after this automation artifact was first generated
- [x] Output schema preserved — automation-summary.md at the configured `test_artifacts` path

## 9. Files Created / Updated

| Path                                                                 | Change                                          |
| -------------------------------------------------------------------- | ----------------------------------------------- |
| `src/backend/tests/ctc_validation_report_tests.rs`                   | +11 integration tests during automation expansion; final story state has 36 integration tests after review patches |
| `src/backend/src/services/ctc_validation_report.rs`                  | +2 unit tests during automation expansion; final story state has 9 service unit tests after review patches |
| `_bmad-output/test-artifacts/automation-summary.md`                  | new — this document                             |

Production source modules, migrations, and frontend dependencies changed later as part of Story 5.3 implementation and review patches; this artifact now tracks those as final-story context rather than treating the test-generation pass as the only change source.

## 10. Key Assumptions & Risks

- **Assumption:** `payroll_validation_staging` schema (story migration `20260307100000`) is the source of truth — tests insert directly via SQL, matching the existing story helpers.
- **Assumption:** Existing `create_ctc_record` helper produces an Active record whose `effective_date` falls in the queried range (`2020-01-01`..`2030-12-31`). This matches the story's existing test pattern.
- **Risk (low):** `date_boundary_records_are_included` only weakly asserts that boundary employees are *visible* (compared OR excluded) because CTC `effective_date` is set by HR endpoint to today and that may not always be in the 2025 window. The payroll side, which is what the test really targets, is exercised via direct SQL insert at the exact boundary dates.
- **Risk (medium):** The final 36-test integration suite still requires a live Postgres instance for end-to-end execution. In the current review environment `DATABASE_URL` was unavailable, so the final suite was compile-checked with `--no-run` instead of executed.

## 11. Next Recommended Workflow

- `bmad-testarch-trace` — keep traceability matrix synchronized with the final 36 integration / 9 unit test inventory and the current conservative gate rationale.
- `bmad-testarch-test-review` — adversarial review of these tests before merge, focusing on whether the G6 audit-payload contract test is strict enough.
- Reviewer should run `cargo test -p xynergy-backend --test ctc_validation_report_tests` against a live Postgres before merging Story 5.3, then rerun the three regression suites if any shared CTC/audit/finance code changes again.
