---
story: '6-3-project-health-dashboard'
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
  - '_bmad-output/implementation-artifacts/6-3-project-health-dashboard.md'
  - '_bmad-output/implementation-artifacts/sprint-status.yaml'
  - '_bmad-output/project-context.md'
  - 'src/backend/src/services/dashboard_service.rs'
  - 'src/backend/tests/dashboard_tests.rs'
  - 'src/frontend/src/pages/dashboard.rs'
  - 'src/frontend/Cargo.toml'
  - 'src/frontend/package.json'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/test-levels-framework.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/test-priorities-matrix.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/data-factories.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/test-quality.md'
---

# Test Automation Expansion — Story 6.3 Project Health Dashboard

## 1. Preflight & Context

### Stack Detection

- `test_stack_type: auto` → resolved to **fullstack** (Rust workspace `Cargo.toml` + Leptos `src/frontend/Cargo.toml` + Tailwind `src/frontend/package.json`).
- No browser test indicators (`playwright.config.*`, `cypress.config.*`) — same posture as Stories 6.1/6.2.
- Profile: **API/backend integration + frontend native unit tests** (the PM card helpers, sort, badge/margin/forecast class helpers, and change-detection map are pure functions usable from non-WASM targets).

### Execution Mode

**BMad-Integrated.** Story `6-3-project-health-dashboard.md` is now `done` after code review and follow-up verification. The prior blockers are resolved in this environment: local Postgres was started, migrations were applied, live backend execution passed, forecast/P&L regression suites passed with a live `DATABASE_URL`, and manual browser verification completed against `xynergy-server`.

- 6 Story 6.3 backend integration tests already existed before this automation expansion (`6.3-INT-001`..`6.3-INT-006` in `src/backend/tests/dashboard_tests.rs`), including the post-review forecast-driven health regression.
- 7 Story 6.3 frontend native unit tests (PM sort modes, name tiebreaker, forecast/over-budget change keys, PM sensitive-field exclusion in `src/frontend/src/pages/dashboard.rs::tests`).

This expansion targets uncovered branches of `derive_health_status` / `project_budget_status` on the backend and the new visual-state helpers + remaining change-detection keys on the frontend.

### Test Framework Verified

- Frontend native tests gated on `#[cfg(all(test, not(target_arch = "wasm32")))]` inside `src/frontend/src/pages/dashboard.rs::tests`.
- Run command: `cargo test -p xynergy-frontend --lib`.
- Backend route regression: `DATABASE_URL=postgres://xynergy:xynergy@localhost:5432/xynergy cargo test --package xynergy-backend --test dashboard_tests`.
- WASM compile contract: `cargo check -p xynergy-frontend --target wasm32-unknown-unknown`.

### Knowledge Loaded (core tier)

- `test-levels-framework.md` — unit-level selection for pure helpers; backend integration retained for response-contract coverage of derived fields.
- `test-priorities-matrix.md` — P0/P1/P2 by risk × impact.
- `data-factories.md` — reuse existing in-module factories (`pm_card_default`, `pm_response_with`, `empty_response`, `set_project_budget_and_margin`, `insert_project_expense`); no new factory layers introduced.
- `test-quality.md` — deterministic, isolated, single-invariant assertions.

Extended/specialized fragments not loaded — no contract tests, no email/auth flows, no Playwright (no browser harness in repo).

---

## 2. Coverage Plan

### Acceptance Criteria → Test Mapping

