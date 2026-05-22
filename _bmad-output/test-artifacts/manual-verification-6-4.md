---
story_id: '6.4'
story_key: 6-4-team-utilization-dashboard
story_title: 'Team Utilization Dashboard'
verification_date: '2026-05-22'
verifier: Putu (Master Test Architect, driven headlessly via Claude /browse)
environment:
  backend: 'http://127.0.0.1:3000 (xynergy-server, debug build)'
  frontend: 'WASM bundle from target/site/pkg (rebuilt 2026-05-22 17:48 UTC)'
  database: 'podman xynergy-db, PostgreSQL 16-alpine'
  fixture_sql: '_bmad-output/test-artifacts/dh-fixture-6-4.sql (requires xynergy.fixture_allow_seed=local-only)'
  dh_user: 'dh-6-4@xynergy.test (department_head)'
  fixture_department: '11111111-1111-1111-1111-111111111111 (DH-6-4 Fixture Dept)'
fixture_team:
  - 'Underutilized Maya (CTC Active, 30% in 2026-05) — UNDERUTILIZED → blue Assign'
  - 'Healthy Arjun     (CTC Active, 85% in 2026-05) — HEALTHY → no action'
  - 'CTC-Missing Pria  (no CTC,    20% in 2026-05) — UNDERUTILIZED but Assign DISABLED'
  - 'Overallocated Sari (CTC Active, 70%+70%=140%)  — OVERALLOCATED → no action'
fixture_budget:
  period: '2026-05'
  total_idr: 1000000000
  alert_threshold_pct: 80
ac1_status: PASS
ac2_status: REVERIFY_REQUIRED
ac3_status: REVERIFY_REQUIRED
ac4_status: PASS
overall_status: REVIEW_PATCHED_REVERIFY_REQUIRED
gate_recommendation: 'Do not promote to done from this artifact alone. Code-review patches changed deep-link timing/error behavior and query validation after this manual pass; rerun browser verification before a clean gate.'
---

# Manual Browser Verification — Story 6.4 (Team Utilization Dashboard)

**Outcome:** ⚠️ **REVIEW PATCHED — REVERIFY REQUIRED**. This report records the initial live Department Head browser pass. Code-review patches later changed the `/team?assign_resource_id=...` timing/error behavior and dashboard range validation, so the AC#3 deep-link and guarded-error rows must be rerun before promoting the story to `done`.

The two open items from the trace gate (`CONCERNS`) were:

1. Manual DH browser verification — **resolved by this report**.
2. Frontend test harness for AC#3 deep-link automation — **carried forward as advisory** (shared with Story 6.3 backlog; not story-specific).

Recommendation: keep the story in `review` until the patched flows are rerun.

---

## AC#1 — Team rows: utilization %, current projects, available capacity, status

**Result:** ✅ PASS
**Evidence:** [01-ac1-dashboard-loaded.png](screenshots-6-4/01-ac1-dashboard-loaded.png)

Verified on `/dashboard` as Department Head:

| Member               | Util % | Available % | Current Projects                                 | Badge        | Action     |
|----------------------|--------|-------------|--------------------------------------------------|--------------|------------|
| CTC-Missing Pria     | 20.0%  | 80.0%       | DH-6-4 Demo Project (20%)                        | Underutilized | **Disabled Assign** (tooltip: "CTC data required to assign. Contact HR to complete employee setup.") |
| Healthy Arjun        | 85.0%  | 15.0%       | DH-6-4 Demo Project (85%)                        | Healthy       | —          |
| Overallocated Sari   | 140.0% | **0.0%**    | DH-6-4 Demo Project (70%), DH-6-4 Demo Project (70%) | Overallocated | —      |
| Underutilized Maya   | 30.0%  | 70.0%       | DH-6-4 Demo Project (30%)                        | Underutilized | Active Assign (blue) |

- **AC contract fields all present:** team members, current utilization %, current projects (with per-project allocation %), available capacity %.
- **Overallocated clamp:** Sari's `available = 0.0%` despite `utilization = 140%` — confirms `max(0, 100 - allocation)` clamp.
- **Aggregate KPIs:** AVG UTILIZATION 34.4%, AVG AVAILABLE 41.2%, UNDERUTILIZED 2, OVERALLOCATED 1, 4 team members, BUDGET 6%. The 34.4% is the weighted-working-day average over the default 30-day window (today → today+30), since allocations only span May the avg is diluted by zero June allocation days — matches PRD FR58 spec.

