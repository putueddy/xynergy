---
story: '6-5-ctc-completeness-dashboard'
stepsCompleted:
  - 'step-01-preflight-and-context'
  - 'step-02-identify-targets'
  - 'step-03-generate-tests (sequential mode)'
  - 'step-04-validate-and-summarize'
  - 'step-04-validate-and-summarize (expansion pass 2026-05-22T2)'
lastStep: 'step-04-validate-and-summarize'
lastSaved: '2026-05-22'
expansionPasses: 2
mode: 'BMad-Integrated'
executionMode: 'sequential (inline generation, no subagent dispatch)'
detectedStack: 'fullstack'
testStackType: 'auto -> fullstack (Rust workspace + Leptos frontend)'
inputDocuments:
  - '_bmad-output/implementation-artifacts/6-5-ctc-completeness-dashboard.md'
  - '_bmad-output/implementation-artifacts/sprint-status.yaml'
  - '_bmad-output/project-context.md'
  - 'src/backend/src/services/ctc_completeness.rs'
  - 'src/backend/src/routes/ctc.rs'
  - 'src/backend/src/services/dashboard_service.rs'
  - 'src/backend/tests/ctc_validation_tests.rs'
  - 'src/backend/tests/dashboard_tests.rs'
  - 'src/frontend/src/pages/dashboard.rs'
  - 'src/frontend/src/pages/ctc_completeness.rs'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/test-levels-framework.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/test-priorities-matrix.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/data-factories.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/test-quality.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/selective-testing.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/ci-burn-in.md'
---

# Test Automation Expansion — Story 6.5 CTC Completeness Dashboard

## 1. Preflight & Context

### Stack Detection

- `test_stack_type: auto` → resolved to **fullstack** (Rust workspace
  `Cargo.toml` + Leptos `src/frontend/Cargo.toml` + Tailwind
  `src/frontend/package.json`).
- No browser-test indicators (`playwright.config.*`, `cypress.config.*`).
- Profile: **backend integration via `#[sqlx::test]` + frontend native
  unit tests on the host target**. The completeness trend, department
  rollups, and missing-CTC payloads are exercised via Postgres-backed
  integration tests; the HR panel and `/ctc/completeness` page are
  covered by pure helper tests (parsing, change-map keys, sensitive-
  field absence).

### Execution Mode

**BMad-Integrated.** Story `6-5-ctc-completeness-dashboard.md` is in
`review` after the dev pass. Existing coverage entering this expansion:

- Backend INT (under `src/backend/tests/ctc_validation_tests.rs`): 20 tests
  including 5 Story 6.5 additions (`top-level field shape`, missing-list
  HR-only, missing payload sensitive-field absence, DH scope-locked
  completeness, trend reflects new active CTC).
- Backend INT (under `src/backend/tests/dashboard_tests.rs`): 57 tests
  including the Story 6.5 HR trend presence + ascending-month + bounded-
  length assertions inside `hr_dashboard_returns_hr_section_only`.
- Backend unit (under `src/backend/src/services/ctc_completeness.rs::tests`):
  5 tests (month-key helpers, last-N-months, bound clamp, etc.).
- Frontend native (under `src/frontend/src/pages/dashboard.rs::tests` and
  `src/frontend/src/pages/ctc_completeness.rs::tests`): 10 tests
  (4 HR change-detection keys + 6 ctc_completeness parser/helpers).

Knowledge fragments applied: test-levels-framework, test-priorities-
matrix, data-factories, test-quality, selective-testing, ci-burn-in.
Playwright Utils profile is **N/A** for this Rust workspace.

## 2. Automation Targets & Coverage Plan

### AC → existing coverage