| AC | Scenario | Test Level | Pre-existing | New |
|----|----------|-----------|--------------|-----|
| #1 PM cards show name, budget status, current margin, forecast margin | Field presence, threshold branches (healthy / warning / critical / unconfigured), value plumbing | INT (backend) + UNIT (frontend) | 6.3-INT-001 field presence, 6.3-INT-002 over-budget critical, 6.3-INT-004 forecast-driven health | **+3 backend (INT-007/008/009) + 0 frontend** |
| #2 Over-budget ⇒ red status + warning icon | `is_over_budget`/`budget_overrun_idr`/`budget_status=critical`/`health_status=critical` + `budget_badge_class`/`budget_badge_label` visual mapping | INT + UNIT | 6.3-INT-002 backend, 6.3-UNIT-033 transition key | **+3 frontend (UNIT-035..037) badge helpers** |
| #3 Below-target margin ⇒ orange/red + P&L deep-link | `margin_alert` semantics, `margin_status_class` + `forecast_status_class` mapping, P&L deep-link click path | INT + UNIT + manual | 6.3-INT-003 margin_alert, 6.3-UNIT-032 forecast_margin_pct key | **+8 frontend (UNIT-038..045) margin/forecast class helpers; +0 deep-link automation (Leptos router, manual)** |
| #4 Sort by Margin / Budget Utilization / End Date | Pure sort helper + `pm_utilization_score` invariants + empty-slice safety | UNIT | 6.3-UNIT-028..031 sort modes + tiebreaker | **+3 frontend (UNIT-046/047/051) score sentinel/boost + empty-slice safety** |
| Cross-cutting: health_status / projected_total_cost_idr / forecast_variance_from_target_pct change-detection keys | Change-detection map keys called out in story Task 3 | UNIT | partial (UNIT-032 forecast_margin_pct only) | **+3 frontend (UNIT-048..050) remaining stable keys** |
| Cross-cutting: derive_health_status `unconfigured` branch | Branch unreached by INT-001..006 | INT | none | **+1 backend (INT-010)** |

### Existing Coverage — Story 6.3

**Backend (6 INT in `src/backend/tests/dashboard_tests.rs`):**

| ID | Test | Priority |
|----|------|----------|
| 6.3-INT-001 | `project_manager_card_includes_forecast_and_overrun_fields` | P1 |
| 6.3-INT-002 | `project_manager_card_marks_over_budget_critical` | P0 |
| 6.3-INT-003 | `project_manager_card_surfaces_margin_alert_when_below_target` | P0 |
| 6.3-INT-004 | `project_manager_card_forecast_below_target_drives_critical_health` | P1 |
| 6.3-INT-005 | `project_manager_active_projects_remain_bounded_after_forecast_wiring` | P1 |
| 6.3-INT-006 | `project_manager_card_scope_unchanged_by_story_6_3` | P0 |

**Frontend (7 UNIT in `src/frontend/src/pages/dashboard.rs::tests`):**

| ID | Test | Priority |
|----|------|----------|
| 6.3-UNIT-028 | `pm_sort_by_margin_orders_lowest_first` | P1 |
| 6.3-UNIT-029 | `pm_sort_by_budget_utilization_orders_over_budget_first` | P1 |
| 6.3-UNIT-030 | `pm_sort_by_end_date_orders_soonest_first` | P1 |
| 6.3-UNIT-031 | `pm_sort_uses_project_name_as_tiebreaker` | P2 |
| 6.3-UNIT-032 | `pm_forecast_margin_change_produces_expected_key` | P0 |
| 6.3-UNIT-033 | `pm_over_budget_transition_produces_expected_key` | P0 |
| 6.3-UNIT-034 | `pm_value_map_excludes_sensitive_keys_for_pm_section` | P0 |

### Expansion Coverage (this run) — 21 new tests

**Backend (4 new INT in `src/backend/tests/dashboard_tests.rs`):**

| ID | Test | Priority | Gap Closed |
|----|------|----------|------------|
| 6.3-INT-007 | `project_manager_card_unconfigured_budget_status_when_no_budget` | P1 | `project_budget_status` `unconfigured` branch when `total_budget_idr <= 0` |
| 6.3-INT-008 | `project_manager_card_warning_budget_status_between_50_and_80_pct` | P1 | Middle threshold (50–80% utilization → `warning`) — uncovered branch |
| 6.3-INT-009 | `project_manager_card_healthy_budget_status_below_50_pct` | P2 | `<50%` branch + `budget_utilization_pct` value plumbing (≈30%) |
| 6.3-INT-010 | `project_manager_card_health_status_unconfigured_when_no_budget_and_no_alert` | P1 | `derive_health_status` `unconfigured` branch (no budget + no margin_alert) |

**Frontend (17 new UNIT in `src/frontend/src/pages/dashboard.rs::tests`):**

