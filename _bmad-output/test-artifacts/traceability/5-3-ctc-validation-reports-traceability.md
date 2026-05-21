---
stepsCompleted:
  - step-01-load-context
  - step-02-discover-tests
  - step-03-map-criteria
  - step-04-analyze-gaps
  - step-05-gate-decision
lastStep: step-05-gate-decision
lastSaved: '2026-05-20'
storyId: 5.3
storyKey: 5-3-ctc-validation-reports
storyTitle: 'CTC Validation Reports'
coverageBasis: acceptance_criteria
oracleConfidence: high
oracleResolutionMode: formal_requirements
oracleSources:
  - _bmad-output/implementation-artifacts/5-3-ctc-validation-reports.md
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/prd.md
externalPointerStatus: not_used
tempCoverageMatrixPath: /tmp/tea-trace-coverage-matrix-5-3-2026-05-20.json
gateDecision: CONCERNS
gateEligible: true
collectionStatus: COLLECTED
---

# Traceability Report — Story 5.3 (CTC Validation Reports)

## Gate Decision: **CONCERNS** (final live-DB execution pending)

**Rationale:** P0 coverage is mapped and Story 5.3 review decisions are
resolved. The gate remains conservative because the final post-review
integration inventory now contains 36 `sqlx::test` cases, and the current
review environment did not expose `DATABASE_URL` to execute that final suite
end-to-end. Final service unit tests, frontend build, frontend check, and
whitespace checks passed; the integration suite was compile-checked with
`--no-run`.

**Caveats noted for reviewer:**

1. Execute `cargo test -p xynergy-backend --test ctc_validation_report_tests`
   against a live PostgreSQL database before merge to validate the final
   36-test integration inventory.
