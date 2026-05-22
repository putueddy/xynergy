---
story: '6-2-real-time-dashboard-updates'
stepsCompleted:
  - 'step-01-preflight-and-context'
  - 'step-02-identify-targets'
  - 'step-03-generate-tests (sequential mode)'
  - 'step-04-validate-and-summarize'
  - 'step-05-expansion-run-2 (2026-05-22)'
lastStep: 'step-05-expansion-run-2'
lastSaved: '2026-05-22'
mode: 'BMad-Integrated'
executionMode: 'sequential (inline generation, no subagent dispatch)'
detectedStack: 'fullstack'
testStackType: 'auto -> fullstack (Rust workspace + Leptos frontend)'
inputDocuments:
  - '_bmad-output/implementation-artifacts/6-2-real-time-dashboard-updates.md'
  - '_bmad-output/implementation-artifacts/sprint-status.yaml'
  - '_bmad-output/project-context.md'
  - '_bmad-output/test-artifacts/traceability/6-2-real-time-dashboard-updates-traceability.md'
  - 'src/frontend/src/pages/dashboard.rs'
  - 'src/frontend/Cargo.toml'
  - 'src/backend/tests/dashboard_tests.rs'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/test-levels-framework.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/test-priorities-matrix.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/data-factories.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/test-quality.md'
---

# Test Automation Expansion — Story 6.2 Real-Time Dashboard Updates

## 1. Preflight & Context

### Stack Detection

- `test_stack_type: auto` → resolved to **fullstack** (Rust workspace `Cargo.toml` + Leptos `src/frontend/Cargo.toml` + Tailwind `src/frontend/package.json`).
- No browser test indicators (`playwright.config.*`, `cypress.config.*`) — same posture as Story 6.1.
- Profile: **API/backend integration + frontend native unit tests** (the change-detection helpers are pure functions usable from non-WASM targets).

### Execution Mode

**BMad-Integrated.** Story `6-2-real-time-dashboard-updates.md` is in `review` state with every implementation task checked off (tasks 1–7). Backend was intentionally untouched per the story scope; the entire delta lives in `src/frontend/src/pages/dashboard.rs` plus a new `dashboard-change-flash` Tailwind utility. The existing implementation already shipped 5 native unit tests for the change-detection helper. Goal of this expansion run is to widen coverage of `dashboard_value_map()` / `compute_changed_keys()` across **all** role panels and to lock in two cross-cutting invariants the story calls out explicitly (no `generated_at` highlight, no sensitive CTC fields leaked).

### Test Framework Verified

- Frontend native tests gated on `#[cfg(all(test, not(target_arch = "wasm32")))]` inside `src/frontend/src/pages/dashboard.rs::tests`.
- Run command: `cargo test -p xynergy-frontend --lib`.
- Backend route regression: `DATABASE_URL=postgres://xynergy:xynergy@localhost:5432/xynergy cargo test --package xynergy-backend --test dashboard_tests`.
- WASM compile contract: `cargo check -p xynergy-frontend --target wasm32-unknown-unknown`.

### Knowledge Loaded (core tier)

- `test-levels-framework.md` — unit-level selection for pure helpers; integration left to existing dashboard_tests.rs.
- `test-priorities-matrix.md` — P0/P1/P2 by risk × impact.
- `data-factories.md` — small in-module factory helpers (`dept_head_response_with_budget_pct`, `pm_response_with`, `finance_response_with_cash`) mirroring the style of the existing `admin_response()` helper.
- `test-quality.md` — deterministic, isolated, single-invariant assertions.

Extended/specialized fragments not loaded — no contract tests, no email/auth flows, no Playwright (no browser harness in repo).

---

## 2. Coverage Plan

### Acceptance Criteria → Test Mapping

| AC | Scenario | Test Level | Pre-existing | New |
|----|----------|-----------|--------------|-----|
| #1 30s polling | Interval lifecycle, no stacking, on_cleanup | manual browser + backend contract | 0 automated runtime tests; manual browser verified 2026-05-22 | 0 automated (manual evidence recorded in story) |
| #2 Manual refresh + last-updated | `generated_at` contract from server | INT (backend) + manual browser | 6.1-INT-018 covers `generated_at` shape and freshness; manual Refresh verified 2026-05-22 | 0 backend changes in 6.2 |
| #3 Changed-value highlight | `dashboard_value_map()` + `compute_changed_keys()` per-role keys, no `generated_at`, no sensitive fields | UNIT (frontend, non-WASM) | 5 (6.2-UNIT-001..005) | **+10 (6.2-UNIT-006..015)** |

### Existing Coverage — 5 native unit tests in `dashboard.rs`