| ID | Test | Priority | Gap Closed |
|----|------|----------|------------|
| 6.3-UNIT-035 | `budget_badge_class_returns_badge_negative_when_over_budget` | P1 | `is_over_budget=true` must force `badge-negative` regardless of `health_status` |
| 6.3-UNIT-036 | `budget_badge_class_maps_budget_status_tokens` | P1 | Each budget status severity → expected Tailwind utility (positive/warning/negative/neutral fallback) |
| 6.3-UNIT-037 | `budget_badge_label_returns_human_readable_labels` | P2 | Label contract: "Over budget" / "On track" / "Watch" / "At risk" / "Unconfigured" |
| 6.3-UNIT-038 | `margin_status_class_negative_when_margin_alert_present` | P1 | `margin_alert_present` dominates → negative styling |
| 6.3-UNIT-039 | `margin_status_class_negative_when_below_target_beyond_threshold` | P1 | Gap from target exceeds alert threshold → negative |
| 6.3-UNIT-040 | `margin_status_class_warning_when_below_target_within_threshold` | P1 | Below target but inside threshold → warning |
| 6.3-UNIT-041 | `margin_status_class_positive_when_at_or_above_target` | P2 | At/above target → positive (incl. exact-equal boundary) |
| 6.3-UNIT-042 | `forecast_status_class_muted_when_unavailable` | P2 | Forecast unavailable → muted, ignoring noise from placeholder card |
| 6.3-UNIT-043 | `forecast_status_class_negative_when_variance_below_threshold` | P1 | `|variance| > threshold` → negative |
| 6.3-UNIT-044 | `forecast_status_class_warning_when_variance_below_target_within_threshold` | P1 | Below target but within threshold → warning |
| 6.3-UNIT-045 | `forecast_status_class_positive_when_variance_at_or_above_target` | P2 | At/above target → positive (incl. zero variance) |
| 6.3-UNIT-046 | `pm_utilization_score_returns_sentinel_for_unconfigured_card` | P1 | Sentinel `-1.0` keeps unconfigured cards at the bottom of Budget Utilization sort |
| 6.3-UNIT-047 | `pm_utilization_score_boosts_over_budget_card_above_in_budget_card_at_same_utilization` | P1 | Over-budget overrun boost keeps overruns visible even at identical % |
| 6.3-UNIT-048 | `pm_health_status_change_produces_stable_key` | P1 | Polling tick that flips `health_status` surfaces the stable change key (`project_manager.project.<id>.health_status`) |
| 6.3-UNIT-049 | `pm_projected_total_cost_idr_change_produces_stable_key` | P1 | Forecast cost change surfaces stable key (story Task 3 explicit) |
| 6.3-UNIT-050 | `pm_forecast_variance_from_target_pct_change_produces_stable_key` | P1 | Forecast variance change surfaces stable key (story Task 3 explicit) |
| 6.3-UNIT-051 | `sort_pm_cards_handles_empty_slice_without_panic` | P2 | Empty PM dashboard must still tolerate sort-mode toggles in all 3 branches |

### Coverage Scope

**critical-paths + branch completion.** Every `derive_health_status` and `project_budget_status` branch now has a backend integration assertion. Every visual-state helper (`budget_badge_class`, `budget_badge_label`, `margin_status_class`, `forecast_status_class`, `pm_utilization_score`) and every stable change-detection key listed in Story 6.3 Task 3 now has a frontend unit assertion.

### Test Level Justification

- **Backend tests (4)** are all integration (`#[sqlx::test]`) because the branches under test live inside service code that touches Postgres (`build_project_manager_dashboard` joins `projects` + P&L + forecast). Pure-unit coverage of `project_budget_status` / `derive_health_status` would not exercise the wiring from row → DTO field.
- **Frontend tests (17)** are all native unit (`#[cfg(all(test, not(target_arch = "wasm32")))]`) because every helper under test is a pure function (`&[T] -> T`, `&str -> &'static str`, `&RoleDashboardResponse -> HashMap<String, String>`).
- **No repeatable E2E tests added.** Repo has no Playwright/Cypress harness. The PM deep-link (`/projects?view=pnl&project_id=<uuid>`), polling refresh of new PM values, sort-control keyboard reachability (`aria-pressed`), and CSS flash on changed values are WASM-only behaviours; manual browser verification was completed on 2026-05-22 with a local dev server and Postgres.
- **No new dependencies, no new harness.** Preserves the story Architecture Compliance "no new dependency" rule and matches Stories 6.1/6.2 posture.

---

## 3. Files Updated

