---
stepsCompleted:
  - step-01-load-context
  - step-02-discover-tests
  - step-03-map-criteria
  - step-04-analyze-gaps
  - step-05-gate-decision
lastStep: step-05-gate-decision
lastSaved: '2026-05-22'
storyId: '6.3'
storyKey: 6-3-project-health-dashboard
storyTitle: 'Project Health Dashboard'
coverageBasis: acceptance_criteria
oracleConfidence: high
oracleResolutionMode: formal_requirements
oracleSources:
  - _bmad-output/implementation-artifacts/6-3-project-health-dashboard.md
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - _bmad-output/test-artifacts/automation-summary.md
externalPointerStatus: not_used
tempCoverageMatrixPath: not_committed
gateDecision: PASS_WITH_ADVISORY
gateEligible: true
collectionStatus: COLLECTED
executionMode: sequential
---

# Traceability Report — Story 6.3 (Project Health Dashboard)

## Gate Decision: **PASS_WITH_ADVISORY**

**Rationale:** PASS_WITH_ADVISORY after follow-up verification. The static mapping shows strong logic coverage for all four acceptance criteria, live backend execution is green, forecast/P&L regression execution is green, and manual browser verification completed against local `xynergy-server`. The current PM dashboard contract has 10 Story 6.3 backend integration tests mapped to every documented budget/health branch, plus 24 Story 6.3 frontend native unit tests for sort, visual-state, and change-detection helpers. Regression coverage adds 27 Story 6.1 backend INT tests, 27 Story 6.2 frontend UNIT tests, 11 forecast regression tests, and 10 P&L regression tests, for 109 tests considered across Story 6.3 and relevant regressions.

Decision is **PASS_WITH_ADVISORY** because required Story 6.3 verification is green while repeatable browser/component automation remains future tech debt. Frontend native suite is **51/51 PASS**, frontend WASM compile is **clean**, backend `dashboard_tests` passed **37/37** live, `project_pl_forecast_tests` passed **11/11**, `project_pl_tests` passed **10/10**, and manual browser verification covered the PM cards, warning label, sort controls, P&L deep-link, close-query cleanup, manual Refresh, and polling flash.

**Remaining advisory caveats:**

1. **No repeatable browser harness.** PM card render, warning label, sort controls, P&L deep-link, polling flash, and manual Refresh were manually verified, but no Playwright/Cypress/wasm-bindgen-test harness exists in repo.
2. **PM card P&L deep-link click path not automated.** `/projects?view=pnl&project_id=<uuid>` query parsing and runtime click flow were browser-verified, but remain manual until a Leptos router/browser harness exists.
3. **Sort-control accessibility not automated.** The `sort_pm_cards` helper and `pm_utilization_score` invariants are unit-covered, and rendered `aria-pressed` selected state was manually verified, but repeatable keyboard/focus automation is absent.
4. **Forecast-service failure degradation path not deterministically exercised.** Forecast failure is handled inline with zeroed forecast fields plus a bounded per-card warning; the separate `safe_unavailable_card` path covers P&L failure and has no test seam injecting that service error.
5. **Carry-over from Stories 6.1/6.2.** No Leptos component test runner or `wasm-bindgen-test` harness exists in repo. Story 6.3 Architecture Compliance forbids introducing new dependencies for this work. Tracked as cross-story tech debt.

---

## Coverage Oracle