2. Frontend page (`ctc_validation.rs`) has no committed E2E suite; route smoke
   and Leptos/Tailwind checks passed, but automated coverage for the
   keyboard-accessible mismatch drilldown (AC #3) is recommended before
   production hardening.

---

## Coverage Oracle

- **Resolution mode:** formal_requirements (story acceptance criteria)
- **Confidence:** high
- **Sources:**
  - `_bmad-output/implementation-artifacts/5-3-ctc-validation-reports.md` (story file with 4 ACs)
  - `_bmad-output/planning-artifacts/epics.md` (Epic 5)
  - `_bmad-output/planning-artifacts/prd.md` (FR49, FR56-FR57, NFR19-NFR20)
- **External pointer status:** not_used
- **Synthetic oracle:** no

---

## Test Inventory

| Category | File | Count | Notes |
|----------|------|-------|-------|
| API/integration | `src/backend/tests/ctc_validation_report_tests.rs` | 36 | `#[sqlx::test]` against live PostgreSQL with migrations; final suite compile-checked with `--no-run` in this environment |
| Unit | `src/backend/src/services/ctc_validation_report.rs` (`mod tests`) | 9 | Pure-Rust unit tests for `match_rate`, JKK/risk-tier behavior, BPJS basis checks, and `compare_record` |
| Frontend | `src/frontend/src/pages/ctc_validation.rs` | 0 | No committed automated tests; route smoke plus Leptos/Tailwind checks are documented in the story record |
| **Total active** |  | **45** |  |

**By level:**
- API: 36 tests
- Unit: 9 tests
- Component: 0
- E2E: 0

**Skipped/pending/fixme:** 0

---

## Traceability Matrix (AC → Tests)

### AC #1 — Finance navigates to CTC Validation, selects date range, runs report; system compares Xynergy CTC vs payroll records

- **Priority:** P0 (primary finance workflow + revenue/compliance-critical access control)
- **Coverage:** FULL (backend); PARTIAL (frontend — manual only)
- **Mapped tests:**

| Test ID | Title | File | Level | Status |
|---------|-------|------|-------|--------|
| 5.3-API-001 | `finance_can_fetch_validation_report` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-002 | `admin_can_fetch_validation_report` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-003 | `hr_denied_validation_report` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-004 | `department_head_denied_validation_report` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-005 | `project_manager_denied_validation_report` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-006 | `inverted_date_range_returns_400` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-007 | `missing_payroll_staging_returns_400` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-019 | `missing_auth_token_returns_401` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-020 | `date_boundary_records_are_included` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-024 | `sampled_run_returns_only_picked_employees` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-025 | `sampled_run_with_invalid_id_returns_400` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-030 | `stale_payroll_data_returns_400` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-031 | `one_recent_payroll_row_does_not_hide_stale_selected_baseline` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-032 | `sampled_run_requires_payroll_for_sampled_ids` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-033 | `sampled_run_rejects_mixed_sample_with_missing_payroll` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |

- **Heuristics:**
  - Endpoint coverage: PRESENT (`GET /api/v1/ctc/validation-report`)
  - Auth/authz negative paths: PRESENT (4 denied roles + missing token)
  - Error-path coverage: PRESENT (inverted dates + missing staging)
  - UI journey E2E: MISSING (frontend page has no Playwright/integration coverage)

### AC #2 — Comparison results show Match Rate %, Discrepancy Count, List of mismatches

- **Priority:** P0 (primary deliverable)
- **Coverage:** FULL
- **Mapped tests:**

| Test ID | Title | File | Level | Status |
|---------|-------|------|-------|--------|
| 5.3-API-008 | `report_returns_summary_counts_and_deterministic_rows` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-009 | `excluded_records_are_counted_when_missing_payroll` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-013 | `pagination_limit_offset_returns_subset` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-014 | `summary_metrics_stable_under_pagination` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-021 | `latest_payroll_row_per_resource_wins` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-027 | `latest_import_wins_when_payroll_effective_date_ties` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-022 | `match_rate_percentage_is_accurate` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-023 | `full_report_is_idempotent_across_runs` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-028 | `payroll_only_records_are_reported_as_excluded` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-034 | `payroll_coverage_pct_surfaces_missing_employees` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-035 | `legacy_ctc_rows_missing_encryption_metadata_are_excluded` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-UNIT-001 | `match_rate_zero_total` | `src/backend/src/services/ctc_validation_report.rs` | unit | active |
| 5.3-UNIT-002 | `match_rate_perfect` | `src/backend/src/services/ctc_validation_report.rs` | unit | active |
| 5.3-UNIT-003 | `match_rate_partial` | `src/backend/src/services/ctc_validation_report.rs` | unit | active |
| 5.3-UNIT-008 | `invalid_risk_tier_still_preserves_plain_field_mismatches` | `src/backend/src/services/ctc_validation_report.rs` | unit | active |

- **Heuristics:**
  - Match Rate % math correctness: PRESENT (5.3-API-022 + unit tests)
  - Deterministic ordering: PRESENT (5.3-API-008, 5.3-API-023)
  - Excluded count surfacing: PRESENT (5.3-API-009)
  - Pagination summary stability: PRESENT (5.3-API-014)

### AC #3 — Click mismatch → Employee, Field, Xynergy Value, Payroll Value, Variance

- **Priority:** P0 (primary drill-down for reconciliation)
- **Coverage:** FULL (backend contract); PARTIAL (frontend drill-down UI — no automated keyboard/aria assertions)
- **Mapped tests:**

| Test ID | Title | File | Level | Status |
|---------|-------|------|-------|--------|
| 5.3-API-015 | `mismatch_row_contract_includes_canonical_fields` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-016 | `variance_amount_is_absolute` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-UNIT-005 | `compare_record_emits_discrepancy_for_plain_field` | `src/backend/src/services/ctc_validation_report.rs` | unit | active |
| 5.3-UNIT-008 | `invalid_risk_tier_still_preserves_plain_field_mismatches` | `src/backend/src/services/ctc_validation_report.rs` | unit | active |

- **Heuristics:**
  - Canonical row contract (`employee_id`, `employee_name`, `field_name`, `xynergy_value`, `payroll_value`, `variance_amount`, `status`): PRESENT
  - `variance_amount` always absolute: PRESENT
  - Frontend keyboard-reachable drilldown (`aria-expanded`, `aria-controls`, focus): MISSING automated assertion (story Task 4 documents the requirement; coverage is manual only)

### AC #4 — BPJS sample validation flags calculation errors against regulation

- **Priority:** P0 (compliance-critical)
- **Coverage:** FULL (backend); PARTIAL (frontend sample selector — optional + manual only)
- **Mapped tests:**

| Test ID | Title | File | Level | Status |
|---------|-------|------|-------|--------|
| 5.3-API-010 | `bpjs_validation_uses_current_regulation_formula` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-026 | `bpjs_validation_respects_created_risk_tier` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-017 | `bpjs_regulation_error_status_distinct_from_discrepancy` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-024 | `sampled_run_returns_only_picked_employees` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-025 | `sampled_run_with_invalid_id_returns_400` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-032 | `sampled_run_requires_payroll_for_sampled_ids` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-033 | `sampled_run_rejects_mixed_sample_with_missing_payroll` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-UNIT-004 | `jkk_rate_invalid_tier_rejected` | `src/backend/src/services/ctc_validation_report.rs` | unit | active |
| 5.3-UNIT-006 | `compare_record_flags_bpjs_regulation_error_with_metadata` | `src/backend/src/services/ctc_validation_report.rs` | unit | active |
| 5.3-UNIT-007 | `compare_record_uses_payroll_basis_for_payroll_bpjs_regulation` | `src/backend/src/services/ctc_validation_report.rs` | unit | active |
| 5.3-UNIT-008 | `invalid_risk_tier_still_preserves_plain_field_mismatches` | `src/backend/src/services/ctc_validation_report.rs` | unit | active |
| 5.3-UNIT-009 | `missing_xynergy_bpjs_field_counts_regulation_error` | `src/backend/src/services/ctc_validation_report.rs` | unit | active |

- **Heuristics:**
  - BPJS recalculation reuses `calculate_bpjs()` from `compliance_report.rs`: PRESENT (5.3-API-010)
  - Distinct `BPJS_REGULATION_ERROR` status surface: PRESENT (5.3-API-017)
  - BPJS metadata (`risk_tier`, `recalculated_value`) attached: PRESENT (5.3-API-017, 5.3-UNIT-006)
  - JKK tier validation: PRESENT (5.3-UNIT-004)

### Non-AC (Task 3 / Task 6 contract) — Audit logging & compliance-report regression

- **Priority:** P0 (security / regression guardrail)
- **Coverage:** FULL
- **Mapped tests:**

| Test ID | Title | File | Level | Status |
|---------|-------|------|-------|--------|
| 5.3-API-011 | `report_generation_creates_audit_log` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-018 | `audit_log_payload_contains_only_lean_metadata` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-012 | `existing_compliance_report_endpoint_unchanged` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-029 | `audit_log_records_sampled_metadata` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |
| 5.3-API-036 | `duplicate_sampled_ids_are_deduped_in_audit` | `src/backend/tests/ctc_validation_report_tests.rs` | api | active |

- **Heuristics:**
  - Audit entry created with `ctc_validation_report_generated` action: PRESENT
  - Audit payload restricted to allowed metadata keys (no mismatch rows / decrypted values): PRESENT
  - `/api/v1/ctc/compliance-report` audit action unchanged: PRESENT

---

## Coverage Statistics

| Metric | Value |
|--------|-------|
| Total requirements (ACs) | 4 |
| Fully covered | 4 |
| Partially covered | 0 |
| Uncovered | 0 |
| **Overall coverage %** | **100%** |

### Priority Breakdown

| Priority | Total | Covered | % |
|----------|-------|---------|---|
| P0 | 4 | 4 | 100% |
| P1 | 0 | 0 | n/a |
| P2 | 0 | 0 | n/a |
| P3 | 0 | 0 | n/a |

### Coverage Heuristics

| Heuristic | Status | Count of gaps |
|-----------|--------|---------------|
| Endpoints without tests | PRESENT | 0 |
| Auth/authz missing negative paths | PRESENT | 0 |
| Error-path / validation paths | PRESENT | 0 |
| UI journey E2E (frontend page) | MANUAL_ONLY | 1 (`/finance/ctc-validation`; route smoke documented, no committed E2E suite) |
| UI state coverage (loading/empty/error/permission) | MANUAL_ONLY | 1 (no committed automated assertions) |

---

## Gap Analysis

### Critical (P0) gaps

None at the API/contract level. Frontend gaps are advisory only because the
story Testing Requirements explicitly designate frontend verification as
**manual**.

### Frontend coverage debt (advisory)

| Item | Why it matters | Recommended action |
|------|----------------|--------------------|
| Keyboard-accessible drilldown (AC #3 — `aria-expanded`, `aria-controls`, visible focus) | Story Task 4 explicitly requires this; only documented checklist exists | Add Playwright/`/qa` smoke covering expand/collapse + Tab order |
| BPJS sample selector (AC #4 optional UI surface) | Story Task 4 mentions an optional sample selector; behavior is untested | Add UI smoke if/when the selector is implemented |
| Summary cards stable while details paginate | Backend invariant 5.3-API-014 holds; UI assertion missing | Add UI smoke that paginates details and asserts cards unchanged |

### Execution-readiness gaps (block release approval)

| Item | Risk | Recommended action |
|------|------|--------------------|
| Final 36-test integration suite not executed in current review environment | Tests may fail against live Postgres despite `--no-run` compile success | Run `cargo test -p xynergy-backend --test ctc_validation_report_tests` with `DATABASE_URL` before merge |
| Regression suites not rerun after the final backend/frontend review patches in this environment | Shared CTC/audit code reuse could regress | Re-run `ctc_validation_tests`, `audit_tests`, `cash_flow_tests` if release policy requires final live-DB evidence |
| Frontend automated E2E absent | Keyboard/a11y drilldown behavior is verified by code review/manual smoke rather than committed browser tests | Add Playwright/route smoke when the repo gains a frontend test harness |

---

## Recommendations

| Priority | Action | Targets |
|----------|--------|---------|
| HIGH | Execute the final 36-test `ctc_validation_report_tests.rs` suite and the three regression suites against a live PostgreSQL before merge | 5.3-API-001..036 |
| MEDIUM | Add an E2E smoke for `/finance/ctc-validation` covering: report run, mismatch expand/collapse keyboard reachability, summary card stability under pagination | AC #2, AC #3 |
| MEDIUM | Add a UI smoke for the BPJS sample selector when the repo has a frontend test harness | AC #4 |
| LOW | Consider `/bmad-testarch-test-review` after merge to assess test quality (assertion strength, fixture hygiene) | All |
| LOW | Treat the existing manual verification checklist in story Testing Requirements as the authoritative pre-merge UI gate | All UI |

---

## Gate Criteria Evaluation

| Criterion | Required | Actual | Status |
|-----------|----------|--------|--------|
| P0 coverage | 100% | 100% | MET |
| P1 coverage (target) | 90% | n/a (no P1) | MET |
| P1 coverage (minimum) | 80% | n/a (no P1) | MET |
| Overall coverage | >=80% | 100% | MET |

---

## Machine-Readable Summary (`e2e-trace-summary.json` equivalent)

```json
{
  "schema_version": "0.1.0",
  "snapshot_at": "2026-05-20T00:00:00Z",
  "repo": "xynergy",
  "collection_mode": "contract_static",
  "collection_status": "COLLECTED",
  "inventory_basis": "acceptance_criteria",
  "gate_basis": "priority_thresholds",
  "target": { "type": "story", "id": "5.3", "label": "CTC Validation Reports" },
  "decision_mode": "automated",
  "evaluator": "Putu",
  "confidence": "high",
  "oracle": {
    "resolution_mode": "formal_requirements",
    "confidence": "high",
    "sources": [
      "_bmad-output/implementation-artifacts/5-3-ctc-validation-reports.md",
      "_bmad-output/planning-artifacts/epics.md",
      "_bmad-output/planning-artifacts/prd.md"
    ],
    "external_pointer_status": "not_used",
    "synthetic": false
  },
  "coverage": {
    "inventory": { "covered": 4, "total": 4, "pct": 100 },
    "priority_breakdown": {
      "P0": { "total": 4, "covered": 4, "pct": 100 },
      "P1": { "total": 0, "covered": 0, "pct": 100 },
      "P2": { "total": 0, "covered": 0, "pct": 100 },
      "P3": { "total": 0, "covered": 0, "pct": 100 }
    },
    "by_level": {
      "e2e":       { "tests": 0,  "criteria_covered": 0 },
      "api":       { "tests": 36, "criteria_covered": 4 },
      "component": { "tests": 0,  "criteria_covered": 0 },
      "unit":      { "tests": 9,  "criteria_covered": 3 },
      "other":     { "tests": 0,  "criteria_covered": 0 }
    }
  },
  "tests": {
    "files": 2,
    "cases": 45,
    "skipped_cases": 0,
    "fixme_cases": 0,
    "pending_cases": 0
  },
  "risk_summary": {
    "critical_open": 0,
    "high_open": 1,
    "medium_open": 1,
    "low_open": 0
  },
  "heuristics": {
    "endpoint_gaps": 0,
    "auth_negative_path_status": "present",
    "error_path_status": "present",
    "ui_journey_status": "manual_only",
    "ui_state_status": "manual_only"
  },
  "blockers": [
    "Final 36-test integration suite requires live PostgreSQL execution before release approval"
  ],
  "gate_status": "CONCERNS",
  "gate_criteria": {
    "p0_coverage_required": "100%",
    "p0_coverage_actual": "100%",
    "p0_status": "MET",
    "p1_coverage_target": "90%",
    "p1_coverage_minimum": "80%",
    "p1_coverage_actual": "100%",
    "p1_status": "MET",
    "overall_coverage_minimum": "80%",
    "overall_coverage_actual": "100%",
    "overall_status": "MET"
  },
  "links": {
    "trace_report_path": "_bmad-output/test-artifacts/traceability/5-3-ctc-validation-reports-traceability.md",
    "trace_report_url": "",
    "artifact_url": "",
    "journey_evidence_url": ""
  }
}
```

---

## Final Gate Summary

```
⚠️ GATE DECISION: CONCERNS

📊 Coverage Analysis:
- P0 Coverage: 100% (Required: 100%) → MET
- P1 Coverage: n/a (no P1 items)     → MET
- Overall Coverage: 100% (>=80%)     → MET

⚠️ Decision Rationale:
Coverage is mapped and review decisions are resolved. The final integration
inventory expanded to 36 DB-backed tests after review patches, but this
environment lacked DATABASE_URL for live execution.

⚠️ Critical Gaps: 0

📝 Recommended Actions:
1. Run `cargo test -p xynergy-backend --test ctc_validation_report_tests`
   against live PostgreSQL before release approval
2. Add an E2E smoke for the finance CTC Validation page drilldown a11y when a
   frontend test harness exists
3. Run `/bmad-testarch-test-review` after merge to assess test quality

📂 Full Report: _bmad-output/test-artifacts/traceability/5-3-ctc-validation-reports-traceability.md

⚠️ GATE: CONCERNS — Coverage mapping exists and decisions are resolved, but
   release approval is pending final live-DB execution of the expanded suite.
```