- `src/frontend/src/pages/dashboard.rs` — `mod tests` expanded from 34 to 51 tests; 17 new tests appended inside the existing `#[cfg(all(test, not(target_arch = "wasm32")))]` block.
- `src/backend/tests/dashboard_tests.rs` — Story 6.3 integration suite now has 10 tests total. This automation expansion added 4 tests (`6.3-INT-007`..`6.3-INT-010`) after the existing forecast/scope tests; the post-review forecast-driven health regression is tracked as `6.3-INT-004`.

---

## 4. Validation

### Test Execution

**Frontend native unit tests:**

```
SQLX_OFFLINE=true cargo test -p xynergy-frontend --lib
```

Result: **51 passed; 0 failed; 0 ignored** (34 pre-existing + 17 new).

**Frontend WASM compile contract:**

```
SQLX_OFFLINE=true cargo check -p xynergy-frontend --target wasm32-unknown-unknown
```

Result: **Finished `dev` profile** — 9 pre-existing dead-code warnings unrelated to dashboard.rs; no errors.

**Backend dashboard binary compile:**

```
SQLX_OFFLINE=true cargo test -p xynergy-backend --test dashboard_tests --no-run
```

Result: **Finished `test` profile** — `dashboard_tests` binary compiled cleanly (1 pre-existing future-incompat warning from `sqlx-postgres 0.7.4`, unchanged from prior stories). Initial live execution failed during sqlx test database setup with `PoolTimedOut`; after starting local Postgres and applying migrations, live execution passed:

```
DATABASE_URL=postgres://xynergy:xynergy@localhost:5432/xynergy cargo test -p xynergy-backend --test dashboard_tests
```

Result: **37 passed; 0 failed; 0 ignored**.

**Forecast/P&L regression suites:**

```
DATABASE_URL=postgres://xynergy:xynergy@localhost:5432/xynergy cargo test -p xynergy-backend --test project_pl_forecast_tests
DATABASE_URL=postgres://xynergy:xynergy@localhost:5432/xynergy cargo test -p xynergy-backend --test project_pl_tests
```

Results: **project_pl_forecast_tests 11 passed; project_pl_tests 10 passed**.

**Manual browser verification:**

Completed on 2026-05-22 against local `xynergy-server` at `http://127.0.0.1:3000` using a PM fixture (`pm.verify@xynergy.com`). Verified PM health cards with current/forecast margins, over-budget accessible warning label, below-target margin alerts, sort ordering for Margin / Budget Utilization / End Date, `/projects?view=pnl&project_id=<uuid>` deep-link into the existing P&L panel, close-query cleanup back to `/projects`, manual Refresh timestamp update, and 30-second polling value update with `dashboard-change-flash`.

### Quality Checklist

- [x] All new tests deterministic (no `sleep`, no random non-seeded data — `Uuid::new_v4()` used only where the test asserts presence-by-id, not equality-by-id).
- [x] Frontend tests are isolated (each test builds its own response from `empty_response(role)` + `pm_card_default(...)`). Backend tests are written with `#[sqlx::test(migrations = "../../migrations")]`; live execution passed after local Postgres was started.
- [x] Each test focused on one invariant; helper factories keep noise out of assertions.
- [x] No duplicate coverage with the 13 pre-existing Story 6.3 tests (verified by ID and by reading the file before appending).
- [x] All 4 `budget_status` branches now have a backend assertion (`unconfigured`, `healthy`, `warning`, `critical`).
- [x] All `derive_health_status` severity branches now have backend assertions: `critical` via over-budget, `warning` via 50-80% budget utilization, `healthy` via below-50% spend without revenue signal, and `unconfigured` via no-budget + no-alert.
- [x] All visual-state helpers (`budget_badge_class`, `budget_badge_label`, `margin_status_class`, `forecast_status_class`, `pm_utilization_score`) now have at least one frontend assertion per branch.
- [x] All story Task 3 change-detection keys (`budget_utilization_pct`, `is_over_budget`, `budget_overrun_idr`, `forecast_margin_pct`, `projected_total_cost_idr`, `forecast_variance_from_target_pct`, `health_status`) now have frontend assertions.
- [x] Browser verification completed with the in-app browser against local `xynergy-server`.
- [x] Test artifacts written to `_bmad-output/test-artifacts/` only.

---

## 5. Assumptions & Risks

### Assumptions