| ID | Test | Priority |
|----|------|----------|
| 6.2-UNIT-001 | `initial_load_produces_no_changed_keys` | P1 |
| 6.2-UNIT-002 | `generated_at_only_change_produces_no_changed_keys` | P0 |
| 6.2-UNIT-003 | `scalar_change_produces_expected_key` (admin) | P1 |
| 6.2-UNIT-004 | `newly_visible_row_produces_expected_key` (hr) | P1 |
| 6.2-UNIT-005 | `removed_row_does_not_panic` | P2 |

### Expansion Coverage (this run) — 10 new tests

| ID | Test | Priority | Gap Closed |
|----|------|----------|-----------|
| 6.2-UNIT-006 | `dept_head_budget_utilization_change_produces_expected_key` | P1 | Department Head budget panel keys |
| 6.2-UNIT-007 | `dept_head_upcoming_assignment_id_change_produces_stable_key` | P1 | DH upcoming row keyed by `allocation_id` (no double-emit of name-based fallback when id is present) |
| 6.2-UNIT-008 | `project_manager_margin_pct_change_produces_expected_key` | P0 | PM project margin highlight; unchanged sibling fields stay quiet |
| 6.2-UNIT-009 | `project_manager_margin_alert_uses_project_id_stable_key` | P1 | PM margin alert keyed by `project_id` when surfacing |
| 6.2-UNIT-010 | `finance_cash_position_change_produces_expected_key` | P0 | Finance ending position + net cash flow highlights |
| 6.2-UNIT-011 | `finance_export_request_new_pending_id_produces_changed_key` | P1 | New pending export uses `id` stable key + pending_count scalar |
| 6.2-UNIT-012 | `finance_audit_alerts_total_change_produces_expected_key` | P1 | Derived audit total reacts to sub-count changes without flashing unchanged sub-counts |
| 6.2-UNIT-013 | `value_map_excludes_generated_at_key` | P0 | Contract: `generated_at` never appears as a key (polling would flash everything otherwise) |
| 6.2-UNIT-014 | `value_map_excludes_sensitive_ctc_fields` | P0 | Security: serialized map cannot leak `daily_rate`, `ciphertext`, `base_salary`, `key_version`, `encryption_*`, `encrypted_*` |
| 6.2-UNIT-015 | `fallback_key_used_when_resource_id_missing` | P2 | Stable fallback key (`hr.recent_changes.idx.{name}`) when backend omits id |

### Coverage Scope

**critical-paths.** Each of the five role panels (HR, Department Head, Project Manager, Finance, Admin) now has at least one key-generation test, and the two cross-cutting invariants the story explicitly warns about (`generated_at` exclusion, sensitive-field non-exposure) are now locked behind assertions.

### Test Level Justification

All 10 new tests are **unit** at the frontend native target. The change-detection helpers are pure `&RoleDashboardResponse -> HashMap/HashSet` functions, so unit-level coverage is the highest-fidelity option:

- **No backend tests added.** Story 6.2 scope explicitly forbids backend changes; the existing 27-test `dashboard_tests.rs` already covers the contract Story 6.2 depends on (including `generated_at` presence, sensitive-field exclusion at the route layer).
- **No E2E tests added.** Repo has no Playwright/Cypress harness. Polling interval/cleanup, request-id ordering, and CSS flash timing are WASM-only behaviours; manual browser verification was completed on 2026-05-22, but repeatable automation remains future harness work.
- **No wasm-bindgen-test runner added.** Out of scope — would require a new test harness, contradicting the "no new dependencies" rule in the story Architecture Compliance section.

---

## 3. Files Updated

- `src/frontend/src/pages/dashboard.rs` — `mod tests` expanded from 5 to 15 tests; three small factory helpers added (`dept_head_response_with_budget_pct`, `pm_response_with`, `finance_response_with_cash`) consistent with the existing `admin_response` / `empty_response` pattern.

No new files. No production code changes. No backend changes. No new dependencies.

---

## 4. Validation

### Test Execution

**Frontend native unit tests:**

```
cargo test -p xynergy-frontend --lib
```

Result: **15 passed; 0 failed; 0 ignored** (5 pre-existing + 10 new).

**Frontend WASM compile contract:**

```
cargo check -p xynergy-frontend --target wasm32-unknown-unknown
```

Result: **Finished `dev` profile** — 9 pre-existing dead-code warnings unrelated to dashboard.rs; no errors.

**Backend dashboard regression (must remain green per Story 6.2 Task 7):**

```
DATABASE_URL=postgres://xynergy:xynergy@localhost:5432/xynergy \
  cargo test --package xynergy-backend --test dashboard_tests
```

Result: **27 passed; 0 failed** — full Story 6.1 + expansion suite intact.

### Quality Checklist

