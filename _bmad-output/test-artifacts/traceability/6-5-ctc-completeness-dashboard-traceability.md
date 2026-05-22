---
stepsCompleted:
  - step-01-load-context
  - step-02-discover-tests
  - step-03-map-criteria
  - step-04-analyze-gaps
  - step-05-gate-decision
lastStep: step-05-gate-decision
lastSaved: '2026-05-22'
lastUpdated: '2026-05-23T00:00:00+07:00'
storyId: '6.5'
storyKey: 6-5-ctc-completeness-dashboard
storyTitle: 'CTC Completeness Dashboard'
workflowType: 'testarch-trace'
coverageBasis: acceptance_criteria
oracleConfidence: high
oracleResolutionMode: formal_requirements
oracleSources:
  - _bmad-output/implementation-artifacts/6-5-ctc-completeness-dashboard.md
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/test-artifacts/automation-summary-6-5.md
externalPointerStatus: not_used
tempCoverageMatrixPath: /tmp/tea-trace-coverage-matrix-6-5-2026-05-22.json
gateDecision: CONDITIONAL_PASS
gateEligible: true
collectionStatus: COLLECTED
executionMode: sequential
manualVerificationStatus: deferred_to_deployed_environment
releaseApproved: false
manualVerificationBlocksRelease: true
inputDocuments:
  - _bmad-output/implementation-artifacts/6-5-ctc-completeness-dashboard.md
  - _bmad-output/implementation-artifacts/sprint-status.yaml
  - _bmad-output/test-artifacts/automation-summary-6-5.md
  - src/backend/src/services/ctc_completeness.rs
  - src/backend/src/routes/ctc.rs
  - src/backend/src/services/dashboard_service.rs
  - src/backend/tests/ctc_validation_tests.rs
  - src/backend/tests/dashboard_tests.rs
  - src/frontend/src/pages/dashboard.rs
  - src/frontend/src/pages/ctc_completeness.rs
---

# Traceability Matrix & Gate Decision — Story 6.5 (CTC Completeness Dashboard)

**Target:** Story 6.5 — CTC Completeness Dashboard
**Date:** 2026-05-22
**Evaluator:** Putu (Master Test Architect)
**Coverage Oracle:** acceptance_criteria
**Oracle confidence:** high
**Oracle resolution mode:** formal_requirements
**Collection status:** COLLECTED
**Gate eligible:** true

---

## 1. Coverage Oracle

Resolved from the formal acceptance criteria documented in
`_bmad-output/implementation-artifacts/6-5-ctc-completeness-dashboard.md`,
cross-referenced with Epic 6 (`_bmad-output/planning-artifacts/epics.md`),
PRD FR46/FR50/FR51/NFR7 (`_bmad-output/planning-artifacts/prd.md`), and the
prior automation expansion summary
(`_bmad-output/test-artifacts/automation-summary-6-5.md`). No external
pointers were required; the in-repo story file is self-contained.

| Oracle item | Source |
|-------------|--------|
| AC #1 — Totals (Total Employees, With CTC, Missing CTC, Completeness %) | Story 6.5 §Acceptance Criteria |
| AC #2 — Department breakdown (name, count, complete count, %) | Story 6.5 §Acceptance Criteria |
| AC #3 — Missing CTC list with Add CTC action | Story 6.5 §Acceptance Criteria |
| AC #4 — Monthly trend toward 100% | Story 6.5 §Acceptance Criteria |

All four ACs are P0 (security/data-correctness invariants for the HR
dashboard); Story 6.5 does not introduce P1/P2/P3 requirements.

---

## 2. Test Inventory (deduplicated, 55 active Story-6.5 mapped tests)

Exact line references are intentionally omitted because review patches moved
test locations after this trace was first generated. Use the test names below
as the stable inventory keys and `rg -n "<test_name>" <file>` for the current
line.

### Backend Integration — `#[sqlx::test(migrations = "../../migrations")]`

