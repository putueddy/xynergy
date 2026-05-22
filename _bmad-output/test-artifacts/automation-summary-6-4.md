---
story: '6-4-team-utilization-dashboard'
stepsCompleted:
  - 'step-01-preflight-and-context'
  - 'step-02-identify-targets'
  - 'step-03-generate-tests (sequential mode)'
  - 'step-04-validate-and-summarize'
lastStep: 'step-04-validate-and-summarize'
lastSaved: '2026-05-22'
mode: 'BMad-Integrated'
executionMode: 'sequential (inline generation, no subagent dispatch)'
detectedStack: 'fullstack'
testStackType: 'auto -> fullstack (Rust workspace + Leptos frontend)'
inputDocuments:
  - '_bmad-output/implementation-artifacts/6-4-team-utilization-dashboard.md'
  - '_bmad-output/implementation-artifacts/sprint-status.yaml'
  - '_bmad-output/project-context.md'
  - 'src/backend/src/routes/dashboard.rs'
  - 'src/backend/src/services/dashboard_service.rs'
  - 'src/backend/tests/dashboard_tests.rs'
  - 'src/frontend/src/pages/dashboard.rs'
  - 'src/frontend/src/pages/team.rs'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/test-levels-framework.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/test-priorities-matrix.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/data-factories.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/test-quality.md'
---

# Test Automation Expansion — Story 6.4 Team Utilization Dashboard

## 1. Preflight & Context

### Stack Detection

- `test_stack_type: auto` → resolved to **fullstack** (Rust workspace
  `Cargo.toml` + Leptos `src/frontend/Cargo.toml` + Tailwind
  `src/frontend/package.json`).
- No browser test indicators (`playwright.config.*`, `cypress.config.*`).
- Profile: **API/backend integration + frontend native unit tests**. The
  Department Head dashboard surface is composed entirely of pure helpers
  (threshold, bar-width clamp, badge/label, key derivation, change map),
  which are exercised directly on the host target without WASM.

### Execution Mode

**BMad-Integrated.** Story `6-4-team-utilization-dashboard.md` is currently
in `review` after the dev pass. Existing coverage entering this expansion:

- 9 Story 6.4 backend integration tests (`6.4-INT-001`..`6.4-INT-009` in
  `src/backend/tests/dashboard_tests.rs`).
- 4 route-level unit tests for `resolve_team_range` in
  `src/backend/src/routes/dashboard.rs::tests`.
- 14 Story 6.4 frontend native unit tests for thresholds, badge branches,
  bar clamp, budget health helpers, per-key change detection, sensitive
  field exclusion, and preset labels.

Live Postgres was available (`xynergy-db` docker container, port 5432
reachable, migrations applied), so backend integration tests were executed
against a real database, not skipped.

### Test Framework Verified

- Backend integration tests use `#[sqlx::test(migrations = "../../migrations")]`
  with automatic transactional isolation per test.
- Frontend native tests gated on
  `#[cfg(all(test, not(target_arch = "wasm32")))]` inside
  `src/frontend/src/pages/dashboard.rs::tests`.

## 2. Coverage Gaps Identified

Before this run, the following Story-6.4–scoped behaviours had no direct
automated coverage:

| ID            | Risk            | Behaviour                                                                                          |
| ------------- | --------------- | -------------------------------------------------------------------------------------------------- |
| 6.4-INT-010   | P1 (data shape) | `DH_CURRENT_PROJECTS_PER_MEMBER_LIMIT=10` cap on nested current_projects per team member.          |
| 6.4-INT-011   | P0 (boundary)   | Strict `< 50.0%` underutilized threshold — INT-002 covered 25/70/90 only, not the 49.9/50.0 edge.  |
| 6.4-INT-012   | P0 (assign guard) | Missing-CTC member surfaced with non-Active `ctc_status` so the `/team` Assign deep-link disables. |
| 6.4-INT-013   | P2 (contract)   | Trend bundle echoes both `start_date` and `end_date` (INT-003 only asserted `end_date`).            |
| 6.4-INT-014   | P1 (empty state) | DH with an empty department renders structured empty lists, not nulls.                              |
| 6.4-INT-015   | P1 (route)      | Partial-query: only `team_start_date` supplied — must use default end and stay 200.                 |
| FE-bar-INF    | P2 (defensive)  | `bar_width_pct` clamps `±INFINITY` (existing test only covers NaN/negative/overflow).               |
| FE-badge-50   | P0 (boundary)   | Badge at exactly 50% is `badge-neutral` / `Available` (mirrors strict backend threshold).           |
| FE-badge-100  | P1 (boundary)   | Badge at exactly 100% is `badge-positive` / `Healthy`, not Overallocated.                           |
| FE-rowdrop    | P1 (highlight)  | Aggregate `member_count` flashes when a row drops out of the team table.                            |
| FE-crossthres | P0 (highlight)  | Aggregate `underutilized_count` flashes when a member crosses the 50% threshold.                    |
| FE-pctflip    | P1 (highlight)  | `current_projects` key flashes when only the allocation percentage of an existing project changes.  |
| FE-avg-empty  | P1 (empty state) | `avg_available_capacity_pct` renders as `0.0` (never NaN) when team is empty.                       |

## 3. Tests Added

### Backend integration (6 tests, `src/backend/tests/dashboard_tests.rs`)

1. **`dept_head_current_projects_capped_at_limit`** — Seeds 12 distinct
   active projects on a single resource (each at 1% so utilization stays
   under 50% and isolates the project-list cap from the overallocation
   branch); asserts `current_projects.len() == 10`.
2. **`dept_head_underutilized_boundary_excludes_exactly_50_pct`** — Two
   members at exactly 50.0% and 49.9%; asserts only the 49.9% member
   appears in `underutilized_members` and the 50.0% member has
   `is_underutilized=false` on the wider `team_members` payload.
3. **`dept_head_missing_ctc_member_surfaced_with_non_active_status`** —
   Member without a CTC record must still appear in `team_members` with
   `ctc_status != "Active"` (frontend Assign guard relies on this).
4. **`dept_head_trend_bundle_echoes_start_and_end_dates`** — Supplies a
   fixed `team_start_date`/`team_end_date` and asserts both echo back on
   the trend bundle (INT-003 previously only checked `end_date`).
5. **`dept_head_with_empty_team_returns_structured_empty_lists`** —
   Department Head with no resources gets `team_members=[]`,
   `underutilized_members=[]`, `utilization_trends.members=[]`, and a
   numeric `average_utilization_pct` (never null).
6. **`dept_head_partial_range_only_start_date_uses_default_end`** —
   Sends only `team_start_date`; asserts `200 OK`, that the supplied
   start_date is honored, and the default end_date is a valid ISO date.

### Frontend native unit (7 tests, `src/frontend/src/pages/dashboard.rs::tests`)

1. **`bar_width_pct_clamps_infinities`** — `INFINITY → 100.0`,
   `NEG_INFINITY → 0.0`.
2. **`utilization_badge_at_exactly_50_pct_is_neutral_available`** — Strict
   threshold parity with backend: 50.0% → `badge-neutral` / `Available`.
3. **`utilization_badge_at_100_pct_is_healthy_not_overallocated`** —
   100.0% is the upper boundary of `Healthy`; the Overallocated branch
   must fire only above 100% or when the flag is set.
4. **`dh_member_count_flashes_when_row_removed`** — Removing a member
   flashes `department_head.team.member_count` and does *not* re-emit
   per-row keys for the absent member.
5. **`dh_underutilized_count_flashes_when_member_crosses_threshold`** —
   Same resource_id moves from 40% → 60%; both
   `department_head.team.underutilized_count` and the per-member
   `is_underutilized` key flash.
6. **`dh_current_projects_flashes_on_allocation_pct_change_only`** —
   Same project name and dates, allocation % moves from 20 → 35; the
   bundled `current_projects` key changes because the value embeds the %.
7. **`dh_avg_available_capacity_is_zero_when_team_is_empty`** — Pure
   aggregate guard against NaN division-by-zero on empty team.

## 4. Validation Results