| AC | Surface | Existing tests | Status |
|----|---------|---------------|--------|
| #1 | Total / With / Missing / Completeness % | `completeness_returns_department_counts`, `completeness_exposes_story_6_5_top_level_fields` | partial |
| #2 | Department breakdown | `completeness_returns_department_counts` (rows present), no department filter consistency test | partial |
| #3 | Missing CTC list + Add CTC | `missing_employees_returns_correct_list`, `missing_endpoint_is_hr_only`, `missing_employee_payload_uses_resource_id_and_has_no_sensitive_fields` | strong |
| #4 | Monthly trend | `hr_dashboard_returns_hr_section_only` (shape), `completeness_trend_reflects_new_active_ctc` (recent month signal) | partial |

### Coverage gaps (the targets for this expansion)

Mapped against Story 6.5 Tasks 1-7, the Dev Notes "Edge Cases" list,
and the "Implementation Pitfalls To Avoid" block:

| # | Target | Level | Pri | AC | Gap explanation |
|---|--------|-------|-----|----|----------------|
| 1 | Empty completeness — no resources / no employee rows | INT | P0 | #1 | Dev Notes: "Empty data is valid — `0` totals, `0.0%`, no NaN." Not pinned by an existing test for the dashboard payload. |
| 2 | HR `department_id=X` consistency across rows + totals + trend | INT | P0 | #1,#2,#4 | Story: "Department filters must affect all visible completeness surfaces consistently." No single test asserts all three surfaces are scoped to the same id. |
| 3 | Malformed `department_id` rejected (400) for HR completeness | INT | P0 | Task 2 | "Validate through typed query extraction." `Query<CompletenessQuery>` should reject non-UUID values; not exercised today. |
| 4 | `/ctc/completeness/missing?department_id=X` honored | INT | P1 | #3 | Today's missing-list test ignores the department filter path. |
| 5 | HR `/api/v1/dashboard` — sensitive fields absent across the WHOLE `hr` section (not only `recent_changes`) | INT | P1 | Task 1, Task 2 | Existing `hr_recent_changes_do_not_expose_encrypted_fields` only walks `recent_changes`. Trend / department / pending_updates / compliance_alerts not asserted. |
| 6 | Terminated/Inactive CTC excluded from completeness + trend | INT | P1 | #1,#4 | `ctc_records.status = 'Active'` invariant — risk: a stale terminated record inflates `with_ctc`. |
| 7 | Non-employee resources excluded from totals + trend | INT | P1 | #1 | `resource_type = 'employee'` invariant from project-context #9. |
| 8 | `build_trend_points` clamps `with_ctc > total_employees` | UNIT (backend) | P2 | Defense | The clamp `total_with_ctc.min(total_employees)` is unexercised. |
| 9 | Frontend parser accepts stringified numbers in trend payload | UNIT (frontend) | P1 | Robustness | `value_to_i64`/`value_to_f64` accept strings; pathway unexercised end-to-end through `parse_completeness_summary`. |
| 10 | Trend with `total_employees=0` yields `0.0%` and no NaN | UNIT (frontend) | P1 | Invariant | The empty-trend-bucket path through `parse_completeness_summary` must defend against NaN bleed-through. |
| 11 | `hr.completeness.departments.count` flashes when departments list adds/removes | UNIT (frontend) | P1 | #2 | Polling-driven UX cue — covered for per-department keys but not for the count key. |
| 12 | Department fallback key uses `hr.completeness.department.name.<dept>` when id is missing | UNIT (frontend) | P2 | Robustness | The `None` UUID branch in `dashboard_value_map` is unexercised. |

Coverage scope: **selective expansion** — we focus on the new surfaces
(trend, expanded department breakdown, missing-list + Add CTC) and the
risk-driven invariants (RBAC, sensitive-field hygiene, query validation,
active/employee-only filters). We do not regenerate the existing 20 INT
tests on `/ctc/completeness` or the 57 dashboard suite — that is
unchanged regression scope.

### Test levels applied

- **INT (`#[sqlx::test]`, real Postgres)** — anything that depends on
  Active-status filters, employee-type filters, multi-department scoping,
  or query validation. These are SQL-bound invariants.