- **Resolution mode:** `formal_requirements` (story acceptance criteria + PRD FR/NFR + UX spec + automation summary).
- **Confidence:** `high` — four independent formal sources (story AC #1–#4, PRD FR48/FR50/FR51/NFR2/NFR7/NFR21-28, UX Stripe Financial direction, automation expansion summary) agree on the same testable contract (PM card fields + budget severity + margin/forecast colouring + sort modes + P&L deep-link reuse).
- **Sources:**
  - `_bmad-output/implementation-artifacts/6-3-project-health-dashboard.md` — 4 ACs, 8 task groups (status: `done`)
  - `_bmad-output/planning-artifacts/epics.md` — Epic 6 / Story 6.3 AC bullets
  - `_bmad-output/planning-artifacts/prd.md` — FR48 (PM dashboard), FR50 (polling), FR51 (manual refresh), NFR2/NFR7/NFR21-28
  - `_bmad-output/planning-artifacts/ux-design-specification.md` — Stripe Financial direction; clear financial cards + drill-downs
  - `_bmad-output/test-artifacts/automation-summary.md` — Run 2026-05-22 automation inventory (10 Story 6.3 backend INT + 24 Story 6.3 frontend UNIT; 109 tests considered with regressions after live verification)
- **External pointer status:** `not_used`
- **Synthetic oracle:** no

## Acceptance Criteria (formal)

| AC | Description | Priority |
|----|-------------|----------|
| AC#1 | Given I navigate to Projects Dashboard, when the page loads, then I see project cards with Name, Budget Status (green/yellow/red), Current Margin, Forecast Margin. | P0 — primary card contract |
| AC#2 | Given I view the project list, when a project exceeds budget, then its status indicator shows red and a warning icon appears. | P0 — over-budget visibility |
| AC#3 | Given I view the project list, when a project's margin is below target, then the margin is displayed in orange/red and I can click for detailed P&L view. | P1 — margin signal + drill-down |
| AC#4 | Given I have multiple projects, when I view the dashboard, then I can sort by Margin, Budget Utilization, End Date. | P1 — operational ordering |

## Story Scope (in/out)

- **In scope (testable):** PM card contract extension (forecast/over-budget/health-status fields), `derive_health_status` and `project_budget_status` branches (healthy / warning / critical / unconfigured), `budget_badge_class`/`budget_badge_label`/`margin_status_class`/`forecast_status_class` visual mapping, `sort_pm_cards`/`pm_utilization_score` invariants, `dashboard_value_map` stable keys for every new visible PM value, `/projects?view=pnl&project_id=<uuid>` query parameter wiring, preservation of Story 6.2 polling / change-flash behavior, PM scope unchanged (owned + active + non-ended).
- **Out of scope (deliberate per story):** new dashboard endpoint families, new charting libraries, SSE/WebSockets, Story 6.4 team utilization, Story 6.5 CTC completeness, executive BI, new P&L formulas, budget mutation workflows, revenue entry changes, replacing the Story 6.1/6.2 polling implementation.

## Knowledge Base Loaded

- `test-priorities-matrix.md` — P0/P1/P2/P3 criteria
- `risk-governance.md` — gate scoring + decision rules
- `probability-impact.md` — risk scoring scale
- `test-quality.md` — DoD (deterministic, isolated, single-invariant)
- `selective-testing.md` — risk-based selection

## Test Inventory (preview — expanded in Step 2)

| Category | File | Count | Notes |
|----------|------|-------|-------|
| Backend integration (Story 6.3) | `src/backend/tests/dashboard_tests.rs` | 10 | 6.3-INT-001..010; `#[sqlx::test(migrations = "../../migrations")]` |
| Backend integration (Story 6.1 regression) | `src/backend/tests/dashboard_tests.rs` | 27 | Inherited contract that Story 6.3 must not regress |
| Frontend unit (Story 6.3 native, non-WASM) | `src/frontend/src/pages/dashboard.rs::tests` | 24 | 6.3-UNIT-028..051; sort, change-detection keys, visual-state helpers |
| Frontend unit (Story 6.2 regression) | `src/frontend/src/pages/dashboard.rs::tests` | 27 | Inherited change-detection coverage Story 6.3 must not regress |
| Frontend WASM compile contract | `cargo check -p xynergy-frontend --target wasm32-unknown-unknown` | n/a | Type-level contract; no test count |
| Tailwind rebuild | `npm --prefix src/frontend run build` | n/a | Output regenerated for new utility classes used by 6.3 cards |
| E2E / browser automation | none | 0 | No Playwright/Cypress harness in repo (carry-over from 6.1/6.2) |
| Manual browser verification | local server + browser | **completed** | Story 6.3 Task 8 completed against local `xynergy-server` |

---

## Step 2 — Test Discovery & Catalog

### 2.1 Backend Integration Tests — `src/backend/tests/dashboard_tests.rs`

All `#[sqlx::test(migrations = "../../migrations")]` async fns. Story 6.3 currently has 10 backend integration tests; the 27 Story 6.1 tests continue to provide regression coverage for PM scope, role isolation, auth boundaries, and `generated_at` freshness that Story 6.3 must not regress.

#### Story 6.3 INT suite (10)

| ID | Title | File | Line | Level | Priority | State |
|----|-------|------|------|-------|----------|-------|
| 6.3-INT-001 | `project_manager_card_includes_forecast_and_overrun_fields` | `src/backend/tests/dashboard_tests.rs` | 1444 | api | P1 | active |
| 6.3-INT-002 | `project_manager_card_marks_over_budget_critical` | `src/backend/tests/dashboard_tests.rs` | 1501 | api | P0 | active |
| 6.3-INT-003 | `project_manager_card_surfaces_margin_alert_when_below_target` | `src/backend/tests/dashboard_tests.rs` | 1550 | api | P0 | active |
| 6.3-INT-004 | `project_manager_card_forecast_below_target_drives_critical_health` | `src/backend/tests/dashboard_tests.rs` | 1593 | api | P1 | active |
| 6.3-INT-005 | `project_manager_active_projects_remain_bounded_after_forecast_wiring` | `src/backend/tests/dashboard_tests.rs` | 1658 | api | P1 | active |
| 6.3-INT-006 | `project_manager_card_scope_unchanged_by_story_6_3` | `src/backend/tests/dashboard_tests.rs` | 1685 | api | P0 | active |
| 6.3-INT-007 | `project_manager_card_unconfigured_budget_status_when_no_budget` | `src/backend/tests/dashboard_tests.rs` | 1754 | api | P1 | active |
| 6.3-INT-008 | `project_manager_card_warning_budget_status_between_50_and_80_pct` | `src/backend/tests/dashboard_tests.rs` | 1802 | api | P1 | active |
| 6.3-INT-009 | `project_manager_card_healthy_budget_status_below_50_pct` | `src/backend/tests/dashboard_tests.rs` | 1851 | api | P2 | active |
| 6.3-INT-010 | `project_manager_card_health_status_unconfigured_when_no_budget_and_no_alert` | `src/backend/tests/dashboard_tests.rs` | 1900 | api | P1 | active |

**Subtotal:** 10 active, 0 skipped/fixme. Last run: live execution passed (`DATABASE_URL=postgres://xynergy:xynergy@localhost:5432/xynergy cargo test -p xynergy-backend --test dashboard_tests` → 37/37 passed across Story 6.3 + inherited dashboard regression tests).

#### Story 6.1 INT regression suite (27)

All 27 tests (6.1-INT-001..027) re-run as regression for Story 6.3 — see `6-2-real-time-dashboard-updates-traceability.md` §2.2 for the full table. Key tests relevant to Story 6.3 PM scope:

- 6.1-INT-006 `project_manager_only_sees_owned_projects`
- 6.1-INT-014 `project_manager_with_no_projects_returns_empty_state`
- 6.1-INT-015 `project_manager_excludes_non_active_projects`
- 6.1-INT-018 `dashboard_response_includes_recent_generated_at`
- 6.1-INT-025 `project_manager_active_projects_bounded_to_ten`

### 2.2 Frontend Unit Tests — `src/frontend/src/pages/dashboard.rs::tests`

Native (non-WASM) `#[test]` functions inside `#[cfg(all(test, not(target_arch = "wasm32")))]`. Pure functions over `RoleDashboardResponse`, sort helpers, visual-state classifiers, and change-detection map. Story 6.3 has 24 frontend unit tests; 17 came from the automation expansion and the rest cover earlier Story 6.3 helper/key contracts. Story 6.2 has 27 retained regression tests.

#### Story 6.3 UNIT suite (24)

| ID | Title | File | Line | Level | Priority | State |
|----|-------|------|------|-------|----------|-------|
| 6.3-UNIT-028 | `pm_sort_by_margin_orders_lowest_first` | `src/frontend/src/pages/dashboard.rs` | 3274 | unit | P1 | active |
| 6.3-UNIT-029 | `pm_sort_by_budget_utilization_orders_over_budget_first` | `src/frontend/src/pages/dashboard.rs` | 3299 | unit | P1 | active |
| 6.3-UNIT-030 | `pm_sort_by_end_date_orders_soonest_first` | `src/frontend/src/pages/dashboard.rs` | 3340 | unit | P1 | active |
| 6.3-UNIT-031 | `pm_sort_uses_project_name_as_tiebreaker` | `src/frontend/src/pages/dashboard.rs` | 3365 | unit | P2 | active |
| 6.3-UNIT-032 | `pm_forecast_margin_change_produces_expected_key` | `src/frontend/src/pages/dashboard.rs` | 3388 | unit | P0 | active |
| 6.3-UNIT-033 | `pm_over_budget_transition_produces_expected_key` | `src/frontend/src/pages/dashboard.rs` | 3415 | unit | P0 | active |
| 6.3-UNIT-034 | `pm_value_map_excludes_sensitive_keys_for_pm_section` | `src/frontend/src/pages/dashboard.rs` | 3460 | unit | P0 | active |
| 6.3-UNIT-035 | `budget_badge_class_returns_badge_negative_when_over_budget` | `src/frontend/src/pages/dashboard.rs` | 3497 | unit | P1 | active |
| 6.3-UNIT-036 | `budget_badge_class_maps_budget_status_tokens` | `src/frontend/src/pages/dashboard.rs` | 3509 | unit | P1 | active |
| 6.3-UNIT-037 | `budget_badge_label_returns_human_readable_labels` | `src/frontend/src/pages/dashboard.rs` | 3520 | unit | P2 | active |
| 6.3-UNIT-038 | `margin_status_class_negative_when_margin_alert_present` | `src/frontend/src/pages/dashboard.rs` | 3530 | unit | P1 | active |
| 6.3-UNIT-039 | `margin_status_class_negative_when_below_target_beyond_threshold` | `src/frontend/src/pages/dashboard.rs` | 3537 | unit | P1 | active |
| 6.3-UNIT-040 | `margin_status_class_warning_when_below_target_within_threshold` | `src/frontend/src/pages/dashboard.rs` | 3544 | unit | P1 | active |
| 6.3-UNIT-041 | `margin_status_class_positive_when_at_or_above_target` | `src/frontend/src/pages/dashboard.rs` | 3551 | unit | P2 | active |
| 6.3-UNIT-042 | `forecast_status_class_muted_when_unavailable` | `src/frontend/src/pages/dashboard.rs` | 3563 | unit | P2 | active |
| 6.3-UNIT-043 | `forecast_status_class_negative_when_variance_below_threshold` | `src/frontend/src/pages/dashboard.rs` | 3571 | unit | P1 | active |
| 6.3-UNIT-044 | `forecast_status_class_warning_when_variance_below_target_within_threshold` | `src/frontend/src/pages/dashboard.rs` | 3578 | unit | P1 | active |
| 6.3-UNIT-045 | `forecast_status_class_positive_when_variance_at_or_above_target` | `src/frontend/src/pages/dashboard.rs` | 3585 | unit | P2 | active |
| 6.3-UNIT-046 | `pm_utilization_score_returns_sentinel_for_unconfigured_card` | `src/frontend/src/pages/dashboard.rs` | 3597 | unit | P1 | active |
| 6.3-UNIT-047 | `pm_utilization_score_boosts_over_budget_card_above_in_budget_card_at_same_utilization` | `src/frontend/src/pages/dashboard.rs` | 3611 | unit | P1 | active |
| 6.3-UNIT-048 | `pm_health_status_change_produces_stable_key` | `src/frontend/src/pages/dashboard.rs` | 3636 | unit | P1 | active |
| 6.3-UNIT-049 | `pm_projected_total_cost_idr_change_produces_stable_key` | `src/frontend/src/pages/dashboard.rs` | 3666 | unit | P1 | active |
| 6.3-UNIT-050 | `pm_forecast_variance_from_target_pct_change_produces_stable_key` | `src/frontend/src/pages/dashboard.rs` | 3696 | unit | P1 | active |
| 6.3-UNIT-051 | `sort_pm_cards_handles_empty_slice_without_panic` | `src/frontend/src/pages/dashboard.rs` | 3726 | unit | P2 | active |

**Subtotal:** 24 active, 0 skipped. Last run: 51/51 PASS (`SQLX_OFFLINE=true cargo test -p xynergy-frontend --lib`, per `automation-summary.md` Run 2026-05-22).

#### Story 6.2 UNIT regression suite (27)

All 27 Story 6.2 tests (6.2-UNIT-001..027) remain active and pass — see `6-2-real-time-dashboard-updates-traceability.md` §2.1 for the full table. They guarantee Story 6.3 did not break:

- The change-detection contract (UNIT-001..005, UNIT-013, UNIT-014)
- Display helpers (`format_idr`, `format_date`, `role_display` — UNIT-016..020)
- Per-role stable id-keyed change paths (UNIT-006..012, UNIT-021..027)

### 2.3 WASM Compile Contract

| ID | Check | Command | Last Run |
|----|-------|---------|----------|
| 6.3-COMPILE-001 | Frontend compiles for wasm32-unknown-unknown | `cargo check -p xynergy-frontend --target wasm32-unknown-unknown` | PASS — 9 pre-existing dead-code warnings in `team.rs`, no errors |
| 6.3-COMPILE-002 | Backend dashboard_tests binary compiles offline | `SQLX_OFFLINE=true cargo test -p xynergy-backend --test dashboard_tests --no-run` | PASS — 1 pre-existing future-incompat warning from `sqlx-postgres 0.7.4` |
| 6.3-BUILD-001 | Tailwind CSS regenerates | `npm --prefix src/frontend run build` | PASS — `src/frontend/public/output.css` regenerated (~42ms, no errors) |

### 2.4 Coverage Heuristics Inventory

**API endpoint coverage**
- Story 6.3 reuses `GET /api/v1/dashboard` (no new endpoints). The 10 Story 6.3 INT tests + 27 Story 6.1 INT tests exercise the PM dashboard contract and inherited role-dashboard guardrails.
- P&L detail endpoints (`/api/v1/projects/:id/pl`, `/api/v1/projects/:id/pl/forecast`) are reused unchanged from Story 5.x; their access checks and shape are inherited.

**Authentication / authorization coverage**
- 401 (no token / invalid token), 403 (unsupported role / missing department), stale-token-vs-current-department: all covered at backend via Story 6.1 INT-001/011/012/013/020 — regression-safe for Story 6.3.
- PM scope (project_manager_id ownership + Active status + non-ended end_date + PM_ACTIVE_PROJECT_LIMIT bound): 6.3-INT-005/006 plus 6.1-INT-006/014/015/025.

**Error-path coverage**
- 401/403 paths covered (inherited).
- Forecast-service failure for a single project (inline zeroed forecast fields plus bounded warning): **not deterministically exercised** — no test seam injects an error from `get_project_pl_forecast`. Behaviour relies on the `warning: Option<String>` field staying deserializable.
- Frontend rendering error states on `/dashboard`: not asserted (no Leptos component harness; carry-over).

**Budget threshold branch coverage** (new in Story 6.3)
- `unconfigured` (`total_budget_idr <= 0`): 6.3-INT-007, 6.3-INT-010 (health_status branch).
- `healthy` (< 50%): 6.3-INT-009 (`30%` utilization).
- `warning` (50% – < 80%): 6.3-INT-008 (`65%` utilization).
- `critical` (≥ 80% or `is_over_budget`): 6.3-INT-002 (250% over-budget) + 6.3-INT-001 path.

All four documented branches exercised.

**Health-status derivation coverage** (`derive_health_status`)
- `critical` (over-budget): 6.3-INT-002.
- `critical` (forecast below target despite healthy current spend/margin): 6.3-INT-004.
- `unconfigured` (no budget + no alert): 6.3-INT-010.
- `healthy` / `warning` exercised through the same end-to-end fixtures of INT-001 + INT-009.

**Frontend visual-state helper coverage**
- `budget_badge_class` (severity → Tailwind utility + override on `is_over_budget`): UNIT-035, UNIT-036.
- `budget_badge_label` (human-readable contract): UNIT-037.
- `margin_status_class` (4 branches: alert dominance, below target beyond threshold, below target within threshold, at/above target): UNIT-038, UNIT-039, UNIT-040, UNIT-041.
- `forecast_status_class` (4 branches: unavailable, variance beyond threshold, variance within threshold, at/above target): UNIT-042, UNIT-043, UNIT-044, UNIT-045.

**Sort helper coverage** (`sort_pm_cards` + `pm_utilization_score`)
- All three sort modes (Margin / Budget Utilization / End Date): UNIT-028, UNIT-029, UNIT-030.
- Name tiebreaker (deterministic ordering on ties): UNIT-031.
- Unconfigured sentinel (`-1.0` keeps unconfigured cards last in Budget Utilization mode): UNIT-046.
- Over-budget boost (over-budget card outranks in-budget card at same utilization %): UNIT-047.
- Empty-slice safety (sort toggling on an empty active_projects without panic): UNIT-051.

**Change-detection key coverage** (all Story 6.3 Task 3 keys)
- `forecast_margin_pct`: UNIT-032.
- `is_over_budget` + `budget_overrun_idr` + `health_status` (over-budget transition): UNIT-033.
- `health_status` standalone change: UNIT-048.
- `projected_total_cost_idr`: UNIT-049.
- `forecast_variance_from_target_pct`: UNIT-050.
- Cross-cutting: PM section never includes CTC ciphertext / key metadata / sensitive components: UNIT-034 (paired with backend 6.1-INT-003).

**UI journey coverage**
- Routes touched: `/dashboard` (PM cards, sort), `/projects?view=pnl&project_id=<uuid>` (deep-link entry).
- E2E automation coverage: **none** — no Playwright/Cypress harness. Manual browser verification **completed** for this story.

**UI state coverage**
- PM-card severity rendering: helper-level unit tests cover the mapping; rendered DOM was manually verified.
- Empty PM dashboard: 6.1-INT-014 covers backend empty-state; 6.3-UNIT-051 covers sort safety on empty slice.
- Over-budget icon (warning SVG + accessible label): manually verified with accessible label `Over budget by Rp 8.000.000`.
- Sort-control accessibility (`aria-pressed`, focus styles, segmented selected state): `aria-pressed` selected state manually verified; repeatable keyboard/focus automation remains advisory.

### 2.5 Discovery Summary

- **Total active tests considered for Story 6.3:** 109 (37 backend INT [10 Story 6.3 + 27 regression] + 51 frontend UNIT [24 Story 6.3 + 27 regression] + 11 forecast regression + 10 P&L regression) + 3 compile/build contracts + manual browser pass.
- **Story-scoped tests:** 34 (10 backend INT + 24 frontend UNIT).
- **Distribution by level (story-scoped):** 29% backend integration (api), 71% frontend unit. 0% E2E. 0% component.
- **Distribution by priority (story-scoped):** P0 = 6 (3 backend INT + 3 frontend UNIT — INT-002/003/006, UNIT-032/033/034), P1 = 21, P2 = 7. No P3.
- **Test cases skipped/fixme:** 0. **Verification remaining:** 0 required checks.
- **Frontend WASM-runtime behaviour (sort-button accessibility, deep-link click flow, polling refresh of new PM keys, CSS flash on new values):** not automated; manual browser verification **completed**.

---

## Step 3 — Coverage Mapping Matrix

### 3.1 Acceptance Criteria → Tests

#### AC#1 — PM cards show Name, Budget Status (green/yellow/red), Current Margin, Forecast Margin

| Sub-aspect | Mapped Tests | Level | Coverage |
|------------|--------------|-------|----------|
| Card includes new fields (`budget_utilization_pct`, `is_over_budget`, `budget_overrun_idr`, `projected_total_cost_idr`, `forecast_margin_pct`, `forecast_variance_from_target_pct`, `health_status`) | 6.3-INT-001 (`project_manager_card_includes_forecast_and_overrun_fields`) | api | FULL |
| Budget status `unconfigured` branch | 6.3-INT-007 | api | FULL |
| Budget status `healthy` branch (< 50% utilization) | 6.3-INT-009 | api | FULL |
| Budget status `warning` branch (50% – < 80% utilization) | 6.3-INT-008 | api | FULL |
| Budget status `critical` branch (≥ 80% or over-budget) | 6.3-INT-002 | api | FULL |
| Health-status derivation `unconfigured` (no budget + no alert) | 6.3-INT-010 | api | FULL |
| Forecast-driven health severity | 6.3-INT-004 | api | FULL |
| Severity → Tailwind utility mapping for the badge | 6.3-UNIT-035, 6.3-UNIT-036 | unit | FULL |
| Human-readable budget label contract | 6.3-UNIT-037 | unit | FULL |
| Current and forecast margin values populated and typed | 6.3-INT-001 (`forecast_margin_pct`, `projected_total_cost_idr` presence) | api | FULL |
| Rendered card layout (badge + warning icon + margin columns + View P&L action) | Manual browser verification (Story 6.3 Task 8) | wasm-runtime | **MANUAL VERIFIED** |
| Cards remain bounded after forecast wiring | 6.3-INT-005 (`PM_ACTIVE_PROJECT_LIMIT=10`) | api | FULL |
| Scope unchanged (only owned + active + non-ended) | 6.3-INT-006 + 6.1-INT-006/014/015 | api | FULL |

**Heuristics**
- Endpoint coverage: present (single endpoint, exhaustively tested at the new branches).
- Auth/authz: inherited from Story 6.1 (401/403/stale-token covered).
- Error path: forecast-failure degradation not deterministically tested (advisory caveat #4).
- UI journey: not asserted in this story; deferred to manual.
- UI states: rendered card not asserted; helper-layer fully covered.

**AC#1 verdict:** **COVERED + manual browser verified** — every documented branch of the new card contract has backend integration coverage, every visual-state helper has unit coverage, and browser rendering was manually verified.

#### AC#2 — Over-budget project ⇒ red status indicator + warning icon

| Sub-aspect | Mapped Tests | Level | Coverage |
|------------|--------------|-------|----------|
| `is_over_budget=true` + positive `budget_overrun_idr` + `budget_status='critical'` + `health_status='critical'` on the backend | 6.3-INT-002 (`project_manager_card_marks_over_budget_critical`) | api | FULL |
| Polling tick that flips a card to over-budget surfaces stable change keys (`is_over_budget`, `budget_overrun_idr`, `health_status`) | 6.3-UNIT-033 (`pm_over_budget_transition_produces_expected_key`) | unit | FULL |
| `budget_badge_class` returns `badge-negative` whenever `is_over_budget=true`, regardless of health_status (defensive against coupling drift) | 6.3-UNIT-035 | unit | FULL |
| Badge label flips to `"Over budget"` whenever `is_over_budget=true` | 6.3-UNIT-037 | unit | FULL |
| Warning SVG renders next to the over-budget card with accessible text/`aria-label` | Manual browser verification (Story 6.3 Task 8) | wasm-runtime | **MANUAL VERIFIED** |
| Polling refresh applies `dashboard-change-flash` to changed PM card values | Manual browser verification | css/wasm-runtime | **MANUAL VERIFIED** |

**Heuristics**
- Critical path coverage: full at logic layer (backend + helpers).
- Visual layer: deferred to manual browser verification.

**AC#2 verdict:** **COVERED + manual browser verified** — backend forces all over-budget flags, helper assertions lock the badge contract, change-detection keys surface the transition, and rendered warning label/flash behavior were manually verified.

#### AC#3 — Below-target margin ⇒ orange/red display + clickable P&L deep-link

| Sub-aspect | Mapped Tests | Level | Coverage |
|------------|--------------|-------|----------|
| Below-target current margin surfaces `margin_alert` and joins `margin_alerts` list | 6.3-INT-003 (`project_manager_card_surfaces_margin_alert_when_below_target`) | api | FULL |
| Polling tick on forecast margin emits stable change key | 6.3-UNIT-032 (`pm_forecast_margin_change_produces_expected_key`) | unit | FULL |
| Polling tick on projected total cost emits stable change key | 6.3-UNIT-049 | unit | FULL |
| Polling tick on forecast variance emits stable change key | 6.3-UNIT-050 | unit | FULL |
| Polling tick on health-status emits stable change key | 6.3-UNIT-048 | unit | FULL |
| Current margin colour — alert dominance | 6.3-UNIT-038 | unit | FULL |
| Current margin colour — gap beyond threshold → negative | 6.3-UNIT-039 | unit | FULL |
| Current margin colour — within threshold → warning | 6.3-UNIT-040 | unit | FULL |
| Current margin colour — at/above target → positive | 6.3-UNIT-041 | unit | FULL |
| Forecast margin colour — unavailable → muted | 6.3-UNIT-042 | unit | FULL |
| Forecast margin colour — variance beyond threshold → negative | 6.3-UNIT-043 | unit | FULL |
| Forecast margin colour — within threshold → warning | 6.3-UNIT-044 | unit | FULL |
| Forecast margin colour — at/above target → positive | 6.3-UNIT-045 | unit | FULL |
| Click P&L action navigates to `/projects?view=pnl&project_id=<uuid>` and opens existing P&L panel | Manual browser verification (Story 6.3 Task 8) | wasm-runtime + leptos-router | **MANUAL VERIFIED** |
| Inaccessible project on deep-link falls back to existing P&L access-check error | Implicit: existing project route guards (covered by Story 5.x P&L tests) | api | FULL (inherited) |

**Heuristics**
- Logic coverage: full across all 8 branches of the two colour classifiers + 5 stable change-detection keys.
- Drill-down (deep-link click): requires Leptos router runtime; deferred.

**AC#3 verdict:** **COVERED + manual browser verified** — `margin_status_class` and `forecast_status_class` are exhaustively branch-covered, all margin/forecast change keys are unit-asserted, backend surfaces `margin_alert` semantics, forecast/P&L regression suites passed, and the deep-link click path was manually verified.

#### AC#4 — Sort by Margin / Budget Utilization / End Date

| Sub-aspect | Mapped Tests | Level | Coverage |
|------------|--------------|-------|----------|
| Margin mode places lowest current margin first | 6.3-UNIT-028 | unit | FULL |
| Budget Utilization mode places over-budget first; unconfigured last | 6.3-UNIT-029 | unit | FULL |
| End Date mode places soonest end date first | 6.3-UNIT-030 | unit | FULL |
| Deterministic name tiebreaker | 6.3-UNIT-031 | unit | FULL |
| `pm_utilization_score` sentinel keeps unconfigured cards at the bottom in Budget Utilization mode | 6.3-UNIT-046 | unit | FULL |
| `pm_utilization_score` boost keeps over-budget cards above in-budget cards at the same nominal utilization | 6.3-UNIT-047 | unit | FULL |
| Empty PM dashboard tolerates sort-mode toggles in all three branches | 6.3-UNIT-051 | unit | FULL |
| Sort change preserves client-side PM data and selected state | Code review + manual browser verification | wasm-runtime | **MANUAL VERIFIED** |
| Segmented control `aria-pressed` selected-state semantics | Manual browser verification | wasm-runtime | **MANUAL VERIFIED** |

**Heuristics**
- Pure-function coverage: full across every sort mode + tiebreaker + score invariant + edge case.
- Accessibility: deferred to manual verification.

**AC#4 verdict:** **COVERED + manual browser verified** — every documented sort invariant is unit-tested, rendered `aria-pressed` selected state was verified, and browser sorting matched the expected card order.

### 3.2 Coverage Status Summary

| AC | Priority | Logic Coverage | Visual/Runtime Coverage | Overall | Justification |
|----|----------|----------------|-------------------------|---------|---------------|
| #1 | P0 | COVERED (10 backend INT + 3 frontend UNIT badge helpers) | MANUAL VERIFIED (rendered card layout + live backend execution) | **PASS_WITH_ADVISORY** | Strong helper/service coverage plus completed browser verification. |
| #2 | P0 | COVERED (backend over-budget INT + over-budget transition unit + badge override unit) | MANUAL VERIFIED (warning SVG/label + flash + live backend execution) | **PASS_WITH_ADVISORY** | Defensive badge override locks the contract; rendered DOM verified manually. |
| #3 | P1 | COVERED (margin_alert INT + forecast-health INT + 4 margin branches + 4 forecast branches + 5 change keys) | MANUAL VERIFIED (deep-link click flow + forecast/P&L regression run) | **PASS_WITH_ADVISORY** | Every colour branch and change key is mapped; runtime/regression execution completed. |
| #4 | P1 | COVERED (3 sort modes + tiebreaker + 2 score invariants + empty slice) | MANUAL VERIFIED (rendered selected state and browser order) | **PASS_WITH_ADVISORY** | Sort logic is exercised and browser ordering was verified. |

### 3.3 Test Reuse / Duplication Check

- No duplicate coverage across levels: backend INT covers the service-layer DTO + branch logic; frontend UNIT covers pure helpers + change-detection. Different surfaces, no overlap.
- UNIT-033 (`pm_over_budget_transition_produces_expected_key`) asserts three keys at once (`is_over_budget`, `budget_overrun_idr`, `health_status`); UNIT-048 (`pm_health_status_change_produces_stable_key`) asserts `health_status` in isolation. Intentional: UNIT-048 catches a regression where the transition path remains intact but a standalone severity flip is missed.
- UNIT-035 (`is_over_budget` forces negative badge) and UNIT-036 (severity → badge mapping) are paired by design — UNIT-035 prevents a future re-tuning of `derive_health_status` from accidentally hiding an overrun behind a non-critical health status.
- UNIT-046 / UNIT-047 (sort score invariants) are paired by design — the unconfigured sentinel and the over-budget boost together guarantee the documented ordering even when nominal utilization values collide.
- Backend INT-004/007/008/009/010 form a deliberate branch-completion set against forecast-driven health, `project_budget_status`, and `derive_health_status`; each test pins one branch.

### 3.4 Mapping Matrix Validation

- ✅ All P0 ACs have multiple mapped tests across at least two layers (INT + UNIT).
- ✅ All P1 sub-aspects with formal contract have unit or integration coverage.
- ✅ All `budget_status` branches and the documented `health_status` branches have at least one INT test.
- ✅ All documented `dashboard_value_map` keys from Story 6.3 Task 3 have at least one UNIT test.
- ✅ Auth/authz inherited from Story 6.1: 401/403/stale-token all covered.
- ⚠️ UI journey E2E: missing for all four ACs (no harness; carry-over advisory).
- ⚠️ UI state on rendered DOM: missing for all four ACs (no Leptos component harness; carry-over).
- ⚠️ Forecast-service failure degradation path not deterministically exercised (advisory caveat #4).

---

## Step 4 — Gap Analysis, Recommendations & Coverage Matrix

### 4.1 Execution Mode

- `tea_execution_mode: auto` → resolved to **sequential** (single-conversation flow; no subagent dispatch requested).
- `tea_capability_probe: true` — agent-team/subagent capabilities not probed for an active orchestration request.

### 4.2 Gap Classification

| AC | Coverage | Priority | Classification |
|----|----------|----------|----------------|
| AC#1 | COVERED + manual browser verified | P0 | pass_with_advisory |
| AC#2 | COVERED + manual browser verified | P0 | pass_with_advisory |
| AC#3 | COVERED + manual browser verified | P1 | pass_with_advisory |
| AC#4 | COVERED + manual browser verified | P1 | pass_with_advisory |

- **Critical (P0) uncovered gaps:** 0
- **High (P1) uncovered gaps:** 0
- **Medium (P2) uncovered gaps:** 0
- **Low (P3) uncovered gaps:** 0
- **Manual-verified visual/runtime items:** 4 (manual browser verification completed)
- **Unit-only items (no cross-level coverage):** 0 (every AC has at least one INT or inherited regression test)

No requirement is wholly uncovered. The rendered DOM, Leptos router behaviour, and CSS flash timing were manually verified; repeatable automation for those browser behaviours remains advisory because the repo has no Playwright/Cypress harness or `wasm-bindgen-test`.

### 4.3 Coverage Heuristics — Findings

| Heuristic | Count | Items |
|-----------|-------|-------|
| Endpoints without tests | 0 | (none — `/api/v1/dashboard` is exhaustively tested across all new branches) |
| Auth/authz missing negative paths | 0 | inherited from Story 6.1 (401/403/stale-token covered) |
| Happy-path-only criteria | 1 | Forecast-service failure inline degradation not exercised deterministically |
| UI journeys without repeatable E2E | 4 | AC#1/AC#2/AC#3/AC#4 manually verified; no automated browser harness |
| UI states without repeatable component automation | 4 | Card render (AC#1), warning icon visual (AC#2), deep-link click (AC#3), sort-button a11y (AC#4) manually verified |
| Branch coverage on budget_status | 0 missing | all four branches asserted (INT-002/007/008/009) |
| Branch coverage on visual-state helpers | 0 missing | every branch of `budget_badge_class`/`margin_status_class`/`forecast_status_class` asserted |
| Stable change-detection keys for Story 6.3 Task 3 | 0 missing | PM visible value-map keys asserted, including forecast revenue signal and threshold keys |

### 4.4 Coverage Statistics

| Metric | Value |
|--------|-------|
| Total requirements (oracle items) | 4 |
| Static/logic coverage mapped | 4 |
| Manual browser verification completed | 4 |
| Uncovered (NONE) | 0 |
| Overall static/logic coverage percentage | 100% |
| P0 static/logic coverage | 2/2 (100%) |
| P1 static/logic coverage | 2/2 (100%) |
| P2 coverage | n/a (no P2 ACs) |
| P3 coverage | n/a (no P3 ACs) |
| Total unique tests inventoried (story-scoped) | 34 (10 backend INT + 24 frontend UNIT) |
| Total tests considered (incl. regression) | 109 |
| Tests by level (story-scoped) | api: 10, unit: 24, e2e: 0, component: 0 |
| Test cases skipped / fixme | 0 / 0 |
| Required verification remaining | 0 |

> **Interpretation note:** Story 6.3's ACs have strong pure-function and service-layer mappings — budget thresholds, severity classifiers, sort invariants, and stable change-detection keys are all reachable by the current harness. Required runtime verification is now green. The gate remains advisory only because repeatable browser/component automation is not present.

### 4.5 Recommendations (Prioritized)

| Priority | Action | Affected items |
|----------|--------|----------------|
| MEDIUM | Introduce a frontend test harness (Playwright + `cargo-leptos serve`, or `wasm-bindgen-test`) so the PM deep-link click flow, sort-control accessibility, and CSS flash on new PM keys can be automated. Cross-story tech debt carried over from Stories 6.1 and 6.2. | AC#3, AC#4 |
| MEDIUM | Add a test seam for forecast-service failure (`get_project_pl_forecast` error) so inline zeroed forecast fields plus the bounded per-card warning can be deterministically exercised. | Cross-cutting (Story 6.3 only happy-path criterion) |
| LOW | Run `/bmad-testarch-test-review` (Acceptance Auditor + Edge Case Hunter) over the current Story 6.3 test set (10 backend INT + 24 frontend UNIT) before landing if another adversarial pass is desired. | Story-wide |

### 4.6 Coverage Matrix Artifact

No committed full coverage-matrix artifact exists for this run. The committed slim artifacts are `_bmad-output/test-artifacts/traceability/6-3-gate-decision.json` and `_bmad-output/test-artifacts/traceability/6-3-e2e-trace-summary.json`; frontmatter records `tempCoverageMatrixPath: not_committed`.

### 4.7 Phase 1 Summary

```
Phase 1 Complete: Static Coverage Map Generated

📊 Coverage Statistics:
- Total Requirements: 4
- Static/logic coverage mapped: 4 (100%)
- Manual browser verification completed: 4
- Uncovered: 0

🎯 Priority Coverage (static/logic layer):
- P0 static/logic coverage: 2/2 (100%)
- P1 static/logic coverage: 2/2 (100%)
- P2: n/a
- P3: n/a

⚠️ Gaps Identified (uncovered):
- Critical (P0): 0
- High (P1): 0
- Medium (P2): 0
- Low (P3): 0
- Manual-verified visual/runtime items: 4 (one per AC; Task 8 completed)

🔍 Coverage Heuristics:
- Endpoints without tests: 0
- Auth negative-path gaps: 0
- Happy-path-only criteria: 1 (forecast-service failure degradation)
- UI journeys without E2E: 4
- UI states missing coverage: 4
- Budget threshold branches: 0 missing
- Visual-state helper branches: 0 missing
- Story Task 3 change-detection keys: 0 missing

📝 Recommendations: 5

🔄 Phase 2: Gate decision (PASS_WITH_ADVISORY; required runtime verification is green)
```

---

## Step 5 — Gate Decision

### 5.1 Coverage Reclassification

| AC | Step 3 (detailed) | Step 5 (gate input) | Justification |
|----|-------------------|---------------------|---------------|
| AC#1 | Covered + manual browser verified | **PASS_WITH_ADVISORY** | 10 backend INT + 3 frontend UNIT cover the card contract and badge mapping; browser render and live backend execution passed. |
| AC#2 | Covered + manual browser verified | **PASS_WITH_ADVISORY** | Backend INT-002 forces over-budget; UNIT-033 surfaces transition keys; UNIT-035 locks badge override; visual warning label/flash and live backend execution passed. |
| AC#3 | Covered + manual browser verified | **PASS_WITH_ADVISORY** | Backend INT-003/004 plus margin/forecast helper branches and change keys are mapped; deep-link click and forecast/P&L regression execution passed. |
| AC#4 | Covered + manual browser verified | **PASS_WITH_ADVISORY** | All sort modes + tiebreaker + score invariants + empty slice are unit-tested; browser sort order and selected state passed. |

### 5.2 Gate Decision Criteria

| Criterion | Required | Actual | Status |
|-----------|----------|--------|--------|
| P0 static/logic mapping | 100% | 100% (2/2 mapped) | MET |
| P1 static/logic mapping | ≥ 90% | 100% (2/2 mapped) | MET |
| Frontend unit execution | green | 51/51 PASS | MET |
| Backend integration execution | green | live `dashboard_tests` 37/37 PASS | MET |
| Forecast/P&L regression execution | green | `project_pl_forecast_tests` 11/11 PASS; `project_pl_tests` 10/10 PASS | MET |
| Manual browser verification | complete | completed against local `xynergy-server` | MET |
| Frontend WASM compile | clean | clean (9 pre-existing dead-code warnings only) | MET |
| Tailwind rebuild | clean | clean | MET |

Gate is **PASS_WITH_ADVISORY**. Static/logic coverage is mapped and required runtime verification is green; remaining advisory is future repeatable DOM/router automation plus deterministic forecast-service failure injection.

### 5.3 Machine-Readable Outputs

| Artifact | Path |
|----------|------|
| Full coverage matrix | not committed |
| Gate decision (slim) | `_bmad-output/test-artifacts/traceability/6-3-gate-decision.json` |
| E2E trace summary | `_bmad-output/test-artifacts/traceability/6-3-e2e-trace-summary.json` |
| Trace report (this file) | `_bmad-output/test-artifacts/traceability/6-3-project-health-dashboard-traceability.md` |

### 5.4 Display Summary

```
🚨 GATE DECISION: PASS_WITH_ADVISORY

📊 Coverage Analysis:
- P0 Coverage:        100% (Required: 100%) → MET
- P1 Coverage:        100% (PASS target: 90%, minimum: 80%) → MET
- Overall Coverage:   100% (Minimum: 80%)   → MET

Decision Rationale:
All 4 ACs have static/logic coverage mapped (10 backend integration tests
+ 24 frontend unit tests scoped to Story 6.3, plus 27+27 regression tests
considered). Frontend native suite 51/51 PASS, frontend WASM compile clean,
and backend dashboard_tests compile offline. Required runtime verification is
green: live backend execution passed, forecast/P&L regression execution
passed with a live DATABASE_URL, and manual browser verification completed.

Critical static gaps: 0
Open verification concerns: 0
Advisory gaps: 3

📝 Top Recommended Actions:
1. [MEDIUM] Introduce a frontend test harness (Playwright + cargo-leptos
   serve or wasm-bindgen-test) so the PM deep-link click flow, sort-control
   accessibility, and CSS flash can be automated. Cross-story tech debt.
2. [MEDIUM] Add a test seam for forecast-service failure so inline zeroed
   forecast fields plus bounded per-card warning can be exercised.

📂 Full Report: _bmad-output/test-artifacts/traceability/6-3-project-health-dashboard-traceability.md

GATE: PASS_WITH_ADVISORY — Story 6.3 is verified. Remaining advisory is
   future repeatable browser/component automation and forecast-service
   failure injection coverage.
```

### 5.5 Next Actions

| Priority | Action |
|----------|--------|
| MEDIUM | Introduce a frontend test harness (Playwright + `cargo-leptos serve`, or `wasm-bindgen-test`) — cross-story tech debt carried over from Stories 6.1 and 6.2. |
| MEDIUM | Add a test seam for forecast-service failure so inline zeroed forecast fields plus the bounded per-card warning can be deterministically exercised. |
| LOW | Run `/bmad-testarch-test-review` over the current Story 6.3 test set (10 backend INT + 24 frontend UNIT) if another adversarial pass is desired. |

---

*Generated by Master Test Architect via `bmad-testarch-trace` (Create mode) — 2026-05-22.*
