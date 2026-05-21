---
stepsCompleted:
  - step-01-load-context
  - step-02-discover-tests
  - step-03-map-criteria
  - step-04-analyze-gaps
  - step-05-gate-decision
lastStep: step-05-gate-decision
lastSaved: '2026-05-21'
storyId: '6.1'
storyKey: 6-1-role-based-dashboard
storyTitle: 'Role-Based Dashboard'
coverageBasis: acceptance_criteria
oracleConfidence: high
oracleResolutionMode: formal_requirements
oracleSources:
  - _bmad-output/implementation-artifacts/6-1-role-based-dashboard.md
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/prd.md
externalPointerStatus: not_used
tempCoverageMatrixPath: /tmp/tea-trace-coverage-matrix-6-1-2026-05-21.json
gateDecision: PASS_WITH_ADVISORY
gateEligible: true
collectionStatus: COLLECTED
---

# Traceability Report — Story 6.1 (Role-Based Dashboard)

## Gate Decision: **PASS_WITH_ADVISORY**

**Rationale:** All four P0 acceptance criteria are FULL-covered by 18 backend
integration tests in `src/backend/tests/dashboard_tests.rs` (10 from the initial
implementation pass + 8 from the automation expansion run). The backend suite
ran 10/10 green during implementation, and high-risk regression suites
(`team_tests`, `project_pl_tests`, `cash_flow_dashboard_tests`,
`ctc_validation_report_tests`, `compliance_audit_report_tests` — 132/132)
re-ran green. Data isolation, sensitive-field guards, list bounds, empty-state,
and security-boundary cases are all explicitly tested.

**Advisory caveats (non-blocking):**

1. **Frontend `/dashboard` has no committed automated tests.** The Leptos page
   `src/frontend/src/pages/dashboard.rs` was verified by `cargo check
   --target wasm32-unknown-unknown` and `npm run build` only. The repo has no
   Leptos component test harness and no Playwright config, so role-specific
   panel rendering, the manual refresh button, the "Last updated" label, and
   loading/empty/error states have no automated coverage.
2. **30-second polling is explicitly deferred to Story 6.2.** Story 6.1 ships
   manual refresh only. Dev Notes warn against `Interval::forget()` — that
   contract must be enforced when 6.2 adds polling.
3. **`login.rs::role_dashboard_path()` change has no automated frontend test.**
   The redirect-after-login behaviour change to send every supported role to
   `/dashboard` is structurally simple but unverified by automated checks;
   manual smoke required at release.

None of these are release-blocking: the backend is the authority for role scope
per Story 6.1 architecture compliance, and all backend role-scoping invariants
are tested.

---

## Coverage Oracle

- **Resolution mode:** `formal_requirements` (story acceptance criteria)
- **Confidence:** `high`
- **Sources:**
  - `_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md` (4 ACs, 8 task groups)
  - `_bmad-output/planning-artifacts/epics.md` (Epic 6 / Story 6.1)
  - `_bmad-output/planning-artifacts/prd.md` (FR45 personalized dashboards; FR50/FR51 polling + manual refresh lay-down)
- **External pointer status:** `not_used`
- **Synthetic oracle:** no

---

## Test Inventory

| Category | File | Count | Notes |
|----------|------|-------|-------|
| API / integration — story-scoped | `src/backend/tests/dashboard_tests.rs` | 18 | `#[sqlx::test(migrations = "../../migrations")]`; executed 10/10 green during implementation and 8 added by the automation expansion run |
| Unit | `src/backend/src/services/dashboard_service.rs` (`mod tests`) | 0 | No inline unit tests; service logic is exercised end-to-end via API tests |
| Component / E2E — frontend | `src/frontend/src/pages/dashboard.rs` | 0 | No committed automated tests; `cargo check --target wasm32-unknown-unknown` and `npm --prefix src/frontend run build` confirm compile + Tailwind regeneration |
| Regression — team | `src/backend/tests/team_tests.rs` | 10 | Re-run green during implementation |
| Regression — project P&L | `src/backend/tests/project_pl_tests.rs` | 10 | Re-run green during implementation |
| Regression — cash flow dashboard | `src/backend/tests/cash_flow_dashboard_tests.rs` | 28 | Re-run green during implementation |
| Regression — CTC validation | `src/backend/tests/ctc_validation_report_tests.rs` | 36 | Re-run green during implementation |
| Regression — compliance audit | `src/backend/tests/compliance_audit_report_tests.rs` | 48 | Re-run green during implementation |
| **Total story-scoped** |  | **18** |  |
| **Total story + regression** |  | **150** |  |

**By level:** API 18 story-scoped (+ 132 regression); Unit 0; Component 0; E2E 0
**Skipped / pending / fixme:** 0

---

## Traceability Matrix (AC → Tests)

### AC #1 — HR sees CTC Completeness Status, Recent CTC Changes, Pending Updates, Compliance Alerts