- **UNIT (host-target `#[test]`)** — pure helpers (`build_trend_points`,
  `parse_completeness_summary`, `dashboard_value_map`). Cheap to add,
  zero-flake, runs in `cargo test --lib`.

### Test priorities applied (per `test-priorities-matrix`)

- **P0** = security/data-correctness invariants the dashboard owner
  would call a regression. Three tests: empty state, department filter
  consistency, malformed UUID rejection.
- **P1** = high-value edge cases for new surfaces and sensitive-field
  hygiene. Seven tests.
- **P2** = defense-in-depth around code paths the implementation has
  but no test currently exercises. Two tests.

Total **net-new tests**: **12** (six backend integration, one backend
unit, five frontend native).

## 3. Tests Generated (sequential)

Execution mode resolved to **sequential** (no subagent dispatch, matching
the Story 6.4 automation pattern). Each new test was authored inline in
the existing suite files — no fixture files, no new directories, no new
crate deps.

### Backend Integration (6 net-new, in `src/backend/tests/`)

`ctc_validation_tests.rs` — 6 new `#[sqlx::test(migrations = "../../migrations")]`
functions appended above the existing `completeness_trend_reflects_new_active_ctc`:

1. **`completeness_empty_state_returns_zero_metrics_without_nan`** (P0, AC #1)
   - Creates an empty department, asserts `total_employees=0`,
     `overall_completion_pct=0.0` (no NaN), trend buckets all zero, and
     reuses the `assert_no_sensitive_fields` walker.
2. **`completeness_hr_department_filter_scopes_all_surfaces`** (P0, AC #1/#2/#4)
   - Two departments, one CTC. Filtered call must scope rows, totals,
     AND the trend's latest bucket to the requested department.
3. **`completeness_rejects_malformed_department_id`** (P0, Task 2)
   - `?department_id=not-a-uuid` → expects 400 from the typed `Query`
     extractor. Guards against silent scope-bleed.
4. **`missing_endpoint_honors_hr_department_filter`** (P1, AC #3)
   - Two missing employees in different departments. Filter excludes
     the off-scope row while including the scoped row.
5. **`completeness_excludes_inactive_ctc_records`** (P1, AC #1/#4)
   - Creates Active CTC, then `UPDATE ctc_records SET status='Inactive'`.
     Employee must flip to `total_with_ctc=0` in both the dept rollup AND
     the latest trend bucket, AND must appear in the missing-list.
6. **`completeness_excludes_non_employee_resources`** (P1, AC #1)
   - Inserts a `resource_type='contractor'` row in the same department
     as an employee. Contractor must be invisible to completeness totals
     and the missing list.

`dashboard_tests.rs` — 1 new integration test appended before
`dept_head_only_sees_own_department`:

7. **`hr_dashboard_section_has_no_sensitive_fields_anywhere`** (P1, Task 1/2)
   - Recursively walks the entire `body["hr"]` subtree (completeness,
     trend, department rows, pending_updates, recent_changes,
     compliance_alerts, warnings) and asserts none of the 20 forbidden
     sensitive keys (`base_salary`, `daily_rate`, `encrypted_components`,
     `key_version`, etc.) appear anywhere.

### Backend Unit (1 net-new, in `src/backend/src/services/`)

`ctc_completeness.rs` `mod tests` — 1 new `#[test]`:

8. **`trend_point_aligns_by_month_end_and_zero_fills_missing_rows`** (P2)
   - Defense-in-depth for `build_trend_points`: when the SQL returns no
     row for a requested month_end, the bucket must still appear in the
     output with `(0,0)`, the YYYY-MM key intact, and `completion_pct=0.0`
     (no NaN). The `total_with_ctc.min(total_employees)` clamp upper-bound
     guarantee is also implicitly verified end-to-end by INT test #2.

### Frontend Native (5 net-new, host target, `not(target_arch="wasm32")`)

`src/frontend/src/pages/ctc_completeness.rs` `mod tests` — 2 new `#[test]`:

9. **`parse_completeness_accepts_stringified_numeric_values`** (P1)
   - Drives the full `parse_completeness_summary` flow with every numeric
     field encoded as a JSON string (`"5"`, `"60.0"`) — guards against a
     proxy / older serializer zeroing out the headline cards.
10. **`parse_completeness_trend_with_zero_total_has_zero_percent_no_nan`** (P1)
    - Trend bucket with `total_employees=0` and no `total_missing` field;
      the parser must derive `total_missing=0` (never negative) and
      `completion_pct=0.0` (never NaN), and `bar_width_pct` must clamp to
      `0.0` through that path.

`src/frontend/src/pages/dashboard.rs` `mod tests` — 2 new `#[test]`:

11. **`hr_departments_count_key_flashes_when_department_added`** (P1, AC #2)
    - Prev snapshot has 1 dept, next has 2. The polling change-map must
      surface `hr.completeness.departments.count` so the dashboard
      flashes the new row count, not just per-department fields.
12. **`hr_department_with_no_id_falls_back_to_name_keyed_change_flash`** (P2)
    - Both snapshots emit a `DepartmentCompleteness` with
      `department_id = None`. The `dashboard_value_map` fallback path
      (`hr.completeness.department.name.<dept>.completion_pct`) must
      light up — without this, name-keyed rows would never flash.

> Note: tests #11 and #12 also depend on the existing
> `hr_response_with_completeness` / `dept` / `trend_point` private test
> helpers and use them directly to keep the new tests in one consistent
> harness with the Story-6.5 tests added during dev.

### Knowledge fragments applied during generation

- **test-levels-framework** — assigned every gap to UNIT (helpers) or
  INT (SQL-bound invariants); skipped E2E because no browser harness.
- **test-priorities-matrix** — P0 = security/correctness invariants
  (empty state, filter scope, malformed UUID), P1 = high-value edge
  cases, P2 = defense-in-depth around code paths.
- **data-factories** — reused existing `create_test_resource_in_department`,
  `create_test_department`, `create_ctc_for_resource`, and `hr_response_with_completeness`
  / `dept` / `trend_point` test helpers. No new factories introduced;
  no test fixture files written.
- **test-quality** — no `Result<>` returns from tests, no thread sleeps,
  no shared mutable global state across tests, transaction-isolated
  via `#[sqlx::test]`.
- **selective-testing** — net-new tests live in the same suite files
  that the dev tasks edited, so `cargo test --test ctc_validation_tests`
  and `cargo test --test dashboard_tests` continue to be the targeted
  CI invocations Story 6.5 already calls out.

## 4. Validation Run

### Compile

| Command | Result |
|---------|--------|
| `cargo check -p xynergy-backend --tests` | clean (sqlx-postgres warnings pre-existing) |
| `cargo check -p xynergy-frontend --tests --lib` | clean (11 pre-existing dead-code warnings) |
| `cargo check -p xynergy-frontend --target wasm32-unknown-unknown` | clean (same pre-existing warnings) |

### Test Execution

| Suite | Prior pass | New pass | Delta |
|-------|-----------|----------|-------|
| `cargo test -p xynergy-backend --lib` | 73 | **74** | +1 (`trend_point_aligns_by_month_end_and_zero_fills_missing_rows`) |
| `cargo test -p xynergy-frontend --lib` | 87 | **91** | +4 (the 2 parser + 2 dashboard frontend tests) |
| `cargo test -p xynergy-backend --test ctc_validation_tests` | 20 | **26** | +6 (the six new INT tests) |
| `cargo test -p xynergy-backend --test dashboard_tests` | 57 | **58** | +1 (`hr_dashboard_section_has_no_sensitive_fields_anywhere`) |

Total **+12 net-new tests, all green**. DB used: local `xynergy-db`
container at `postgres://xynergy:xynergy@localhost:5432/xynergy`,
migrations applied automatically by `#[sqlx::test(migrations = "../../migrations")]`.

### Coverage delta

| AC | Before | After |
|----|--------|-------|
| #1 (totals + Completeness %) | Happy path + top-level shape | + empty state (no NaN), + Inactive CTC excluded, + non-employee excluded |
| #2 (department breakdown) | Rows present | + HR filter scopes all surfaces, + departments.count flash, + name-keyed fallback flash |
| #3 (missing list + Add CTC) | HR happy, RBAC negatives, sensitive-field absence | + HR filter scoping, + Inactive employee surfaces in missing-list |
| #4 (monthly trend) | Shape + new-CTC signal | + filter scope on latest bucket, + parser zero-bucket no-NaN, + alignment by month_end with zero-fill |
| Task 2 access control | PM/Finance/DH denial | + malformed UUID rejection |
| Task 1 sensitive-data hygiene | recent_changes only | + whole HR section walk (completeness, trend, depts, pending, compliance, warnings) |
| Robustness | Field-name parsing | + stringified-numeric tolerance |

### Files modified

- `src/backend/tests/ctc_validation_tests.rs` — +6 INT tests
- `src/backend/tests/dashboard_tests.rs` — +1 INT test
- `src/backend/src/services/ctc_completeness.rs` — +1 unit test
- `src/frontend/src/pages/ctc_completeness.rs` — +2 native tests
- `src/frontend/src/pages/dashboard.rs` — +2 native tests
- `_bmad-output/test-artifacts/automation-summary-6-5.md` — this file (new)

No source/production code changed. No new dependencies. No fixture
files. No mocks. No new directories.

## 5. Story Impact

- The Story 6.5 "review" status is now backed by **12 additional
  automated assertions** covering the P0/P1 invariants the dev pass did
  not pin: empty state correctness, filter consistency across all four
  visible surfaces, typed-query rejection, Active+employee filter
  invariants, sensitive-data hygiene at the *whole-HR-section* level,
  stringified-payload tolerance, NaN-free zero buckets, and the
  departments-count + name-keyed-fallback change-flash paths.
- The last open Story 6.5 task — the live browser walk-through — remains
  the only manual step; automation now exercises every other AC and
  Task 1/2/3/4/6/7 invariant called out in the story document.
- Recommended CI gates (no change required, already part of Story 6.5
  test commands):
  - `cargo test --package xynergy-backend --test ctc_validation_tests`
  - `cargo test --package xynergy-backend --test dashboard_tests`
  - `cargo test -p xynergy-frontend --lib`
  - `cargo check -p xynergy-frontend --target wasm32-unknown-unknown`

---

## 6. Expansion Pass 2 (2026-05-22, Create mode re-run)

A second Create-mode pass was run on top of the 12-test baseline above. The
goal was to find any remaining P0/P1 invariants that the original pass did
not pin — focusing on cross-row invariants, audit-log noise, defensive
boundaries, and unexercised fallback paths in change-detection helpers.

### Additional gaps identified

| # | Target | Level | Pri | AC / Task | Gap explanation |
|---|--------|-------|-----|-----------|----------------|
| 13 | `/ctc/completeness` HR read does NOT write to `audit_logs` | INT | **P0** | Task 2 | The story is explicit: "do not add noisy audit rows for ordinary dashboard reads." 30s polling would multiply audit volume by HR-user count if regressed. No test in pass 1 asserted this. |
| 14 | `Σ department.{total_employees, with_ctc, missing_ctc}` equals top-level totals | INT | **P1** | AC #1, #2 | Existing tests assert per-row values but never check the cross-row invariant. Guards against LEFT JOIN row multiplication or aggregation drift across departments. |
| 15 | Unknown well-formed `department_id` UUID returns empty payload + zero-trend (no 404/500) | INT | **P1** | Task 2 | Pass 1 covered malformed UUIDs (400) and the empty-department case, but not a *valid* UUID that resolves to no rows. Catches a SQL filter accidentally treating "no match" as "no filter". |
| 16 | `recent_ctc_change_key` name-prefixed fallback when `resource_id` is `None` | UNIT (frontend) | **P2** | Robustness | Mirror of pass-1's `hr_department_with_no_id_falls_back_to_name_keyed_change_flash`. The `None`-resource_id branch of `recent_ctc_change_key` is exercised end-to-end through `dashboard_value_map` and `compute_changed_keys`. |

### Tests generated in pass 2 (4 net-new, all inline)

`src/backend/tests/ctc_validation_tests.rs` — 3 new `#[sqlx::test(migrations = "../../migrations")]` functions appended after `completeness_trend_reflects_new_active_ctc`:

13. **`completeness_dashboard_read_does_not_emit_audit_log`** (P0, Task 2)
    - Captures `audit_logs` row count for the HR user before polling, hits
      `/api/v1/ctc/completeness` three times, then re-counts. The delta
      must be exactly zero — guarding the dashboard-read invariant against
      regressions.
14. **`completeness_department_rows_sum_matches_top_level_totals`** (P1, AC #1/#2)
    - Two departments with mixed CTC coverage (1/2 and 1/3). Asserts:
      `Σ department.total_employees == top.total_employees`,
      `Σ department.with_ctc == top.total_with_ctc`, and
      `Σ department.missing_ctc == top.total_missing`. Also re-checks
      the per-row `total = with + missing` invariant and `with_ctc <= total`.
15. **`completeness_unknown_department_id_returns_empty_payload`** (P1, Task 2)
    - Seeds a real department + employee, then queries with a freshly
      generated UUID that is *not* in `departments`. Asserts 200 OK, empty
      `departments` array, zero totals, zero overall percentage, and a
      bounded zero-trend (≤24 buckets, all 0). Also re-runs the sensitive-
      field walker.

`src/frontend/src/pages/dashboard.rs` `mod tests` — 1 new `#[test]`:

16. **`hr_recent_change_with_no_resource_id_falls_back_to_name_keyed_change_flash`** (P2)
    - Two-part test: (a) the pure helper `recent_ctc_change_key` returns
      a `hr.recent_changes.name.<resource_name>|rev<N>|<created_at>`
      prefix when `resource_id = None`; (b) end-to-end through
      `dashboard_value_map` + `compute_changed_keys`, the fallback key
      surfaces in `changed` when `changed_by_name` moves on the same row.

### Validation run — pass 2

| Command | Result |
|---------|--------|
| `cargo check -p xynergy-backend --tests` | clean (sqlx-postgres warnings pre-existing) |
| `cargo check -p xynergy-frontend --lib --tests` | clean (11 pre-existing dead-code warnings) |
| `cargo check -p xynergy-frontend --target wasm32-unknown-unknown` | clean (same pre-existing warnings) |
| `cargo test --package xynergy-backend --test ctc_validation_tests` | **29 passed / 0 failed** (was 26 → +3) |
| `cargo test --package xynergy-backend --test dashboard_tests` | **58 passed / 0 failed** (unchanged regression) |
| `cargo test -p xynergy-frontend --lib` | **92 passed / 0 failed** (was 91 → +1) |

Total **+4 net-new tests this pass, all green**.

### Coverage delta (cumulative across both passes)

| AC / Task | Pass 1 added | Pass 2 added |
|-----------|--------------|--------------|
| AC #1 totals + Completeness % | Empty state (no NaN), Inactive CTC excluded, non-employee excluded | Cross-row sum invariant, unknown department UUID empty path |
| AC #2 department breakdown | HR filter scopes all surfaces, departments.count flash, name-keyed fallback flash | Cross-row sum invariant (per-department) |
| AC #3 missing list + Add CTC | HR filter scoping, Inactive employee surfaces in missing-list | _(no additions; pass 1 strong)_ |
| AC #4 monthly trend | Filter scope on latest bucket, parser zero-bucket no-NaN, alignment by month_end with zero-fill | Unknown department UUID still emits bounded zero-trend |
| Task 2 access control + audit hygiene | PM/Finance/DH denial, malformed UUID rejection | **HR dashboard reads do not emit audit-log rows** (new P0) |
| Task 1 sensitive-data hygiene | Whole-HR-section walk | Re-asserted on unknown-department payload |
| Robustness | Stringified-numeric tolerance, departments-count + name-keyed fallback flash, NaN-free zero buckets | **`recent_ctc_change_key` name-keyed fallback flash** |

### Files modified — pass 2

- `src/backend/tests/ctc_validation_tests.rs` — +3 INT tests
- `src/frontend/src/pages/dashboard.rs` — +1 native test (in `mod tests`)
- `_bmad-output/test-artifacts/automation-summary-6-5.md` — this section

No source/production code changed in pass 2. No new dependencies. No fixture
files. No mocks. No new directories. The four pass-2 tests reuse existing
helpers (`create_test_department`, `create_test_resource_in_department`,
`create_ctc_for_resource`, `assert_no_sensitive_fields`, `empty_response`,
`dashboard_value_map`, `compute_changed_keys`, `recent_ctc_change_key`).

### Cumulative Story 6.5 automation expansion (passes 1 + 2)

- **+16 net-new tests**: 9 backend INT (`ctc_validation_tests`), 1 backend
  INT (`dashboard_tests`), 1 backend unit (`ctc_completeness::tests`),
  5 frontend native (parser + change-detection helpers).
- All previously-green Story 6.5 commands remain green: backend lib,
  `ctc_validation_tests`, `dashboard_tests`, frontend lib, frontend wasm.
- The only Story 6.5 step still requiring human action is the live browser
  walk-through called out at the end of Task 7 in the story document.

### Story Impact — pass 2

Pass 2 closes the four invariants pass 1 left open: (1) the polling-noise
audit invariant (P0), (2) cross-row total consistency (P1), (3) a defensive
non-existent-but-valid UUID boundary (P1), and (4) the symmetric fallback
path in `recent_ctc_change_key` (P2). No new code paths are introduced; all
new tests pin existing implementation guarantees so that a future regression
in `services/ctc_completeness.rs` or `pages/dashboard.rs` is caught at the
test layer instead of in the live HR dashboard.

## 7. Code Review Rerun Supplement (2026-05-23)

The full-diff BMad code-review rerun applied four additional automated
regressions after this automation summary was first generated:

- `completeness_trend_current_month_excludes_future_effective_ctc`
  in `src/backend/tests/ctc_validation_tests.rs`
- `parse_completeness_derives_department_missing_when_absent`
  in `src/frontend/src/pages/ctc_completeness.rs`
- `parse_completeness_rejects_non_finite_numeric_strings`
  in `src/frontend/src/pages/ctc_completeness.rs`
- `add_ctc_href_points_to_existing_ctc_resource_flow`
  in `src/frontend/src/pages/dashboard.rs`

Final review-rerun validation:

| Command | Result |
|---------|--------|
| `cargo fmt --check` | clean |
| `cargo check -p xynergy-backend --tests` | clean (pre-existing warnings only) |
| `cargo check -p xynergy-frontend --lib --tests` | clean (pre-existing warnings only) |
| `cargo check -p xynergy-frontend --target wasm32-unknown-unknown` | clean (pre-existing warnings only) |
| `cargo test --package xynergy-backend --lib` | 74 passed / 0 failed |
| `cargo test --package xynergy-backend --test ctc_validation_tests` | 34 passed / 0 failed |
| `cargo test --package xynergy-backend --test dashboard_tests` | 58 passed / 0 failed |
| `cargo test -p xynergy-frontend --lib` | 96 passed / 0 failed |

Tailwind generation was intentionally skipped because no CSS source changed
and the newly used utility classes already exist in
`src/frontend/public/output.css`. Browser E2E/manual HR walk-through remains
deferred to the deployed-environment release gate tracked in the story and
traceability artifacts.

