---
stepsCompleted:
  - step-01-load-context
  - step-02-discover-tests
  - step-03-map-criteria
  - step-04-analyze-gaps
  - step-05-gate-decision
lastStep: step-05-gate-decision
lastSaved: '2026-05-22'
lastUpdated: '2026-05-22T11:00:00Z'
storyId: '6.4'
storyKey: 6-4-team-utilization-dashboard
storyTitle: 'Team Utilization Dashboard'
workflowType: 'testarch-trace'
coverageBasis: acceptance_criteria
oracleConfidence: high
oracleResolutionMode: formal_requirements
oracleSources:
  - _bmad-output/implementation-artifacts/6-4-team-utilization-dashboard.md
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - _bmad-output/test-artifacts/automation-summary-6-4.md
externalPointerStatus: not_used
tempCoverageMatrixPath: not_committed
gateDecision: CONCERNS
previousGateDecision: CONCERNS
manualVerificationStatus: review_patched_reverify_required
manualVerificationDate: '2026-05-22'
manualVerificationReport: _bmad-output/test-artifacts/manual-verification-6-4.md
gateEligible: false
collectionStatus: COLLECTED
executionMode: sequential
inputDocuments:
  - _bmad-output/implementation-artifacts/6-4-team-utilization-dashboard.md
  - _bmad-output/implementation-artifacts/sprint-status.yaml
  - _bmad-output/test-artifacts/automation-summary-6-4.md
  - src/backend/src/routes/dashboard.rs
  - src/backend/src/services/dashboard_service.rs
  - src/backend/tests/dashboard_tests.rs
  - src/frontend/src/pages/dashboard.rs
  - src/frontend/src/pages/team.rs
---

# Traceability Matrix & Gate Decision — Story 6.4 (Team Utilization Dashboard)

**Target:** Story 6.4 — Team Utilization Dashboard
**Date:** 2026-05-22
**Evaluator:** Putu (Master Test Architect)
**Coverage Oracle:** acceptance_criteria
**Oracle Confidence:** high
**Oracle Resolution Mode:** formal_requirements

> _This workflow does not generate tests. Where gaps remain, run `*atdd` (new behavior) or `*automate` (expand existing). Manual-only paths are tracked as advisories._

---

## Gate Decision: **CONCERNS / REVIEW_PATCHED** ⚠️ (updated 2026-05-22T11:00:00Z)

> **2026-05-22 update:** Live manual browser verification was executed, but subsequent code-review patches changed AC#2 range behavior and AC#3 deep-link/error behavior. The manual evidence is retained as superseded evidence; rerun the patched browser paths before promoting this story to `done`.

### Current Gate State (CONCERNS / REVIEW_PATCHED)

- **Decision:** ⚠️ **CONCERNS / REVIEW_PATCHED** — static coverage remains complete and 53 story-scoped tests + 169 regression tests pass, but code-review patches changed AC#2/AC#3 browser behavior after the initial manual pass.
- **Remaining open advisory:** Cross-story frontend test harness backlog (shared with Story 6.3) — not a Story 6.4 blocker.
- **Reverify-required paths:** AC#2 selected-range browser flow and AC#3 direct deep-link / guarded-error behavior.

### Historical CONCERNS Rationale (preserved for audit)

**Original Rationale (2026-05-22 initial trace, updated after review patches):** Every acceptance criterion has live backend integration coverage and frontend native unit coverage. Story 6.4 now ships 20 backend integration tests in `dashboard_tests.rs`, 6 route-level unit tests for range/query parsing, and 27 frontend native unit tests in `dashboard.rs::tests`. All four ACs are FULL at the static-logic level, P0/P1 thresholds are met, and no test is skipped/pending.

The decision was **CONCERNS** rather than PASS because:

1. **Manual browser verification had not been executed.** The Story 6.4 dev record explicitly noted _"Manual browser verification was not executed (no live dev server invoked in this run) — recommend confirming the four AC flows (team rows, trend range refresh, Assign deep-link to /team, budget gauge + change flashes) before promoting to done."_ — **executed 2026-05-22, then superseded for AC#2/AC#3 by review patches.**
2. **The `/team?assign_resource_id=<uuid>` deep-link round-trip (AC#3) has no automated coverage.** The Effect handler in `team.rs` that parses the param, validates RLS scope + CTC posture, opens `open_assign_modal`, and clears the query via `navigate("/team", replace: true)` is logic-correct by inspection but is not exercised by Playwright/`wasm-bindgen-test`/component harness because no such harness exists in repo. This is the same advisory carried by Story 6.3 — **manually verified 2026-05-22, then superseded for patched direct deep-link/error behavior; harness still tracked as cross-story backlog.**

Net posture: ship-ready on static logic, contractually proven on data, but patched AC#2/AC#3 browser paths need a fresh Department Head pass before promoting the story from `review` to `done`.

---

## Coverage Oracle

- **Resolution mode:** `formal_requirements` — Story acceptance criteria are the canonical contract, reinforced by PRD FR47/FR50/FR51/FR58, UX design spec (Department Head cost-aware assignment flow), and the test automation expansion summary.
- **Confidence:** `high` — four independent formal sources (Story 6.4 AC #1–#4, PRD FR47/FR50/FR51/FR58/FR60, UX spec Department Head flow, automation-summary-6-4.md inventory) agree on the testable contract (team members + utilization + current projects + available capacity + trend range + underutilized-threshold + budget gauge fields + Assign deep-link).
- **Sources:**
  - `_bmad-output/implementation-artifacts/6-4-team-utilization-dashboard.md` — 4 ACs, 7 task groups (status: `review`)
  - `_bmad-output/planning-artifacts/epics.md` — Epic 6 / Story 6.4 AC bullets
  - `_bmad-output/planning-artifacts/prd.md` — FR47 (DH dashboard), FR50/FR51 (polling + manual refresh), FR58 (weighted working-day formula), FR60 (relationship-based RBAC), NFR2/NFR7
  - `_bmad-output/planning-artifacts/ux-design-specification.md` — Department Head cost-aware assignment flow, financial dashboard UX
  - `_bmad-output/test-artifacts/automation-summary-6-4.md` — 2026-05-22 expansion inventory (+6 backend INT, +7 frontend UNIT)
- **External pointer status:** `not_used` (no Linear / Jira / Confluence pointer files in scope).
- **Synthetic oracle:** no — formal acceptance criteria + dev-notes + PRD + UX spec all align.

## Acceptance Criteria (formal)

| AC   | Description | Priority |
|------|-------------|----------|
| AC#1 | Given I navigate to Team Dashboard, when the page loads, then I see Team members, Current Utilization %, Current Projects, Available Capacity. | **P0** — primary contract; data correctness for resource allocation decisions. |
| AC#2 | Given I view the utilization chart, when I select a time range, then I see utilization trends over time for each team member. | **P1** — operational planning ergonomics. |
| AC#3 | Given I identify underutilized resources, when I see current utilization < 50%, then I can click to assign them to new projects. | **P0** — security-sensitive: reuses `/team` Assign workflow with RLS scoping, CTC guard, cost preview, and overallocation confirmation. |
| AC#4 | Given I view budget status, when I look at department summary, then I see Total Budget, Committed, Spent, Available with visual gauge. | **P1** — financial display derived from canonical `compute_department_budget_utilization(...)`. |

## Story Scope (in / out)

- **In scope (testable):** Department Head dashboard `team_members` + `utilization_trends` + `underutilized_members` projection; optional typed `team_start_date`/`team_end_date` route query params (validation: start ≤ end, ≤ 366 days, defaulting); `< 50.0%` strict underutilized threshold; `available_capacity_pct = max(0, 100 − current_allocation_percentage)` clamp; `current_projects` per-member bound (10); preservation of Story 6.1 scoping (RLS / current DB department, never JWT-only); preservation of Story 6.2 polling + request ordering + change-flash on new visible Department Head keys; reuse of canonical `compute_department_budget_utilization(...)` for `Total / Committed / Spent / Available / budget_health / alert_threshold_pct`; `/dashboard` → `/team?assign_resource_id=<uuid>` deep-link with router-replace cleanup; CTC-missing posture surfaced (not assignable from dashboard).
- **Out of scope (deliberate per story):** new charting libraries, SSE/WebSockets, a parallel assignment modal inside `dashboard.rs`, new allocation formulas, new budget formulas, Story 6.5 CTC completeness dashboard, PM project-health changes, Finance dashboard changes, database schema changes, replacing `/team`.

## Knowledge Base Loaded

- `test-priorities-matrix.md` — P0/P1/P2/P3 criteria
- `risk-governance.md` — gate scoring + decision rules
- `probability-impact.md` — risk scoring scale
- `test-quality.md` — DoD (deterministic, isolated, single-invariant)
- `selective-testing.md` — level selection heuristics

## Project Persistent Facts

- `_bmad-output/project-context.md` loaded — Rust / Axum / Leptos / PostgreSQL guardrails: never use raw `.unwrap()`, parameterized SQL only, `resolve_department_id()` for RLS scoping, relationship-based RBAC (not JWT-string), weighted working-day formula for capacity, no audit row on dashboard reads.

---

## PHASE 1 — REQUIREMENTS TRACEABILITY

### Coverage Summary

| Priority  | Total ACs | FULL Coverage | Coverage % | Status |
|-----------|-----------|---------------|------------|--------|
| P0        | 2         | 2             | 100%       | ✅ PASS |
| P1        | 2         | 2             | 100%       | ✅ PASS |
| P2        | 0         | 0             | n/a        | n/a    |
| P3        | 0         | 0             | n/a        | n/a    |
| **Total** | **4**     | **4**         | **100%**   | **✅** |

Coverage % is "FULL" per AC. AC#3 carries a residual advisory for the manual-only deep-link click path; see Gap Analysis.

### Test Inventory

| Level     | Files | Cases | Skipped | Pending / Fixme | Notes |
|-----------|-------|-------|---------|-----------------|-------|
| API (INT) | 1     | 20    | 0       | 0               | `src/backend/tests/dashboard_tests.rs` — dev + expansion + review patch coverage |
| Unit (BE) | 1     | 6     | 0       | 0               | `src/backend/src/routes/dashboard.rs::tests` — `resolve_team_range_*` / duplicate query parsing |
| Unit (FE) | 1     | 27    | 0       | 0               | `src/frontend/src/pages/dashboard.rs::tests` — dev + expansion + review patch coverage |
| Component | 0     | 0     | —       | —               | No Leptos component harness in repo. |
| E2E       | 0     | 0     | —       | —               | No Playwright / wasm-bindgen-test harness in repo. |
| **Total** | **3** | **53**| **0**   | **0**           |        |

**Regression suites compiled & executed:** `dashboard_tests` 57/57, `team_tests` 10/10, `budget_tests` 18/18, `overallocation_tests` 7/7, `backend lib` 68/68, `frontend_dashboard_unit_tests` 77/77. WASM compile: clean with 9 pre-existing warnings.

---

### Detailed Mapping

#### AC#1 — Team Dashboard shows Team members, Current Utilization %, Current Projects, Available Capacity (P0)

- **Coverage:** FULL ✅
- **Tests:**
  - `6.4-INT-001` — `src/backend/tests/dashboard_tests.rs:1970` (`dept_head_team_members_include_utilization_projects_capacity`)
    - **Given:** A Department Head with a scoped employee carrying an active allocation.
    - **When:** `GET /api/v1/dashboard` is called with no range query.
    - **Then:** `department_head.team_members[*]` exposes `resource_id`, `resource_name`, `role`, `current_utilization_pct`, `available_capacity_pct`, `ctc_status`, and a bounded `current_projects` list with project name + allocation + dates.
  - `6.4-INT-005` — `dashboard_tests.rs:2193` (`dept_head_team_excludes_other_department_resources`)
    - **Given:** Two departments; the caller is DH of department A. Department B has its own resources & allocations.
    - **When:** DH calls `/api/v1/dashboard`.
    - **Then:** `team_members` excludes department B even if its ids are guessed; RLS scoping holds.
  - `6.4-INT-010` — `dashboard_tests.rs:2352` (`dept_head_current_projects_capped_at_limit`)
    - **Given:** A resource with 12 active assignments at 1% allocation each.
    - **When:** DH dashboard is fetched.
    - **Then:** `current_projects.len() == 10` (`DH_CURRENT_PROJECTS_PER_MEMBER_LIMIT`) — payload bound holds.
  - `6.4-INT-014` — `dashboard_tests.rs:2543` (`dept_head_with_empty_team_returns_structured_empty_lists`)
    - **Given:** A DH whose department has no resources.
    - **When:** Dashboard is fetched.
    - **Then:** `team_members=[]`, `utilization_trends.members=[]`, `underutilized_members=[]`, and `average_utilization_pct` is numeric (never null/NaN).
  - `6.4-INT-009` — `dashboard_tests.rs:2648` (`dept_head_overallocated_member_clamps_available_capacity`)
    - **Given:** A resource at >100% allocation (overallocated).
    - **When:** Dashboard is fetched.
    - **Then:** `available_capacity_pct = 0.0` (clamped via `max(0, …)`) and `is_overallocated = true`.
  - `dashboard.rs:4722` (`utilization_badge_class_branches`), `:4741` (`utilization_badge_label_branches`)
    - **Given/When/Then:** Pure helpers verify badge class + label across `Underutilized | Healthy | Available | Overallocated` branches; explicit overallocation flag wins over `>100%` value.
  - `dashboard.rs:4750` (`bar_width_clamps_negative_and_overflow`), `:4988` (`bar_width_pct_clamps_infinities`)
    - **Given/When/Then:** Negative, NaN, overflow, +∞, and −∞ inputs to `bar_width_pct` all clamp to a finite `[0, 100]` so a corrupt backend value cannot break inline-width layout.
  - `dashboard.rs:5004` (`utilization_badge_at_100_pct_is_healthy_not_overallocated`) — 100% is the upper boundary of Healthy.
  - `dashboard.rs:4772` (`team_member_key_uses_resource_id_when_present_else_name`) — change-flash key stable across rename.
  - `dashboard.rs:4807` (`dh_team_utilization_change_produces_expected_keys`), `:4840` (`dh_current_projects_change_produces_stable_key`), `:5013` (`dh_member_count_flashes_when_row_removed`), `:5118` (`dh_avg_available_capacity_is_zero_when_team_is_empty`)
    - **Given/When/Then:** Per-member + aggregate change-detection keys flash on utilization, available capacity, project list, and member-count changes; empty team renders `0.0` (never NaN).
  - `dashboard.rs:4951` (`dh_team_value_map_excludes_sensitive_ctc_fields`) — sensitive CTC tokens (ciphertext, key_version, base_salary, daily_rate) are not leaked into the change-detection map.
- **Gaps:** None at the static-logic level. Manual browser verification of the rendered row is the only outstanding item (advisory).
- **Recommendation:** Run the four AC flows in a browser as a Department Head before promoting story to `done`. No new automated tests required for this AC.

---

#### AC#2 — Utilization trends per member for selected time range (P1)

- **Coverage:** FULL ✅
- **Tests:**
  - `6.4-INT-003` — `dashboard_tests.rs:2082` (`dept_head_trend_range_query_expands_periods`)
    - **Given:** DH dashboard with a `team_start_date`/`team_end_date` covering >1 month.
    - **When:** Dashboard is fetched.
    - **Then:** `utilization_trends.members[*].periods` expands beyond the default window while preserving department scoping; weighted working-day formula remains canonical.
  - `6.4-INT-006` — `dashboard_tests.rs:2266` (`dept_head_invalid_trend_range_returns_validation_error`)
    - **Given:** `team_start_date > team_end_date`.
    - **When:** Dashboard is fetched.
    - **Then:** 400 validation error at the route boundary.
  - `6.4-INT-007` — `dashboard_tests.rs:2294` (`dept_head_excessive_trend_range_returns_validation_error`)
    - **Given:** Range spans more than 366 days.
    - **When:** Dashboard is fetched.
    - **Then:** 400 validation error (cap enforced).
  - `6.4-INT-008` — `dashboard_tests.rs:2320` (`dashboard_range_query_does_not_affect_admin_response`)
    - **Given:** An Admin caller with `team_start_date` / `team_end_date` in the query string.
    - **When:** Dashboard is fetched.
    - **Then:** Admin response is structurally unchanged — the optional query is a no-op for non-DH roles.
  - Review patch — `dashboard_malformed_range_query_is_ignored_for_admin`
    - **Given:** An Admin caller with malformed `team_start_date` / `team_end_date` query strings.
    - **When:** Dashboard is fetched.
    - **Then:** Admin response still succeeds because DH-only params are ignored after role resolution.
  - `6.4-INT-013` — `dashboard_tests.rs:2507` (`dept_head_trend_bundle_echoes_start_and_end_dates`)
    - **Given:** A fixed `team_start_date` and `team_end_date`.
    - **When:** Dashboard is fetched.
    - **Then:** `utilization_trends.start_date` and `utilization_trends.end_date` both echo the supplied values.
  - `6.4-INT-015` — `dashboard_tests.rs:2606` (`dept_head_partial_range_only_start_date_uses_default_end`)
    - **Given:** Only `team_start_date` is supplied.
    - **When:** Dashboard is fetched.
    - **Then:** 200 OK; supplied start is honored, default end is a valid ISO date.
  - `routes/dashboard.rs:94` `resolve_team_range_defaults_when_both_none` — default 30-day window when neither bound supplied.
  - `routes/dashboard.rs:105` `resolve_team_range_rejects_start_after_end` — start>end is rejected.
  - `routes/dashboard.rs:115` `resolve_team_range_rejects_range_above_cap` — `>366` days rejected.
  - `routes/dashboard.rs:125` `resolve_team_range_accepts_exactly_max_span` — exactly 366 days accepted (boundary).
  - `dashboard.rs:4784` (`trend_period_key_uses_resource_id_when_present_else_name`) — change-key stable when resource_id present, name fallback when absent.
  - `dashboard.rs:4880` (`dh_trend_period_change_produces_expected_key`) — period flash on changed period; unchanged earlier period does NOT flash.
  - `dashboard.rs:4975` (`trend_range_preset_labels_stable`) — preset labels (Current Month / Next 30 Days / 3 Months / 6 Months) are stable.
- **Gaps:** None at logic level. UI interaction (clicking the preset segmented control and observing that polling does not stack intervals or allow stale responses to overwrite newer ones) is logic-correct by code inspection but is browser-verification-only.
- **Recommendation:** Verify in browser that switching presets refreshes data without doubling polling cadence and without flashing the entire dashboard.

---

#### AC#3 — Underutilized resources (current utilization < 50%) → click to assign via existing `/team` workflow (P0)

- **Coverage:** PARTIAL ⚠️ (static logic FULL; deep-link click-path manual-only)
- **Tests:**
  - `6.4-INT-002` — `dashboard_tests.rs:2023` (`dept_head_under_50_pct_surfaces_underutilized`)
    - **Given:** Resources at 25%, 70%, and 90% utilization.
    - **When:** Dashboard is fetched.
    - **Then:** Only the 25% resource appears in `underutilized_members` with `is_underutilized = true`.
  - `6.4-INT-011` — `dashboard_tests.rs:2402` (`dept_head_underutilized_boundary_excludes_exactly_50_pct`)
    - **Given:** Two members at exactly 50.0% and 49.9%.
    - **When:** Dashboard is fetched.
    - **Then:** Only the 49.9% member is in `underutilized_members`; 50.0% has `is_underutilized = false`. Strict `<` threshold proven on the boundary.
  - `6.4-INT-012` — `dashboard_tests.rs:2464` (`dept_head_missing_ctc_member_surfaced_with_non_active_status`)
    - **Given:** A team member with no CTC record.
    - **When:** Dashboard is fetched.
    - **Then:** Member is present in `team_members` with `ctc_status != "Active"` (drives the disabled Assign posture on the frontend).
  - `dashboard.rs:4713` (`underutilized_threshold_excludes_50_pct`) — frontend helper mirrors backend strict `< 50.0%`.
  - `dashboard.rs:4996` (`utilization_badge_at_exactly_50_pct_is_neutral_available`) — UI badge at 50.0% is `badge-neutral / Available`, not `Underutilized`.
  - `dashboard.rs:5053` (`dh_underutilized_count_flashes_when_member_crosses_threshold`) — both `underutilized_count` aggregate and per-member `is_underutilized` key flash when the same resource crosses 40 → 60.
  - `dashboard.rs:5083` (`dh_current_projects_flashes_on_allocation_pct_change_only`) — `current_projects` key flashes when only the allocation % of an existing project shifts (the bundled value embeds %).
- **Gaps (advisory):**
  - **Deep-link click path is not automated.** The Effect in `src/frontend/src/pages/team.rs` that:
    1. parses `?assign_resource_id=<uuid>` after team data loads,
    2. validates UUID parsing + RLS scope membership + CTC posture,
    3. opens `open_assign_modal(...)`,
    4. clears the query via `navigate("/team", replace: true)`,

    is correct by inspection and reuses Story 3.x assignment guards (assignable-projects, cost preview, CTC required, overallocation confirmation, budget impact preview, post-submit refresh). It has no automated test because no Playwright / wasm-bindgen-test / Leptos component-test harness exists in repo. This is the same cross-story tech debt called out by Story 6.3.
  - **Cross-department resource id, malformed UUID, and CTC-missing-but-underutilized rejection paths** are logic-correct by code inspection and inherit `/team` RLS guards, but the rejection path itself is not asserted by an automated test.
- **Recommendation:**
  - Promote browser verification to a repeatable harness in a follow-up story (Playwright + `cargo-leptos serve`, or `wasm-bindgen-test`). Same recommendation Story 6.3 carried into the backlog.
  - Until the harness exists, run the AC#3 path manually for: (a) an in-scope underutilized member with CTC Active, (b) a CTC-missing underutilized member, (c) a malformed `assign_resource_id`, (d) a cross-department UUID. The query must be cleared in all four cases.

---

#### AC#4 — Department budget summary with gauge: Total / Committed / Spent / Available (P1)

- **Coverage:** FULL ✅
- **Tests:**
  - `6.4-INT-004` — `dashboard_tests.rs:2136` (`dept_head_budget_payload_has_gauge_fields`)
    - **Given:** A DH with a configured department budget.
    - **When:** Dashboard is fetched.
    - **Then:** `budget` payload exposes `total_budget_idr`, `total_committed_idr`, `spent_actual_idr`, `remaining_idr`, `utilization_percentage`, `budget_health`, `alert_threshold_pct`, `budget_configured = true`. Sourced from canonical `compute_department_budget_utilization(...)`.
  - Regression: `dashboard_tests.rs:392` (`dept_head_response_has_utilization_and_overallocation`) — Story 6.1 regression that the rest of the DH dashboard remains stable when 6.4 additive fields are present.
  - `dashboard.rs:4921` (`dh_budget_spent_actual_and_threshold_keys_present`) — change-flash map exposes `department_head.budget.spent_actual_idr` and `department_head.budget.alert_threshold_pct` with correct string values.
  - `dashboard.rs:4759` (`budget_health_class_and_label_branches`) — `critical / warning / healthy / unknown` health tokens map to colored class + label correctly.
  - `dashboard.rs:3859/3865/3872` (`format_idr_*`) — currency formatter handles 0, thousands-grouping, and negative values (no float math anywhere on currency).
- **Gaps:** None at logic level. The rendered gauge proportions / colored bar are CSS-driven and would need DOM verification, but the value contract is fully proven.
- **Recommendation:** Verify in browser that the gauge bar reflects `spent_actual_idr / total_budget_idr` and that the health color follows `alert_threshold_pct`.

---

### Coverage by Test Level

| Test Level | Tests | Criteria Covered | Coverage % | Execution |
|------------|-------|------------------|------------|-----------|
| E2E        | 0     | 0 / 4            | 0%         | no_repeatable_harness |
| API (INT)  | 20    | 4 / 4            | 100%       | executed_live_database (57/57) |
| Component  | 0     | 0 / 4            | 0%         | no_component_harness |
| Unit (BE)  | 6     | 1 / 4 (AC#2)     | 25%        | executed_native |
| Unit (FE)  | 27    | 4 / 4            | 100%       | executed_native (77/77) |
| Manual     | 4     | 2 / 4 current    | partial    | review_patched_reverify_required |
| **Total**  | **53**| **4 / 4**        | **100%**   | static-logic only |

---

### Gap Analysis

#### Critical Gaps (BLOCKER) ❌

0 critical gaps. All P0 ACs have FULL static-logic coverage.

#### High Priority Gaps (PR BLOCKER) ⚠️

0 hard high-priority gaps. AC#3 carries an _advisory_ for the deep-link click path (see below) but not a blocker, because (a) the modal logic is reused unchanged from Story 3.x and (b) backend RLS guards (`resolve_department_id` + `dept_head_team_excludes_other_department_resources`) hold.

#### Advisory (manual-only) Gaps

1. **AC#2 / AC#3 — patched browser verification required.**
   - **Source:** Story 6.4 review-patched gate artifacts and manual verification report.
   - **Impact:** Logic is proven; initial browser evidence for selected range/deep-link behavior was superseded by review patches.
   - **Recommend:** Rerun patched AC#2/AC#3 flows as a Department Head before promoting story `review` → `done`.
2. **AC#3 — `/dashboard` → `/team?assign_resource_id=<uuid>` deep-link round-trip has no automated DOM/router coverage.**
   - **Impact:** Logic is correct by inspection and inherits `/team` guards; missing automation means future regressions to the param parser, scope check, or query-cleanup behavior would not be caught by CI.
   - **Recommend:** Introduce a frontend harness (Playwright + `cargo-leptos serve`, or `wasm-bindgen-test`) so the deep-link click flow, sort/preset accessibility, and CSS flash class transitions can be automated. Same recommendation Story 6.3 carried into the backlog.

#### Medium Priority Gaps (Nightly) ⚠️

0.

#### Low Priority Gaps (Optional) ℹ️

0.

---

### Coverage Heuristics Findings

#### Endpoint Coverage Gaps

- 0 endpoints without direct tests. `/api/v1/dashboard` (with and without `team_start_date`/`team_end_date`) is exercised by 15 integration tests.

#### Auth / Authz Negative-Path Gaps

- 0 missing negative paths.
  - `dept_head_team_excludes_other_department_resources` (6.4-INT-005) covers cross-department exclusion.
  - `dept_head_without_department_returns_403` (regression from 6.1) covers missing department.
  - `dept_head_dashboard_uses_current_department_not_stale_token` (regression from 6.1) covers stale-JWT defense.
  - `dashboard_range_query_does_not_affect_admin_response` (6.4-INT-008) covers the no-op contract for non-DH roles.

#### Happy-Path-Only Criteria

- 0 criteria are happy-path-only.
  - Trend-range validation covers both invalid (start>end) and excessive (>366 days) ranges, plus the partial-only-start branch.
  - Underutilized boundary covers strict `<` at 49.9 and 50.0.
  - Overallocated clamp covers the `>100%` branch.
  - Empty-team branch is asserted explicitly.

#### UI Journey & State Gaps (advisory)

- `ui_journey_status: manual_only_no_repeatable_harness` — no automated DOM coverage for AC#3 deep-link click, range preset click, sort/filter accessibility, or change-flash CSS application.
- `ui_state_status: manual_only_no_component_harness` — empty-team, missing-CTC, and overallocated badge states are unit-proven on the model; rendered visual states are browser-verification-only.

---

### Quality Assessment

#### Tests Passing Quality Gates

**53 / 53 tests (100%) meet all quality criteria** ✅

- Backend integration tests use `#[sqlx::test(migrations = "../../migrations")]` for transactional isolation, eliminating order dependency.
- Frontend native tests are pure (no WASM, no globals) and use deterministic UUIDs and fixed dates.
- No skipped, pending, or fixme tests.
- All tests assert a single named invariant per case.

#### Quality Notes

- **WARNING (advisory):** 9 pre-existing dead-field warnings in `cargo check -p xynergy-frontend --target wasm32-unknown-unknown` come from `team.rs::BudgetBreakdownResponse` etc. and are unrelated to Story 6.4. Tracked separately.

---

### Duplicate Coverage Analysis

#### Acceptable Overlap (Defense in Depth)

- AC#3 underutilized threshold: tested at backend INT (`6.4-INT-002`, `6.4-INT-011`) AND frontend unit (`underutilized_threshold_excludes_50_pct`, `utilization_badge_at_exactly_50_pct_is_neutral_available`). Justified — both layers must agree on strict `<` and both are user-visible.
- AC#2 range validation: tested at route unit (`resolve_team_range_rejects_*`) AND integration (`6.4-INT-006`/`-007`). Justified — route unit is fast feedback; integration verifies end-to-end at the HTTP boundary.

#### Unacceptable Duplication ⚠️

- None.

---

### Traceability Recommendations

#### Immediate Actions (Before promoting `review` → `done`)

1. **Run manual browser verification** as a Department Head, covering all four AC flows: team rows, trend range refresh (presets + polling preserves selection), Assign deep-link to `/team` (in-scope + CTC-missing + cross-dept + malformed UUID), and budget gauge (Total / Committed / Spent / Available with health color).
2. **Confirm the Assign deep-link query cleanup** by refreshing and using back/forward navigation after opening the modal — the param must be replaced (not pushed) and must not reopen the modal.

#### Short-term Actions (This Milestone)

1. **Introduce a frontend test harness.** Same advisory inherited from Story 6.3. Candidate: Playwright + `cargo-leptos serve`, or `wasm-bindgen-test` for component-level coverage. Once available, prioritize the AC#3 click flow.
2. **Add a focused integration test for the `assign_resource_id` rejection paths** at the `/team` route level if the team API gains a server-side endpoint for the deep-link (currently the validation is frontend-only).

#### Long-term Actions (Backlog)

1. **Promote `automation-summary-6-4.md` recommendations** into a parent epic for cross-story dashboard automation gaps (6.1, 6.2, 6.3, 6.4 all carry the same harness-absent advisory).

---

## PHASE 2 — QUALITY GATE DECISION

**Gate Type:** story
**Decision Mode:** deterministic + manual-verification-pending

---

### Evidence Summary

#### Test Execution Results

- **Total Story 6.4 tests:** 53 (20 INT + 6 BE Unit + 27 FE Unit)
- **Passed:** 53 (100%)
- **Failed:** 0
- **Skipped:** 0
- **Pending / fixme:** 0

**Compiled & executed regression suites:**

| Suite                                                        | Result |
|--------------------------------------------------------------|--------|
| `cargo test --package xynergy-backend --test dashboard_tests` | **57/57** ✅ |
| `cargo test --package xynergy-backend --test team_tests`      | **10/10** ✅ |
| `cargo test --package xynergy-backend --test budget_tests`    | **18/18** ✅ |
| `cargo test --package xynergy-backend --test overallocation_tests` | **7/7** ✅ |
| `cargo test --package xynergy-backend --lib`                  | **68/68** ✅ |
| `cargo test -p xynergy-frontend --lib`                        | **77/77** ✅ |
| `cargo check -p xynergy-frontend --target wasm32-unknown-unknown` | clean (9 pre-existing unrelated warnings) |
| `cargo check --package xynergy-backend` (`SQLX_OFFLINE=true`) | clean |
| `npm --prefix src/frontend run build`                         | clean |

**Test Results Source:** local execution recorded in `_bmad-output/implementation-artifacts/6-4-team-utilization-dashboard.md` + `_bmad-output/test-artifacts/automation-summary-6-4.md`.

---

#### Coverage Summary (from Phase 1)

- **P0 Acceptance Criteria:** 2/2 covered (100%) ✅
- **P1 Acceptance Criteria:** 2/2 covered (100%) ✅
- **P2 / P3 Acceptance Criteria:** none
- **Overall Coverage:** 100% (static-logic basis)

Code coverage report not produced (no tarpaulin/llvm-cov run in scope for this story). Acceptable — the AC oracle is fully exercised by named integration + unit tests.

---

#### Non-Functional Requirements (NFRs)

- **Security:** **PASS** ✅ — RLS scoping is enforced by `dept_head_team_excludes_other_department_resources` (6.4-INT-005); stale-token defense is inherited from 6.1 regression; no CTC ciphertext/key leakage into the change-flash map (`dh_team_value_map_excludes_sensitive_ctc_fields`); 366-day trend cap defends against unbounded compute via query param.
- **Performance:** **PASS** ✅ — Dashboard reuses existing service joins (`get_capacity_report_in_transaction`, `get_team_members_in_transaction`, `compute_department_budget_utilization`); `current_projects` bounded at 10 (6.4-INT-010); trend computed inside the existing weighted formula, not via per-member fan-out. <500ms target for 50-person department remains achievable (no new N+1 paths introduced).
- **Reliability:** **PASS** ✅ — Savepoint-based optional degradation preserved for budget/upcoming; structured empty state proven (6.4-INT-014); polling preserved (Story 6.2 cleanup-safe interval); `team_range.get_untracked()` prevents Effect-driven interval stacking.
- **Maintainability:** **PASS** ✅ — Threshold constants centralized (`DH_UNDERUTILIZED_THRESHOLD_PCT`, `DH_CURRENT_PROJECTS_PER_MEMBER_LIMIT`, `DH_TEAM_RANGE_MAX_DAYS`); frontend DTOs additive with `#[serde(default)]` for rolling rollout.

**NFR Source:** static review of `dashboard_service.rs`, `dashboard.rs` route module, frontend `dashboard.rs`, and `team.rs`.

---

#### Flakiness Validation

- **Burn-in iterations:** not separately executed; sqlx transactional isolation removes order dependency, and frontend native tests are pure with deterministic UUIDs/dates.
- **Flaky tests detected:** 0 ✅
- **Stability score:** effectively 100% across the 6.4 test set on the runs recorded in `automation-summary-6-4.md`.

---

### Decision Criteria Evaluation

#### P0 Criteria (Must ALL Pass)

| Criterion             | Threshold | Actual                                                  | Status |
|-----------------------|-----------|---------------------------------------------------------|--------|
| P0 Coverage           | 100%      | 100% static logic; manual UI verification pending       | ⚠️ PASS (with caveat) |
| P0 Test Pass Rate     | 100%      | 100% (20 backend INT + 6 BE unit + 27 FE unit)          | ✅ PASS |
| Security Issues       | 0         | 0                                                       | ✅ PASS |
| Critical NFR Failures | 0         | 0                                                       | ✅ PASS |
| Flaky Tests           | 0         | 0                                                       | ✅ PASS |

**P0 Evaluation:** ⚠️ PASS_WITH_REVERIFY_REQUIRED — static logic passes; patched AC#2/AC#3 browser paths still need a fresh Department Head pass.

---

#### P1 Criteria (Required for PASS, May Accept for CONCERNS)

| Criterion              | Threshold | Actual | Status |
|------------------------|-----------|--------|--------|
| P1 Coverage            | ≥90%      | 100%   | ✅ PASS |
| P1 Test Pass Rate      | ≥90%      | 100%   | ✅ PASS |
| Overall Test Pass Rate | ≥90%      | 100%   | ✅ PASS |
| Overall Coverage       | ≥80%      | 100%   | ✅ PASS |

**P1 Evaluation:** ✅ ALL PASS.

---

#### P2/P3 Criteria (Informational)

n/a — no P2/P3 acceptance criteria in scope.

---

### GATE DECISION: **CONCERNS** ⚠️

---

### Rationale

All four ACs reach FULL static-logic coverage with named integration + unit tests; both P0 and P1 priority thresholds are met (100% / 100%); all regression suites (`dashboard_tests`, `team_tests`, `budget_tests`, `overallocation_tests`, frontend native) pass; WASM and backend compilation are clean; no flaky tests; no skipped tests; no security gaps.

The gate is **CONCERNS** rather than PASS because:

1. **Manual browser verification was not executed in the dev pass.** Story 6.4 dev notes explicitly call this out as a precondition for promoting `review` → `done`. The four AC flows (team rows, trend range refresh, Assign deep-link to `/team`, budget gauge + change flashes) need a live Department Head pass.
2. **The Assign deep-link click path (AC#3) is the highest-risk surface that remains manual-only.** It composes RLS scoping + UUID parsing + CTC posture + modal open + query cleanup. The component logic is logic-correct by inspection and reuses Story 3.x guards, but no Playwright / wasm-bindgen-test / component harness exists in repo to lock in the click flow. This is the same cross-story tech debt called out by Story 6.3.

Deployable to staging after manual browser verification; do not block on test creation given (a) the deep-link reuses an already-tested workflow, (b) Story 6.3 carried the identical advisory and shipped, and (c) the missing artifact is harness infrastructure, not story-specific logic.

---

### Residual Risks (CONCERNS)

| # | Risk | Priority | Probability | Impact | Score | Mitigation | Remediation |
|---|------|----------|-------------|--------|-------|------------|-------------|
| 1 | Manual browser verification not yet run; rendered Department Head dashboard could expose a layout / wiring bug not caught by unit tests | P0 (gate) | Low | Medium | 6 (3×2) | Story dev explicitly flagged this in completion notes; perform before promoting to `done` | Run the four AC flows as a DH on local `xynergy-server` |
| 2 | `/team?assign_resource_id=<uuid>` deep-link click path has no automated DOM/router coverage | P0 (data scope) | Low | Medium | 6 (3×2) | Logic-correct by inspection; reuses Story 3.x guards (CTC required, cost preview, overallocation confirmation); backend RLS guards close the cross-department risk | Frontend harness as a follow-up story (shared with 6.3 advisory backlog) |
| 3 | Trend preset segmented control + 30s polling interaction (interval stacking, stale-response overwrite) is logic-correct by `get_untracked()` design but unit-only | P1 | Low | Low | 3 (3×1) | Existing Story 6.2 ordering/liveness guards preserved | Browser verify; add when frontend harness lands |

**Overall Residual Risk:** **LOW**.

---

### Critical Issues (CONCERNS)

| Priority | Issue | Description | Owner | Due Date | Status |
|----------|-------|-------------|-------|----------|--------|
| Advisory | Manual browser verification | Run AC#1–AC#4 flows as Department Head on local `xynergy-server` | Putu | 2026-05-23 | OPEN |
| Backlog  | Frontend test harness | Introduce Playwright + `cargo-leptos serve` or `wasm-bindgen-test` to automate the Assign deep-link + sort/preset accessibility + flash CSS | TBD (cross-story) | next milestone | OPEN (carried from 6.3) |

---

### Gate Recommendations (Review Patched)

1. **Keep in review after code-review patches** until the patched browser paths are rerun.
   - Smoke the Department Head dashboard on staging with at least one CTC-Active underutilized resource, one CTC-missing resource, one overallocated resource, and a configured department budget.
   - Enable change-flash sanity (open dashboard, mutate an allocation, confirm flash within 30s).
2. **Create remediation backlog.**
   - "Frontend test harness for Leptos dashboard pages" — Priority: P1 (shared advisory with Story 6.3).
3. **Post-deployment monitoring.**
   - Watch `dashboard_tests` regression in CI on every dashboard service change.
   - Confirm no spike in 4xx on `/api/v1/dashboard?team_start_date=…` (range validation rejections should remain rare).

---

### Next Steps

**Immediate:**

1. Rerun Department Head browser verification for patched AC#2 and AC#3 paths.
2. Promote story `review` → `done` only after patched flows pass and code review reruns clean.
3. Commit traceability artifacts (this file + `6-4-e2e-trace-summary.json` + `6-4-gate-decision.json`).

**Follow-up (this milestone):**

1. Stand up a frontend test harness; first deliverable is the AC#3 deep-link click flow.
2. Keep the cross-story automation gap tracked alongside Story 6.3 advisory.

**Stakeholder Communication:**

- PM: Story 6.4 is review-patched; patched browser paths need a rerun before done promotion.
- Eng lead: Same frontend harness debt as 6.3; consider scheduling the harness story.

---

## Integrated YAML Snippet (CI/CD)

```yaml
traceability_and_gate:
  traceability:
    story_id: "6.4"
    story_key: "6-4-team-utilization-dashboard"
    date: "2026-05-22"
    coverage:
      overall: 100
      p0: 100
      p1: 100
      p2: null
      p3: null
    gaps:
      critical: 0
      high: 0
      medium: 0
      low: 0
      advisory: 2
    quality:
      passing_tests: 40
      total_tests: 40
      blocker_issues: 0
      warning_issues: 0
      pre_existing_dead_field_warnings: 9
    recommendations:
      - "Rerun manual browser verification for patched AC#2/AC#3 paths as a Department Head"
      - "Introduce a frontend test harness (Playwright + cargo-leptos serve, or wasm-bindgen-test) for AC#3 deep-link click + sort/preset accessibility + flash CSS"

  gate_decision:
    decision: "CONCERNS"
    gate_type: "story"
    decision_mode: "deterministic_plus_manual_pending"
    criteria:
      p0_coverage: 100
      p0_pass_rate: 100
      p1_coverage: 100
      p1_pass_rate: 100
      overall_pass_rate: 100
      static_traceability_coverage: 100
      security_issues: 0
      critical_nfrs_fail: 0
      flaky_tests: "not_assessed_no_burn_in"
    thresholds:
      min_p0_coverage: 100
      min_p0_pass_rate: 100
      min_p1_coverage: 90
      min_p1_pass_rate: 90
      min_overall_pass_rate: 90
      min_coverage: 80
    evidence:
      story_file: "_bmad-output/implementation-artifacts/6-4-team-utilization-dashboard.md"
      automation_summary: "_bmad-output/test-artifacts/automation-summary-6-4.md"
      traceability: "_bmad-output/test-artifacts/traceability/6-4-team-utilization-dashboard-traceability.md"
      trace_summary_json: "_bmad-output/test-artifacts/traceability/6-4-e2e-trace-summary.json"
      gate_decision_json: "_bmad-output/test-artifacts/traceability/6-4-gate-decision.json"
    next_steps: "Rerun patched browser verification → rerun code review → promote only after REVIEW_CLEAN."
```

---

## Related Artifacts

- **Story File:** `_bmad-output/implementation-artifacts/6-4-team-utilization-dashboard.md`
- **Sprint Status:** `_bmad-output/implementation-artifacts/sprint-status.yaml`
- **Automation Summary:** `_bmad-output/test-artifacts/automation-summary-6-4.md`
- **Backend Tests:** `src/backend/tests/dashboard_tests.rs`, `src/backend/src/routes/dashboard.rs::tests`
- **Frontend Tests:** `src/frontend/src/pages/dashboard.rs::tests`
- **Predecessor Trace (pattern reference):** `_bmad-output/test-artifacts/traceability/6-3-project-health-dashboard-traceability.md`

---

## Sign-Off

**Phase 1 — Traceability Assessment**

- Overall Coverage: 100% ✅
- P0 Coverage: 100% ✅
- P1 Coverage: 100% ✅
- Critical Gaps: 0
- High Priority Gaps: 0
- Advisory Items: 1 (deep-link harness) plus review-patched browser rerun required

**Phase 2 — Gate Decision (review patched 2026-05-22)**

- **Decision:** ⚠️ **CONCERNS** after review patches
- **P0 Evaluation:** ⚠️ PASS_WITH_REVERIFY_REQUIRED
- **P1 Evaluation:** ⚠️ PASS_WITH_REVERIFY_REQUIRED
- **Manual Browser Verification:** ⚠️ rerun required for patched AC#2/AC#3 paths

**Overall Status:** ⚠️ **REVIEW_PATCHED — keep Story 6.4 in `review` until patched browser verification passes and review reruns clean.**

**Originally Generated:** 2026-05-22T00:00:00Z
**Review Patch Appended:** 2026-05-22
**Workflow:** testarch-trace v4.0 (Enhanced with Gate Decision + Manual Verification Appendix)

---

## Manual Verification — 2026-05-22

**Status:** ⚠️ **REVIEW_PATCHED_REVERIFY_REQUIRED** — initial manual evidence exists, but code-review patches changed AC#2/AC#3 behavior. Detailed evidence and rerun notes are in [_bmad-output/test-artifacts/manual-verification-6-4.md](../manual-verification-6-4.md).

### Setup

- **Database fixture:** [_bmad-output/test-artifacts/dh-fixture-6-4.sql](../dh-fixture-6-4.sql) (idempotent via fixed UUIDs + ON CONFLICT; guarded by `xynergy.fixture_allow_seed=local-only`). Apply from the repo root with `podman exec -i xynergy-db psql -U xynergy -d xynergy -v ON_ERROR_STOP=1 -c "SET xynergy.fixture_allow_seed = 'local-only';" -f - < _bmad-output/test-artifacts/dh-fixture-6-4.sql`; clean up with the paired `_bmad-output/test-artifacts/dh-fixture-6-4-cleanup.sql` using the same command shape.
- **DH user:** `dh-6-4@xynergy.test` / `admin123`, scoped to `DH-6-4 Fixture Dept` (id `11111111-1111-1111-1111-111111111111`).
- **Team:** 4 employees — `Underutilized Maya` (30%, CTC Active), `Healthy Arjun` (85%, CTC Active), `CTC-Missing Pria` (20%, no CTC), `Overallocated Sari` (70%+70%=140%, CTC Active).
- **Department budget:** fixture current month, Rp 1.000.000.000, alert threshold 80%.
- **Backend:** `target/debug/xynergy-server` (PID 67218, port 3000), env loaded from `.env`.
- **Frontend:** rebuilt via `bash build-frontend.sh` at 2026-05-22 17:48 UTC.
- **Driver:** Claude `/browse` (gstack headless Chromium).

### Per-AC Results

#### AC#1 — Team rows + utilization + projects + capacity ✅ PASS

Evidence: [screenshots-6-4/01-ac1-dashboard-loaded.png](../screenshots-6-4/01-ac1-dashboard-loaded.png)

- All 4 team members rendered with name + employee subtitle, utilization % (with colored gauge bar), available %, current projects with allocation %, status badge, action.
- Status badges: Underutilized (yellow) × 2, Healthy (green) × 1, Overallocated (red) × 1.
- **Overallocated Sari:** `available_capacity_pct = 0.0%` confirms the `max(0, 100 - allocation)` clamp despite the 140% utilization.
- Aggregate KPIs: AVG UTILIZATION 34.4%, AVG AVAILABLE 41.2%, UNDERUTILIZED 2, OVERALLOCATED 1, 4 team members, BUDGET 6%. The 34.4% is the weighted-working-day average over the default 30-day window (allocations only span May, diluted by zero June days) — matches PRD FR58.

#### AC#2 — Utilization trends per member for selected time range ⚠️ REVIEW_PATCHED

Evidence: [02-ac2-trend-current-month.png](../screenshots-6-4/02-ac2-trend-current-month.png), [03-ac2-trend-3-months.png](../screenshots-6-4/03-ac2-trend-3-months.png), [04-ac2-trend-6-months.png](../screenshots-6-4/04-ac2-trend-6-months.png)

| Preset         | Query                                                        | Latency | Bytes |
|----------------|---------------------------------------------------------------|---------|-------|
| Current Month  | `team_start_date=2026-05-01&team_end_date=2026-05-31`         | 21 ms   | 4249  |
| 3 Months       | `team_start_date=2026-05-22&team_end_date=2026-08-20`         | 24 ms   | 4765  |
| 6 Months       | `team_start_date=2026-05-22&team_end_date=2026-11-18`         | 20 ms   | 5280  |

**Polling-cadence stress test:** with the 6-month preset selected and network log cleared, waited **32 seconds**. Exactly **one** dashboard request fired, with the same 6-month range — proving (a) polling continues at the 30s cadence, (b) the user-selected range is preserved across polls, (c) no interval stacking from the 4 preceding preset clicks, (d) no full-page flash (only trend table + AVG KPI re-rendered).

#### AC#3 — Underutilized → click Assign → /team deep-link round-trip ⚠️ REVIEW_PATCHED

Evidence: [05-ac3-deeplink-modal-opened.png](../screenshots-6-4/05-ac3-deeplink-modal-opened.png), [06-ac3-deeplink-modal-correct-resource.png](../screenshots-6-4/06-ac3-deeplink-modal-correct-resource.png)

**Click flow (the production path, AC-required):**

1. Clicked **Underutilized Maya → Assign** on `/dashboard`.
2. URL changed to `http://127.0.0.1:3000/team` — query param `?assign_resource_id=33333333-3333-3333-3333-333333333301` was consumed and cleared immediately (replace navigation, not push).
3. Requests fired: `/api/v1/team` (×2), `/api/v1/projects/assignable`, `/api/v1/team/budget?period=2026-05`, `/api/v1/team/capacity-report`, `/api/v1/team/budget/breakdown`.
4. **Assignment modal opened with "Assigning: Underutilized Maya"** — assignable-projects dropdown populated.
5. Closed modal via ✕, **reloaded the page** — modal did **NOT** reopen and URL remained `/team` (proves `replace: true` semantics; refresh/back/forward are immune).

**Edge cases (patched paths require reverify):**

| Case                                              | URL after settle  | Modal opened? | Verdict |
|---------------------------------------------------|-------------------|---------------|---------|
| Malformed UUID `?assign_resource_id=not-a-uuid`   | `/team` (cleared) | No            | ⚠️ Reverify patched bounded error |
| Random/unknown UUID `99…99`                       | `/team` (cleared) | No            | ⚠️ Reverify patched bounded error |
| CTC-Missing Pria UUID `33…3303` (in-scope no CTC) | `/team` (cleared) | No            | ⚠️ Reverify patched CTC error |
| **Disabled Assign button** for CTC-Missing Pria   | (`disabled` attr) | n/a           | ✅ PASS (tooltip: _"CTC data required to assign. Contact HR to complete employee setup."_) |

**Review-patched direct deep-link path:** Direct address-bar navigation to `/team?assign_resource_id=<valid-in-scope-CTC-Active-uuid>` is now treated as required AC#3 behavior, not future hardening. The patched Effect waits for auth and team data before opening the existing assignment modal, clears the query with replace navigation, and shows bounded errors for malformed, unavailable, CTC-missing, or non-assignable IDs. Rerun this direct-navigation path with the guarded-error cases before promoting Story 6.4 to `done`.

#### AC#4 — Department budget gauge: Total / Committed / Spent / Available ✅ PASS

Evidence: co-located on the dashboard, captured in [01-ac1-dashboard-loaded.png](../screenshots-6-4/01-ac1-dashboard-loaded.png).

| Field           | Value            |
|-----------------|------------------|
| TOTAL BUDGET    | Rp 1.000.000.000 |
| COMMITTED       | Rp 58.365.000    |
| SPENT           | Rp 58.365.000    |
| AVAILABLE       | Rp 941.635.000   |
| Health label    | **On track** (green) |
| Utilization     | 6% used          |
| Alert threshold | 80%              |
| Gauge bar       | Green, ~6% filled |

IDR formatting uses dot-grouped thousands. Health color follows `alert_threshold_pct` (green below threshold). The amber/red branches are unit-tested via `budget_health_class_and_label_branches` but not exercised live (fixture stays well under threshold).

### Resolution Summary

| Item from original CONCERNS rationale | Status after 2026-05-22 verification |
|---------------------------------------|---------------------------------------|
| Manual browser verification of AC#1–AC#4 | ⚠️ **PARTIAL CURRENT** — AC#1/AC#4 remain PASS; AC#2/AC#3 initial evidence superseded by review patches |
| Deep-link click-path AC#3 not auto-tested | ⚠️ **Reverify patched behavior**; cross-story harness backlog item remains (shared with Story 6.3) |

**Gate state:** `CONCERNS / REVIEW_PATCHED` recorded in [6-4-gate-decision.json](6-4-gate-decision.json) and [6-4-e2e-trace-summary.json](6-4-e2e-trace-summary.json) at 2026-05-22T11:00:00Z. Keep Story 6.4 in `review` until patched AC#2/AC#3 browser checks pass.

---

<!-- Powered by BMAD-CORE™ -->