- [x] All new tests deterministic (no `sleep`, no random non-seeded data — `Uuid::new_v4()` used only where the test asserts presence-by-id, not equality-by-id)
- [x] All new tests isolated (each builds its own response from `empty_response(role)`; no shared mutable state)
- [x] Each test focused on one invariant; helper factories keep noise out of assertions
- [x] No duplicate coverage with existing 5 native tests
- [x] Per-role coverage: HR (UNIT-014, UNIT-015), Department Head (UNIT-006, UNIT-007), Project Manager (UNIT-008, UNIT-009), Finance (UNIT-010, UNIT-011, UNIT-012), Admin (UNIT-013 covers admin via `admin_response`)
- [x] Cross-cutting security invariants asserted explicitly (UNIT-013 generated_at, UNIT-014 sensitive fields)
- [x] No CLI sessions opened, no browser processes spawned
- [x] Test artifacts written to `_bmad-output/test-artifacts/` only

---

## 5. Assumptions & Risks

### Assumptions

- Local PostgreSQL at `postgres://xynergy:xynergy@localhost:5432/xynergy` is the canonical test DB (matches Story 5.3 / 5.4 / 6.1 precedent).
- `dashboard_value_map()` is the authoritative source of stable keys; tests bind to its current key naming (`hr.recent_changes.{id}`, `project_manager.project.{id}.margin_pct`, etc.). If the helper renames a key, the corresponding test will fail at the assertion — surfacing the rename intentionally rather than silently drifting.
- The frontend DTOs (`RecentCtcChange`, `UpcomingAssignment`, `ProjectHealthCard`, `MarginAlert`, `PendingExportRequest`, `AuditAlertEntry`) carry `Option<Uuid>` id fields and tolerate `None` from the backend (verified by UNIT-015 and the existing fallback paths in `dashboard_value_map`).

### Risks

- **Low** — UNIT-014 uses substring-on-Debug format (`format!("{:?}", map)`) to scan for forbidden tokens. If a future map key legitimately contains one of the forbidden substrings (e.g. an obscure key like `daily_rate_pct`), this test would false-positive. Today no such key exists; if one is added, switch to a per-key check or rename the key.
- **Low** — UNIT-008 / UNIT-010 use `Uuid::nil()` as a stable test-only id to keep assertion strings deterministic. Production code does not emit `Uuid::nil()` from the backend; tests do not rely on production guarantees.
- **None observed** — No flakiness across local runs; helpers are pure-functional.

### Out of Scope

- **Polling interval lifecycle / on_cleanup / request-id ordering** — WASM-only behaviour; no wasm-bindgen-test harness in this repo. Manual browser verification was completed on 2026-05-22; repeatable automation still requires a future frontend harness.
- **CSS flash duration (1.8s + prefers-reduced-motion variant)** — pure visual behaviour; not testable without a real browser engine. The normal flash/fade path was manually verified on 2026-05-22.
- **Network/auth retry flow on poll** — covered indirectly by existing `auth.rs::authenticated_get` behaviour; full polling network choreography is integration-level and would need an E2E harness.

---

## 6. Next Recommended Workflow

- **`/bmad-testarch-trace`** — produce a traceability matrix mapping the 15 dashboard.rs unit tests + 27 backend integration tests to Story 6.2 ACs #1–#3 and emit a quality-gate decision.
- Optionally **`/bmad-testarch-test-review`** — adversarial pass over the 10 new unit tests (Acceptance Auditor + Edge Case Hunter) before landing the expansion.
- Story 6.2 is now in `done` state; the manual browser verification previously left to Task 7 was completed on 2026-05-22 (`/dashboard` polling, manual refresh, flash duration, cleanup-on-navigation, session-expired redirect).

---

*Generated by Master Test Architect via `bmad-testarch-automate` (Create mode) — 2026-05-21.*

---

## 7. Expansion Run 2 — 2026-05-22

### Context

Run 2 was triggered after the `/bmad-testarch-trace` gate decision (`PASS_WITH_ADVISORY`, 2026-05-22) noted that change-detection logic was over-covered relative to other layers, but that **display helpers** (`format_idr`, `format_date`, `role_display`) and several **dashboard_value_map key paths** (HR pending-sample list, HR top-risks list, Dept Head top-at-risk list, PM project name-based fallback, PM active_projects.count scalar, Finance audit_alerts.recent list, Finance ctc_validation status/match_rate_pct) remained un-asserted. Story scope (frontend-only, no new dependencies, no backend changes) is preserved.

### Coverage Gaps Closed (12 new native unit tests)