- The frontend visual-state helpers (`budget_badge_class`, `budget_badge_label`, `margin_status_class`, `forecast_status_class`) and `pm_utilization_score` are the authoritative source of severity → CSS-token mapping for the PM dashboard cards. Tests bind to their current return values; a rename of a Tailwind utility (e.g., `badge-positive` → `chip-positive`) will fail these assertions intentionally and surface the rename rather than silently drifting.
- The PM card DTO (`ProjectHealthCard`) on the frontend carries every Story 6.3 field as a typed value (with `#[serde(default)]` shim for forward compatibility). Tests construct cards directly without exercising the deserialization path; deserialization is exercised implicitly by the WASM compile and by real polling.
- The backend `derive_health_status` rules in `dashboard_service.rs::derive_health_status(...)` are the authoritative severity ladder (`unconfigured` ↔ `healthy` ↔ `warning` ↔ `critical`). Tests bind to each documented severity branch without re-stating the full matrix in every test.
- Local PostgreSQL availability is not required for this expansion run; the new backend tests share the proven `#[sqlx::test]` migration scaffolding used by 27 prior tests in the same file. They are expected to execute green on any workstation with `DATABASE_URL` pointing at a running PG, matching the pattern established by Stories 5.3 / 5.4 / 6.1 / 6.2.

### Risks

- **Low** — `pm_utilization_score_boosts_over_budget_card_above_in_budget_card_at_same_utilization` uses a small fixture (`total_budget_idr=100`, `budget_overrun_idr=1`) to keep the assertion stark. The boost formula is `max(utilization, 100) + (overrun / budget) * 100`. If the boost formula is ever softened (e.g., capped or zeroed), this test will fail by direction (`>` becomes `≤`) — that's the intended failure mode.
- **Low** — `forecast_status_class_negative_when_variance_below_threshold` and `margin_status_class_negative_when_below_target_beyond_threshold` rely on strict-greater-than comparisons (`|variance| > threshold`, `(target - margin) > threshold`). The current implementation uses `>` (not `≥`), so boundary values like `variance=-5, threshold=5` would map to **warning**, not negative. Tests deliberately pick values outside the boundary so a future relaxation from `>` to `≥` would not silently break them.
- **Low** — `pm_health_status_change_produces_stable_key` and friends construct test cards with `Uuid::new_v4()`. Two siblings hashing into the same UUID is astronomically unlikely; tests rely only on equality of the chosen id between `prev` and `next` maps, not on absolute id value.
- **None observed** — frontend helpers are pure-functional, live `dashboard_tests` passed after local Postgres was available, and manual browser verification completed successfully.

### Out of Scope

- **Automated PM card deep-link click path** (`/projects?view=pnl&project_id=<uuid>`) — Leptos router behaviour requires a browser or WASM test runner. Manual browser verification completed; repeatable automation remains future harness work.
- **Automated polling refresh of new PM values + CSS flash on `health_status` / forecast values** — these are WASM-runtime/browser concerns. Manual browser verification completed on 2026-05-22.
- **Automated sort-control keyboard reachability (`aria-pressed`, focus styles)** — accessibility validation requires a real DOM; the underlying sort helper is fully unit-tested and the rendered `aria-pressed` selected state was manually verified.
- **Backend forecast-service failure degradation** (inline zeroed forecast fields plus bounded warning) — exercising this branch deterministically requires injecting an error from `get_project_pl_forecast`, which has no test seam today. Tracked implicitly by the `warning: Option<String>` field on `ProjectHealthCard` being deserializable; `safe_unavailable_card` is the separate P&L failure fallback.
- **Cross-role regression for HR/DH/Finance/Admin** — the existing 27 backend tests + 34 frontend tests still cover those role panels; this expansion deliberately stays inside the Story 6.3 PM section.

---

## 6. Next Recommended Workflow

- **`/bmad-testarch-trace`** — refreshed traceability should now report **PASS_WITH_ADVISORY**: live backend execution, forecast/P&L regression execution, and manual browser verification are green; repeatable browser automation remains advisory tech debt.
- Optionally **`/bmad-testarch-test-review`** — adversarial pass (Acceptance Auditor + Edge Case Hunter) over the Story 6.3 test set before landing.
- Story 6.3 is now `done`; remaining advisory is future repeatable DOM/router automation and a deterministic test seam for forecast-service failure injection.

---

*Generated by Master Test Architect via `bmad-testarch-automate` (Create mode) — 2026-05-22.*