- **Priority:** P0 (primary HR workflow; sensitive CTC data scope; HR-only missing-CTC list)
- **Coverage:** FULL (backend); manual only (frontend)
- **Mapped tests:**

| Test ID | Title | File | Level | Status |
|---------|-------|------|-------|--------|
| 6.1-API-001 | `hr_dashboard_returns_hr_section_only` | `src/backend/tests/dashboard_tests.rs:256` | api | active |
| 6.1-API-002 | `hr_recent_changes_do_not_expose_encrypted_fields` | `src/backend/tests/dashboard_tests.rs:289` | api | active |
| 6.1-API-014 | `hr_pending_updates_sample_bounded_to_limit` | `src/backend/tests/dashboard_tests.rs:690` | api | active |

**Heuristics:** endpoint covered • auth gate verified (HR-only completeness/missing list) • sensitive-field guard explicit • list bound enforced at `MISSING_CTC_SAMPLE_LIMIT=5`.

### AC #2 — Department Head sees Team Utilization, Budget Status, Overallocations, Upcoming Assignments

- **Priority:** P0 (department-scoped data isolation; RLS / `head_id` correctness)
- **Coverage:** FULL (backend); manual only (frontend)
- **Mapped tests:**

| Test ID | Title | File | Level | Status |
|---------|-------|------|-------|--------|
| 6.1-API-003 | `dept_head_only_sees_own_department` | `src/backend/tests/dashboard_tests.rs:328` | api | active |
| 6.1-API-004 | `dept_head_response_has_utilization_and_overallocation` | `src/backend/tests/dashboard_tests.rs:382` | api | active |
| 6.1-API-011 | `dept_head_without_department_returns_403` | `src/backend/tests/dashboard_tests.rs:599` | api | active |
| 6.1-API-015 | `dept_head_upcoming_excludes_past_allocations` | `src/backend/tests/dashboard_tests.rs:724` | api | active |

**Heuristics:** cross-department leak guard explicit • orphan-DH security gate explicit • date-window correctness verified (past allocations excluded from "upcoming").

### AC #3 — Project Manager sees Project Health Cards, Active Projects, Margin Alerts

- **Priority:** P0 (relationship-based access via `projects.project_manager_id`; non-owned projects must not surface)
- **Coverage:** FULL (backend); manual only (frontend)
- **Mapped tests:**

| Test ID | Title | File | Level | Status |
|---------|-------|------|-------|--------|
| 6.1-API-005 | `project_manager_only_sees_owned_projects` | `src/backend/tests/dashboard_tests.rs:412` | api | active |
| 6.1-API-012 | `project_manager_with_no_projects_returns_empty_state` | `src/backend/tests/dashboard_tests.rs:620` | api | active |
| 6.1-API-013 | `project_manager_excludes_non_active_projects` | `src/backend/tests/dashboard_tests.rs:652` | api | active |

**Heuristics:** ownership filter explicit • empty-state verified • status filter (`Active` only) verified against `Completed/Closed/Cancelled`. Margin alerts surface via reuse of `ProjectPlDashboardResult.margin_alert` (no duplicated P&L formulas).

### AC #4 — Finance sees Cash Position, CTC Validation Status, Audit Alerts, Export Requests Pending

- **Priority:** P0 (finance/admin role gate; audit alert signal correctness; no-data state safety)
- **Coverage:** FULL (backend); manual only (frontend)
- **Mapped tests:**

| Test ID | Title | File | Level | Status |
|---------|-------|------|-------|--------|
| 6.1-API-006 | `finance_dashboard_has_cash_and_audit_state` | `src/backend/tests/dashboard_tests.rs:457` | api | active |
| 6.1-API-007 | `finance_validation_returns_no_data_when_no_payroll` | `src/backend/tests/dashboard_tests.rs:502` | api | active |
| 6.1-API-017 | `finance_audit_alerts_count_recent_security_events` | `src/backend/tests/dashboard_tests.rs:807` | api | active |

**Heuristics:** cash totals verified • `no_data` validation status verified (does not fail the whole dashboard) • audit alert allow-list enforced (`ACCESS_DENIED`, `LOGIN_FAILED`, `LOGIN_BLOCKED`, `CHAIN_VERIFICATION_FAILED`, `VERIFY_CHAIN_FAILED`) • finance role does not receive HR/DH/PM sections.

---

## Supporting Tests (cross-cutting, not bound to a single AC)