| Suite                                                      | Before | After | Notes                                                                                       |
| ---------------------------------------------------------- | ------ | ----- | ------------------------------------------------------------------------------------------- |
| `cargo test --package xynergy-backend --test dashboard_tests` | 46     | **57** | +6 INT-010..INT-015 plus review regressions for malformed/duplicate DH range params, non-DH range immunity, and DH RBAC all green. |
| `cargo test --package xynergy-backend --test team_tests`      | 10     | 10    | No regression.                                                                              |
| `cargo test --package xynergy-backend --test budget_tests`    | 18     | 18    | No regression.                                                                              |
| `cargo test --package xynergy-backend --test overallocation_tests` | 7      | 7     | No regression.                                                                              |
| `cargo test -p xynergy-frontend --lib`                        | 64     | **77** | +7 expansion tests plus review regressions for UTC range presets and dashboard change keys all green. |
| `cargo check -p xynergy-frontend --target wasm32-unknown-unknown` | clean | clean | Same 9 pre-existing dead-field warnings (`team.rs` `BudgetBreakdownResponse`, etc.).        |

No flaky behaviour observed in the executed runs. Separate burn-in was not
performed; backend tests use sqlx transactional isolation, so order independence
is expected from the test harness.

## 5. Risk Coverage Map (Story 6.4)

| AC  | Story Behaviour                                                                                      | Pre-existing Coverage                                                                            | Expansion Coverage                                                                                                                                                                |
| --- | ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #1  | Dashboard shows team members, current utilization %, current projects, available capacity.          | INT-001, INT-005 (cross-dept exclusion), frontend badge/label, key-stable tests.                  | INT-010 (per-member project cap), INT-014 (empty team structure), FE row-drop + avg-empty tests.                                                                                  |
| #2  | Utilization trends per member for selected range.                                                    | INT-003 (range expands periods, end_date echoed), FE trend stable-key + change-detection tests.   | INT-013 (start_date also echoed), INT-015 (partial range branch).                                                                                                                 |
| #3  | Underutilized < 50% → Assign action that reuses `/team` workflow.                                    | INT-002 (25/70/90), FE strict-threshold helper test.                                              | INT-011 (49.9/50.0 boundary on backend), INT-012 (missing-CTC assign guard contract), FE badge-50/100 boundaries, FE underutilized-count flash.                                   |
| #4  | Department budget summary with gauge (Total, Committed, Spent, Available + visual gauge).           | INT-004 (gauge fields), FE `dh_budget_spent_actual_and_threshold_keys_present`, FE budget_health. | Boundary/empty-state coverage above keeps the gauge from rendering against undefined averages; existing 6.4 tests already cover the field surface.                              |

## 6. Files Touched

- `src/backend/tests/dashboard_tests.rs` — added 6 integration tests
  (6.4-INT-010..6.4-INT-015) in a dedicated expansion block, plus one review
  regression for malformed non-DH team range params.
- `src/frontend/src/pages/dashboard.rs` — added 7 native unit tests at the
  end of the existing `tests` module.

No production code changed during the test-automation expansion itself. Later Story 6.4 code-review patches are tracked in the story file and gate artifacts.

## 7. Recommendations & Next Steps

- Story 6.4 currently lacks a true E2E (browser) test for the Assign
  deep-link round-trip (`/dashboard` → `/team?assign_resource_id=…` →
  modal open → query-param clean). The dev notes already flag that the
  manual browser verification was not executed in the dev pass — this
  remains the highest-value next gap, but it is outside the
  no-Playwright posture of this run.
- A near-zero-cost follow-up would be a backend integration test that
  asserts the request returns identical Story 6.1 fields
  (`utilization`, `budget`, `overallocations`, `upcoming_assignments`,
  `warnings`) for a Department Head when the new 6.4 fields are present,
  guarding against a future regression that silently drops a legacy
  Department Head field. (Coverage today is implicit through the
  Story 6.1 tests still passing.)
- Consider promoting the Story 6.4 `Status` from `review` to `done` once
  the manual browser pass is performed; the regression net is now
  thorough on both backend and frontend.