---

## AC#2 — Utilization trends per member for selected time range

**Result:** ⚠️ REVIEW_PATCHED — reverify range validation / selected-range browser path after code-review patches
**Evidence:**

- [02-ac2-trend-current-month.png](screenshots-6-4/02-ac2-trend-current-month.png) — Current Month preset
- [01-ac1-dashboard-loaded.png](screenshots-6-4/01-ac1-dashboard-loaded.png) — default Next 30 Days preset on initial dashboard load
- [03-ac2-trend-3-months.png](screenshots-6-4/03-ac2-trend-3-months.png) — 3 Months preset
- [04-ac2-trend-6-months.png](screenshots-6-4/04-ac2-trend-6-months.png) — 6 Months preset

Each preset click produces exactly one `GET /api/v1/dashboard?team_start_date=…&team_end_date=…` request:

| Preset         | Query (start / end)            | Latency | Bytes |
|----------------|--------------------------------|---------|-------|
| Current Month  | 2026-05-01 / 2026-05-31         | 21 ms   | 4249  |
| Next 30 Days   | 2026-05-22 / 2026-06-21         | initial load | 4249 |
| 3 Months       | 2026-05-22 / 2026-08-20         | 24 ms   | 4765  |
| 6 Months       | 2026-05-22 / 2026-11-18         | 20 ms   | 5280  |

**Polling-cadence stress test:** with the 6-month preset selected, network log cleared, waited **32 seconds**. Exactly **one** `GET /api/v1/dashboard?team_start_date=2026-05-22&team_end_date=2026-11-18` fired — proving:

- Polling continues automatically at the 30s cadence
- The user-selected range is preserved across polls (no reset to default)
- No interval stacking from the four preceding preset clicks (would have produced ≥2 requests if any prior timer had not been cancelled)
- No full-page flash; only the trend table and AVG KPI re-rendered

This is the live-verified counterpart to the unit-tested `get_untracked()` guard called out in the trace report.

---

## AC#3 — Underutilized → click Assign → /team deep-link round-trip

**Result:** ⚠️ REVIEW_PATCHED — reverify direct deep-link and guarded-error browser paths after code-review patches
**Evidence:**

- [05-ac3-deeplink-modal-opened.png](screenshots-6-4/05-ac3-deeplink-modal-opened.png) — modal opened from dashboard click, "Assigning: Underutilized Maya", assignable projects loaded
- [06-ac3-deeplink-modal-correct-resource.png](screenshots-6-4/06-ac3-deeplink-modal-correct-resource.png) — full modal with all fields (project, start/end date, allocation %, cost preview placeholder)

### Click flow (the production path, AC-required)

1. Clicked **Underutilized Maya → Assign** on `/dashboard`.
2. URL changed to `http://127.0.0.1:3000/team` — query param `?assign_resource_id=33333333-3333-3333-3333-333333333301` was consumed and cleared immediately (replace navigation, not push).
3. Requests fired (confirmed via `$B network`): `/api/v1/team` (twice), `/api/v1/projects/assignable`, `/api/v1/team/budget?period=2026-05`, `/api/v1/team/capacity-report`, `/api/v1/team/budget/breakdown`.
4. **Assignment modal opened with "Assigning: Underutilized Maya"** and the assignable-projects dropdown populated.
5. Closed modal via ✕, **reloaded the page** — modal did **NOT** reopen and URL remained `/team` (proves `replace: true` semantics; refresh/back/forward are immune).

### Edge cases

| Case                                              | URL after settle                                      | Modal opened? | Verdict |
|---------------------------------------------------|-------------------------------------------------------|---------------|---------|
| Malformed UUID `?assign_resource_id=not-a-uuid`   | `/team` (cleared)                                     | No            | ⚠️ Reverify patched bounded error |
| Random/unknown UUID `99…99`                       | `/team` (cleared)                                     | No            | ⚠️ Reverify patched bounded error |
| CTC-Missing Pria UUID `33…3303` (in-scope no CTC) | `/team` (cleared)                                     | No            | ⚠️ Reverify patched CTC error |
| **Disabled Assign button** for CTC-Missing Pria   | (button visibly grayed, `disabled` attr, tooltip)     | n/a           | ✅ PASS |

### Review Patch Follow-Up