| Test ID | Title | Purpose | File |
|---------|-------|---------|------|
| 6.1-SEC-001 | `unauthenticated_dashboard_returns_401` | Auth gate on `/api/v1/dashboard` | `dashboard_tests.rs:245` |
| 6.1-SEC-002 | `invalid_bearer_token_returns_401` | Malformed JWT must be rejected, not silently anonymous | `dashboard_tests.rs:863` |
| 6.1-SEC-003 | `unsupported_role_returns_403` | `team_member` (and other unsupported roles) get 403 | `dashboard_tests.rs:548` |
| 6.1-CON-001 | `dashboard_response_includes_recent_generated_at` | Contract: `generated_at` present, RFC3339, in request window | `dashboard_tests.rs:772` |
| 6.1-ADM-001 | `admin_dashboard_returns_admin_section` | Admin gets operational-totals view, never blank dashboard | `dashboard_tests.rs:522` |

---

## Coverage Statistics

- Total requirements: 4
- Fully covered: 4 (100%)
- Partially covered: 0
- Uncovered: 0

**Priority breakdown:**

| Priority | Total | Covered | % |
|----------|-------|---------|---|
| P0 | 4 | 4 | 100% |
| P1 | 0 | 0 | n/a |
| P2 | 0 | 0 | n/a |
| P3 | 0 | 0 | n/a |

---

## Gaps & Recommendations

**No P0/P1 coverage gaps.** All blocking acceptance criteria have backend
integration coverage including isolation, bounds, empty-state, and security
edges.

**Advisory follow-ups:**

| Priority | Action | Scope |
|----------|--------|-------|
| MEDIUM | Add E2E or Leptos component coverage for `/dashboard` once a frontend test harness lands | UI-frontend-dashboard |
| MEDIUM | Add explicit loading / empty / error / permission-denied state assertions for `/dashboard` rendering | UI-state-empty-error |
| MEDIUM | Story 6.2 polling implementation must use leak-safe `Interval` cleanup (no `Interval::forget()`) | Story 6.2 carry-over |
| LOW | Run `/bmad-testarch-test-review` on `dashboard_tests.rs` to confirm test-quality DoD | dashboard_tests.rs |

---

## Coverage Heuristics Summary

- **Endpoints covered:** 1 / 1 — `GET /api/v1/dashboard` ✓
- **Auth negative paths:** present (missing token → 401; malformed token → 401; unsupported role → 403; DH without department → 403)
- **Error / no-data paths:** present (CTC validation `no_data`, empty PM project list, missing-CTC bounded sample)
- **Sensitive-field guard:** present (HR recent changes blocklist on `encrypted_components`, `encrypted_daily_rate`, `ciphertext`, `key_version`, `encryption_algorithm`, `encryption_version`, `base_salary`, `daily_rate`)
- **Data isolation:** present (DH cross-department exclusion; PM non-owned-project exclusion; finance does not see HR/DH/PM)
- **List bounds:** present (`MISSING_CTC_SAMPLE_LIMIT=5`; bounded recent changes / upcoming assignments / active projects)
- **UI journey E2E:** missing (compile-only)
- **UI states (loading/empty/error/permission-denied):** missing (compile-only)
- **Polling cleanup contract:** deferred to Story 6.2

---

## Verification Evidence (from Dev Agent Record)

- `cargo test --package xynergy-backend --test dashboard_tests` → **10/10 passed** (initial implementation pass)
- Automation expansion added 8 more tests (`6.1-API-011..018`); total now 18 cases in one file
- Regression suites re-run during implementation:
  - `team_tests` 10/10
  - `project_pl_tests` 10/10
  - `cash_flow_dashboard_tests` 28/28
  - `ctc_validation_report_tests` 36/36
  - `compliance_audit_report_tests` 48/48
- `cargo check -p xynergy-frontend --target wasm32-unknown-unknown` → green (after pre-computing `no_widgets` to avoid borrow-of-moved-value)
- `npm --prefix src/frontend run build` → Tailwind v4.1.18 regenerated `public/output.css` in 44ms

---

## Gate Decision Summary

```
🚨 GATE DECISION: PASS_WITH_ADVISORY

📊 Coverage Analysis:
- P0 Coverage: 100% (Required: 100%) → MET
- P1 Coverage: n/a (no P1 items)     → N/A
- Overall Coverage: 100% (≥ 80%)     → MET

✅ Decision Rationale:
4/4 ACs FULL covered by 18 backend integration tests. Data isolation,
sensitive-field guards, list bounds, empty-state, and security-boundary cases
explicitly tested. Backend dashboard suite 10/10 green; high-risk regression
suites (132 tests) re-run green. Frontend has only compile-level verification —
advisory, not blocking, because backend is the authority for role scope.

⚠️ Advisory gaps: 3
  1. Frontend /dashboard has no automated E2E/component coverage
  2. 30-second polling deferred to Story 6.2 (by design)
  3. login.rs role_dashboard_path() change has no automated frontend test

📂 Full Report: _bmad-output/test-artifacts/traceability/6-1-role-based-dashboard-traceability.md
📂 Machine-readable: _bmad-output/test-artifacts/traceability/6-1-e2e-trace-summary.json
📂 Gate signal:      _bmad-output/test-artifacts/traceability/6-1-gate-decision.json
```