| ID | Test | Priority | Gap Closed |
|----|------|----------|-----------|
| 6.2-UNIT-016 | `format_idr_handles_zero` | P2 | Pure helper sanity (no leading `-`, no grouping dots) |
| 6.2-UNIT-017 | `format_idr_groups_thousands_with_dots` | P1 | IDR display correctness for 1k / 1M / 1B |
| 6.2-UNIT-018 | `format_idr_handles_negative_amount` | P2 | Negative cash flow rendering (`Rp -50.000`) |
| 6.2-UNIT-019 | `format_date_strips_iso_time_component` | P2 | Last-updated label safety (ISO time strip + passthrough + empty input) |
| 6.2-UNIT-020 | `role_display_returns_canonical_labels` | P2 | All 5 supported roles + fallback never raw-leaks the role string |
| 6.2-UNIT-021 | `hr_pending_updates_sample_id_keyed_change` | P1 | HR pending-sample list `id`-stable key + value contract |
| 6.2-UNIT-022 | `hr_compliance_top_risks_id_keyed_row_change` | P1 | HR top-risks list `resource_id`-stable key, no name-fallback double-emit |
| 6.2-UNIT-023 | `dept_head_top_at_risk_id_stable_key` | P1 | Department Head top_at_risk list `resource_id`-stable key |
| 6.2-UNIT-024 | `pm_project_fallback_key_used_when_project_id_missing` | P2 | PM active-projects name-based fallback path when backend omits `project_id` |
| 6.2-UNIT-025 | `pm_active_projects_count_changes_when_project_added` | P1 | PM list-length scalar `active_projects.count` change |
| 6.2-UNIT-026 | `finance_audit_alerts_recent_row_uses_id_stable_key` | P1 | Finance audit recent-row `id`-stable key, no fallback double-emit |
| 6.2-UNIT-027 | `finance_ctc_validation_status_change_produces_expected_key` | P1 | CTC validation status transition + Option→Some match_rate_pct |

### Validation

```
cargo test -p xynergy-frontend --lib
# → test result: ok. 27 passed; 0 failed; 0 ignored

cargo check -p xynergy-frontend --target wasm32-unknown-unknown
# → Finished `dev` profile — 9 pre-existing dead-code warnings unrelated to dashboard.rs; no errors

DATABASE_URL=postgres://xynergy:xynergy@localhost:5432/xynergy \
  cargo test --package xynergy-backend --test dashboard_tests
# → test result: ok. 27 passed; 0 failed; 0 ignored
```

### Cumulative State After Run 2

| Surface | Tests | Last Run |
|---------|-------|----------|
| Frontend native unit (`src/frontend/src/pages/dashboard.rs::tests`) | **27** (5 from impl + 10 from Run 1 + 12 from Run 2) | 27/27 PASS |
| Backend integration (`src/backend/tests/dashboard_tests.rs`) | 27 | 27/27 PASS |
| Frontend WASM compile | n/a | clean |

### Files Updated

- `src/frontend/src/pages/dashboard.rs` — appended 12 native unit tests (IDs 6.2-UNIT-016..027) inside the existing `#[cfg(all(test, not(target_arch = "wasm32")))] mod tests` block. No production code changes; no DTO changes; no backend changes; no new dependencies.

### Coverage Posture

- **Display helpers** (`format_idr`, `format_date`, `role_display`): now FULL-covered (previously 0 tests).
- **`dashboard_value_map()` key paths**: HR ✓ (recent_changes + pending_updates.sample + compliance_alerts.top_risks + fallback), Department Head ✓ (utilization + budget + upcoming_assignments + top_at_risk), Project Manager ✓ (active_projects with id + name-fallback + count scalar + margin_alerts), Finance ✓ (cash_position + ctc_validation status & match_rate_pct + audit_alerts.total + audit_alerts.recent + export_requests + pending_count), Admin ✓ (total_active_projects). All visible business surfaces now have at least one change-detection assertion.
- **Cross-cutting invariants**: `generated_at` exclusion (UNIT-002 + UNIT-013); sensitive CTC field exclusion (UNIT-014); name-fallback / id-fallback symmetry (UNIT-015 + UNIT-024); id-stable key uniqueness without double-emitting fallback keys (UNIT-007 + UNIT-022 + UNIT-026). All FULL.

### Assumptions, Limits, and Out-of-Scope

- Polling interval lifecycle, monotonic request-id ordering at runtime, on_cleanup behavior, CSS flash duration, and `prefers-reduced-motion` variant remain WASM-runtime concerns for automation. Manual browser verification for polling, refresh, flash/fade, navigation cleanup, and session-expired redirect was completed on 2026-05-22; repeatable coverage still needs a future frontend harness.
- Run 2 did not introduce a new test framework, harness, or production dependency, preserving the Story 6.2 Architecture Compliance constraint.

---

*Run 2 generated by Master Test Architect via `bmad-testarch-automate` (Create mode) — 2026-05-22.*