`src/backend/tests/ctc_validation_tests.rs` (23 tests covering this story's surfaces):

| ID | Test | Level | ACs |
|----|------|-------|-----|
| INT-01 | `completeness_returns_department_counts` | api | #1, #2 |
| INT-02 | `completeness_denied_for_pm` | api | Task 2 |
| INT-03 | `completeness_denied_for_finance` | api | Task 2 |
| INT-04 | `missing_employees_returns_correct_list` | api | #3 |
| INT-05 | `completeness_exposes_story_6_5_top_level_fields` | api | #1, #4 |
| INT-06 | `missing_endpoint_is_hr_only` | api | #3, Task 2 |
| INT-07 | `missing_employee_payload_uses_resource_id_and_has_no_sensitive_fields` | api | #3, Task 1 |
| INT-08 | `completeness_department_head_is_scoped_to_own_department` | api | Task 2 |
| INT-09 | `completeness_department_head_without_department_returns_403` | api | Task 2 |
| INT-10 | `completeness_department_head_must_match_department_head_record` | api | Task 2 |
| INT-11 | `completeness_empty_state_returns_zero_metrics_without_nan` | api | #1, #4 |
| INT-12 | `completeness_hr_department_filter_scopes_all_surfaces` | api | #1, #2, #4 |
| INT-13 | `completeness_rejects_malformed_department_id` | api | Task 2 |
| INT-14 | `missing_endpoint_honors_hr_department_filter` | api | #3 |
| INT-15 | `completeness_excludes_inactive_ctc_records` | api | #1, #3, #4 |
| INT-16 | `completeness_excludes_non_employee_resources` | api | #1 |
| INT-17 | `completeness_trend_reflects_new_active_ctc` | api | #4 |
| INT-18 | `completeness_trend_returns_exact_multi_month_percentages` | api | #4 |
| INT-19 | `completeness_trend_null_resource_created_at_counts_current_month_only` | api | #4 |
| INT-20 | `completeness_trend_current_month_excludes_future_effective_ctc` | api | #4 |
| INT-21 | `completeness_dashboard_read_does_not_emit_audit_log` | api | Task 2 |
| INT-22 | `completeness_department_rows_sum_matches_top_level_totals` | api | #1, #2 |
| INT-23 | `completeness_unknown_department_id_returns_empty_payload` | api | Task 2 |

`src/backend/tests/dashboard_tests.rs` (5 tests covering HR section):

| ID | Test | Level | ACs |
|----|------|-------|-----|
| INT-24 | `hr_dashboard_returns_hr_section_only` (Story-6.5 trend assertions) | api | #1, #2, #4 |
| INT-25 | `hr_recent_changes_do_not_expose_encrypted_fields` | api | Task 1 |
| INT-26 | `hr_dashboard_section_has_no_sensitive_fields_anywhere` | api | Task 1 |
| INT-27 | `hr_pending_updates_sample_bounded_to_limit` | api | Regression |
| INT-28 | `hr_recent_changes_limited_to_constant` | api | Regression |

### Backend Unit — `src/backend/src/services/ctc_completeness.rs::tests`

| ID | Test | Level | ACs |
|----|------|-------|-----|
| UNIT-01 | `test_percent_with_zero_denominator` | unit | #1 invariant |
| UNIT-02 | `test_percent_with_values` | unit | #1 invariant |
| UNIT-03 | `trend_month_ends_default_returns_12_buckets_ending_with_anchor_month` | unit | #4 |
| UNIT-04 | `trend_month_ends_handles_year_boundary` | unit | #4 |
| UNIT-05 | `trend_month_ends_clamps_to_max_months` | unit | #4 |
| UNIT-06 | `trend_month_ends_clamps_to_minimum_one` | unit | #4 |
| UNIT-07 | `trend_point_clamps_with_ctc_to_total_employees` | unit | #4 defense |
| UNIT-08 | `trend_point_aligns_by_month_end_and_zero_fills_missing_rows` | unit | #4 |

### Frontend Native — `src/frontend/src/pages/ctc_completeness.rs::tests`

| ID | Test | Level | ACs |
|----|------|-------|-----|
| FE-01 | `parse_completeness_uses_new_field_names` | unit | #1, Task 4 |
| FE-02 | `parse_completeness_handles_missing_optional_fields` | unit | #1 robustness |
| FE-03 | `parse_completeness_derives_total_missing_when_absent` | unit | #1 |
| FE-04 | `parse_completeness_derives_department_missing_when_absent` | unit | #2 |
| FE-05 | `parse_completeness_ignores_pre_story_6_5_field_names` | unit | Task 4 |
| FE-06 | `bar_width_pct_clamps_negative_and_oversized` | unit | #4 |
| FE-07 | `color_class_thresholds_match_dashboard` | unit | UX |
| FE-08 | `valid_resource_id_accepts_only_uuid_targets` | unit | #3 |
| FE-09 | `parse_completeness_accepts_stringified_numeric_values` | unit | #1 robustness |
| FE-10 | `parse_completeness_rejects_non_finite_numeric_strings` | unit | #1, #4 robustness |
| FE-11 | `parse_completeness_trend_with_zero_total_has_zero_percent_no_nan` | unit | #4 |

### Frontend Native — `src/frontend/src/pages/dashboard.rs::tests` (Story 6.5 keys)

| ID | Test | Level | ACs |
|----|------|-------|-----|
| FE-12 | `hr_total_missing_key_flashes_on_change` | unit | #1 |
| FE-13 | `add_ctc_href_points_to_existing_ctc_resource_flow` | unit | #3 |
| FE-14 | `hr_department_completion_pct_key_is_stable_on_id` | unit | #2 |
| FE-15 | `hr_trend_period_keys_flash_only_for_changed_months` | unit | #4 |
| FE-16 | `hr_departments_count_key_flashes_when_department_added` | unit | #2 |
| FE-17 | `hr_department_with_no_id_falls_back_to_name_keyed_change_flash` | unit | #2 defense |
| FE-18 | `hr_recent_change_with_no_resource_id_falls_back_to_name_keyed_change_flash` | unit | Recent-change fallback defense |
| FE-19 | `hr_generated_at_only_change_does_not_flash_completeness_keys` | unit | Polling invariant |

**Test inventory totals:** 55 active Story-6.5 mapped tests across 5 files. Zero
skipped, pending, or fixme. Zero blockers.

---

## 3. Traceability Matrix (AC → Tests)

### AC #1 — Total Employees / With CTC / Missing CTC / Completeness %  →  **FULL**

| Layer | Tests |
|-------|-------|
| Backend INT (api) | INT-01, INT-05, INT-09, INT-10, INT-13, INT-14, INT-17 |
| Backend UNIT | UNIT-01, UNIT-02 |
| Frontend (parser) | FE-01, FE-02, FE-03, FE-05, FE-09, FE-10 |
| Frontend (change-detection) | FE-12 |

Coverage notes: top-level field shape (`total_with_ctc`, `total_missing`,
`overall_completion_pct`), happy/empty/inactive/non-employee paths, cross-row
sum invariant, percentage zero-denominator, parser tolerance for stringified
numerics and missing fields.

### AC #2 — Department breakdown (name / count / complete count / %)  →  **FULL**

| Layer | Tests |
|-------|-------|
| Backend INT (api) | INT-01, INT-12, INT-22, INT-24 |
| Frontend (parser + change-detection) | FE-04, FE-14, FE-16, FE-17 |

Coverage notes: rows present with required columns, HR department filter
scopes all surfaces consistently, sum-of-department-rows = top-level totals,
change-flash on per-department `completion_pct` (id-keyed), departments.count
flash on add/remove, name-keyed fallback when `department_id` is `None`.

### AC #3 — Missing CTC count → list of employees with `Add CTC`  →  **FULL (automated API/helper) / MANUAL_DEFERRED (click + deep-link navigation)**

| Layer | Tests |
|-------|-------|
| Backend INT (api) | INT-04, INT-06, INT-07, INT-14, INT-15 |
| Frontend (helper) | FE-08, FE-13 |

Coverage notes: happy list, HR-only (PM/Finance/DH denied), `Add CTC` target
== resource_id, no sensitive fields, HR department filter respected,
Inactive-CTC employee correctly surfaces in missing list. The live click path
from the Missing CTC card to `/ctc?resource_id=<uuid>` remains manual-deferred
until the HR browser walk-through is recorded.

### AC #4 — Monthly completeness trend  →  **FULL**

| Layer | Tests |
|-------|-------|
| Backend INT (api) | INT-05, INT-11, INT-12, INT-15, INT-17, INT-18, INT-19, INT-20, INT-23, INT-24 |
| Backend UNIT | UNIT-03, UNIT-04, UNIT-05, UNIT-06, UNIT-07, UNIT-08 |
| Frontend (parser + change-detection) | FE-06, FE-10, FE-11, FE-15 |

Coverage notes: trend present + ascending months + bounded ≤24, default
12 buckets, year-boundary alignment, clamp to min/max months,
`with_ctc <= total_employees` clamp, zero-row alignment + zero-fill,
filter scope on latest bucket, Inactive CTC reflected in trend, new active
CTC signal, unknown department UUID still emits bounded zero-trend, parser
zero-bucket no-NaN, bar width clamps for negatives/overflow, per-month
change-flash keys.

### Cross-cutting (Tasks 1/2/3/4/7) → tests

| Task / Concern | Coverage | Mapped Tests |
|-----------------|----------|--------------|
| Task 1 — Sensitive-field hygiene across whole HR section | FULL | INT-07, INT-22, INT-23, INT-26 |
| Task 2 — Access control (HR/DH allow, PM/Finance deny, DH scoping, malformed UUID 400, unknown UUID 200, no audit-log noise) | FULL | INT-02, INT-03, INT-06, INT-08, INT-09, INT-10, INT-13, INT-21, INT-23 |
| Task 3 / Task 7 — Polling and change-detection keys (no generated_at-only flashes) | FULL | FE-12, FE-14, FE-15, FE-16, FE-17, FE-18, FE-19 |
| Task 4 — `/ctc/completeness` page upgrade (Story-6.5 parser, expandable breakdown, bars, no charting dep) | FULL (helper layer) | FE-01..FE-11 |
| Task 6 — Backend regression coverage commands all green | FULL | INT-01..INT-28 |

---

## 4. Coverage Heuristics

| Heuristic | Status | Notes |
|-----------|--------|-------|
| API endpoint coverage | ✅ Present | `/api/v1/ctc/completeness`, `/api/v1/ctc/completeness/missing`, `/api/v1/dashboard` HR section — all covered positive + negative |
| Auth/authz coverage | ✅ Present | PM-403, Finance-403, DH scoping, missing HR-only — all negative paths covered |
| Error-path coverage | ✅ Present | 400 (malformed UUID), empty state (no NaN), Inactive CTC excluded, non-employee excluded, unknown UUID empty 200 |
| UI journey coverage (parser/helper layer) | ✅ Present | Parser, change-detection, sensitive-field walks all covered |
| UI journey coverage (live browser) | ⚠️ Deferred — blocks release sign-off, does not block automated gate | Story-acknowledged: live HR walk-through on deployed environment is the only remaining manual step. No Playwright/Cypress harness in repo. Helper-layer assertions cover the same surfaces except click-level navigation. |
| UI state coverage | ✅ Present | Empty/zero-total/string-numeric/no-NaN/loading-via-parser paths all asserted |

---

## 5. Coverage Statistics

| Metric | Value |
|--------|-------|
| Total acceptance criteria | 4 |
| Fully covered | 4 |
| Partially covered | 0 |
| Uncovered | 0 |
| **Overall coverage** | **100%** |

### Layer breakdown

| Layer | Status | Blocks release? |
|-------|--------|-----------------|
| Automated AC coverage | 100% FULL | n/a |
| Manual UI verification | DEFERRED | Yes |

### Priority breakdown

| Priority | Total | Covered | % |
|----------|-------|---------|---|
| P0 | 4 | 4 | **100%** |
| P1 | 0 | 0 | 100% (no P1 reqs) |
| P2 | 0 | 0 | 100% |
| P3 | 0 | 0 | 100% |

### By level

| Level | Tests | Criteria Covered |
|-------|-------|------------------|
| e2e | 0 | 0 |
| api | 28 | 4 |
| component | 0 | 0 |
| unit | 27 | 4 |
| other | 0 | 0 |

---

## 6. Gap Analysis

- **Critical (P0):** 0
- **High (P1):** 0
- **Medium (P2):** 0
- **Low (P3):** 1 manual UI verification item
- **Partial coverage:** 0 automated; AC #3 has manual-deferred click/deep-link evidence
- **Unit-only:** 0

### Remaining advisory item

- **Live HR browser walk-through (Task 7 final sub-bullet).** Acknowledged
  by the story as deferred to a deployed environment because no Playwright/
  Cypress harness exists in this repo. The automated gate is complete, but
  release sign-off remains conditional until the deployed-environment
  click/deep-link and layout checks are recorded.

---

## 7. Recommendations

1. **MEDIUM (manual verification gate)** — Run the live HR browser walk-through against a deployed
   environment to close the last unchecked sub-bullet of Task 7. Verify:
   `/dashboard` HR completeness summary polls and highlights changes;
   `/ctc/completeness` shows correct headline metrics and expandable
   department breakdown; missing-CTC click expands a bounded employee list
   and `Add CTC` routes to `/ctc?resource_id=<uuid>`; trend renders cleanly
   for empty, partial, and 100% completeness states without layout shift.
2. **LOW** — When broader UI E2E is added across Epic 6+ (Playwright or
   similar), promote the deferred walk-through into a smoke spec to
   convert the remaining advisory heuristic gap into formal E2E coverage.
3. **LOW** — Optional follow-up: run `/bmad:tea:test-review` on the
   Story-6.5 additions if any test-quality polish is desired (assertion
   density, fixture reuse, naming consistency).

---

## 8. Gate Decision

### 🟡 GATE: CONDITIONAL_PASS

**Rationale:** Automated P0 coverage is 100% (4/4 acceptance criteria), no P1
requirements present (effective P1 = 100%), overall coverage is 100%, the
formal acceptance-criteria oracle has high confidence, and the collection
status is COLLECTED. 55 active Story-6.5 mapped tests cover all four ACs and the cross-cutting
Task 1/2/3/7 invariants (sensitive-field hygiene, RBAC, audit-log
hygiene, polling change-detection). No automated blockers, skipped, or pending
tests. Release sign-off is conditional on the deferred live HR browser walk-through.

### Gate criteria

| Criterion | Required | Actual | Status |
|-----------|----------|--------|--------|
| P0 coverage | 100% | 100% | ✅ MET |
| P1 coverage (target/min) | 90% / 80% | 100% (no P1) | ✅ MET |
| Overall coverage | ≥80% | 100% | ✅ MET |

### Critical automated gaps: **0**

### Manual verification status

`deferred_to_deployed_environment` — the live HR walk-through called out
as the final sub-bullet of Task 7 is the only remaining manual step. It
does not block the automated gate, but it does block release sign-off.
Promote to PASS only after the deployed-environment walk-through is recorded.

### Companion artifacts

- Machine-readable summary: `_bmad-output/test-artifacts/traceability/6-5-e2e-trace-summary.json`
- Slim gate decision: `_bmad-output/test-artifacts/traceability/6-5-gate-decision.json`
- Coverage matrix (temp, full): `/tmp/tea-trace-coverage-matrix-6-5-2026-05-22.json`

---

🟡 **GATE: CONDITIONAL_PASS** — Automated coverage meets standards. Release sign-off pending live HR browser walk-through on deployed environment.