The earlier observation that direct browser navigation to `/team?assign_resource_id=<valid-in-scope-CTC-Active-uuid>` did not open the modal is now treated as a real review finding, not a non-defect. The code-review patch waits for auth and team data before consuming the query param, normalizes UUID formatting, preserves unrelated query/hash state, and shows bounded errors for malformed, out-of-scope, CTC-missing, or non-assignable IDs. Rerun AC#3 browser verification before marking this story clean.

---

## AC#4 — Department budget gauge: Total / Committed / Spent / Available

**Result:** ✅ PASS
**Evidence:** [01-ac1-dashboard-loaded.png](screenshots-6-4/01-ac1-dashboard-loaded.png) (Department Budget panel is co-located on the dashboard)

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

- IDR formatting uses dot-grouped thousands (the `format_idr_*` helpers are unit-tested but visually confirmed here).
- Health color follows `alert_threshold_pct` (80% threshold, current 6% → green / "On track"). At >80% the bar transitions to amber, >100% to red — not exercised live because the fixture deliberately stays well under threshold, but the branch is covered by `budget_health_class_and_label_branches` in the frontend unit test set.

---

## Environment & Reproducibility

- **Database fixture:** `_bmad-output/test-artifacts/dh-fixture-6-4.sql` (idempotent via fixed UUIDs + ON CONFLICT; guarded by `xynergy.fixture_allow_seed=local-only`). Apply from the repo root with `podman exec -i xynergy-db psql -U xynergy -d xynergy -v ON_ERROR_STOP=1 -c "SET xynergy.fixture_allow_seed = 'local-only';" -f - < _bmad-output/test-artifacts/dh-fixture-6-4.sql`; clean up with the paired `_bmad-output/test-artifacts/dh-fixture-6-4-cleanup.sql` using the same command shape. Sanity row: `dh_user_present=1, head_id=44…44, scoped_resources=4, ctc_rows=3, allocations=5, budget=1000000000`.
- **Backend:** `target/debug/xynergy-server` launched with `.env` exported into the shell (no dotenv crate in repo; running via `cargo run --bin xynergy-server` from a non-loaded shell fails with `DATABASE_URL not set` — operationally important).
- **Frontend:** `bash build-frontend.sh` produces `target/site/pkg/xynergy_frontend.{js,wasm}`. Build completed cleanly in 1m22s release mode + wasm-bindgen.
- **Browser driver:** gstack `/browse` (headless Chromium) — auto-started persistent daemon, ~100ms per command.
- **No live errors observed** on the dashboard or team page (only a Leptos-internal "deprecated initialization parameters" warning, unrelated to Story 6.4).

---

## Gate Decision Update Recommendation

After the code-review patch, keep the story in `review` and rerun the browser checks below before setting a clean gate.

```yaml
# Suggested update for _bmad-output/test-artifacts/traceability/6-4-gate-decision.json
gate_status: CONCERNS
manual_browser_status: review_patched_reverify_required
p0_status: PASS_WITH_REVERIFY_REQUIRED
p1_status: PASS_WITH_REVERIFY_REQUIRED
overall_status: REVIEW_PATCHED
rationale_appendix: |
  Code-review patches changed AC#3 deep-link timing/error behavior and dashboard
  range validation after the initial manual pass. Rerun AC#2/AC#3 browser checks
  before promoting Story 6.4.
next_steps:
  - 'Rerun Department Head browser verification for patched AC#2/AC#3 paths'
  - 'Then rerun code review for REVIEW_CLEAN'
```

The cross-story frontend harness advisory remains open as a **separate backlog item**, not a Story 6.4 blocker.

---

## Sign-Off

| AC  | Coverage (logic) | Coverage (live UX) | Verdict |
|-----|------------------|--------------------|---------|
| #1  | FULL (18 INT + 25 FE unit) | ✅ Live verified | PASS |
| #2  | FULL (4 BE unit + INT + FE unit) | ⚠️ Reverify patched range validation / Next 30 Days evidence | REVIEW_PATCHED |
| #3  | PARTIAL → upgraded | ⚠️ Reverify patched direct deep-link and bounded-error behavior | REVIEW_PATCHED |
| #4  | FULL (1 INT + FE unit) | ✅ Gauge + 4 fields + health color verified | PASS |

**Overall:** ⚠️ **REVIEW_PATCHED — keep Story 6.4 in `review` until patched AC#2/AC#3 browser checks pass.**

**Generated:** 2026-05-22
**Driver:** Claude `/browse` headless Chromium, gstack v1.x
**Skill:** testarch-trace (manual verification follow-up to validation mode)

<!-- Powered by BMAD-CORE™ -->
