---
stepsCompleted:
  - step-01-load-context
  - step-02-discover-tests
  - step-03-apply-rubric
  - step-04-produce-report
lastStep: step-04-produce-report
lastSaved: '2026-05-21'
storyId: '6.1'
storyKey: 6-1-role-based-dashboard
reviewScope: single-file
reviewTarget: src/backend/tests/dashboard_tests.rs
testStackType: backend
testFramework: 'cargo test + sqlx::test + tower::oneshot'
totalTests: 18
totalHelpers: 16
fileLineCount: 873
qualityGate: PASS
inputDocuments:
  - .claude/skills/bmad-testarch-test-review/resources/tea-index.csv
  - .claude/skills/bmad-testarch-test-review/resources/knowledge/test-quality.md
  - .claude/skills/bmad-testarch-test-review/resources/knowledge/data-factories.md
  - _bmad-output/project-context.md
  - _bmad-output/implementation-artifacts/6-1-role-based-dashboard.md
  - _bmad-output/test-artifacts/traceability/6-1-role-based-dashboard-traceability.md
  - _bmad-output/test-artifacts/traceability/6-1-gate-decision.json
  - src/backend/tests/dashboard_tests.rs
---

# Test Quality Review — Story 6.1 Dashboard Tests

**Reviewer role:** Master Test Architect
**Target file:** [src/backend/tests/dashboard_tests.rs](src/backend/tests/dashboard_tests.rs) (873 lines, 18 tests, 16 helpers)
**Stack:** Rust 1.75+ / Axum 0.7 / sqlx 0.7 / tower / `#[sqlx::test]` per-test PgPool
**Traceability gate:** PASS_WITH_ADVISORY (4/4 P0 ACs FULL covered)

## Quality Gate: ✅ **PASS**

The suite passes the Master Test Architect quality bar with no blocking findings. The 18 tests demonstrate strong test-architect discipline: explicit data-isolation tests, sensitive-field allow-listing, list-bound enforcement, empty-state assertions, and security-boundary coverage. Findings below are LOW-priority refinements.

---

## 1. Context Snapshot

| Item | Value |
|---|---|
| Review scope | single file |
| AUT layer | Backend integration (Axum router → sqlx PostgreSQL via real migrations) |
| Test driver | `#[sqlx::test(migrations = "../../migrations")]` + `tower::ServiceExt::oneshot` |
| Isolation model | Fresh PgPool per test (sqlx::test fixture) — fully parallel-safe |
| Auth strategy | Real `/api/v1/auth/login` round-trip per test → captured Bearer token |
| Crypto in setup | Real `DefaultCtcCryptoService` (AES-256-GCM) for CTC seeding |

**Test sections (8):** Authentication · HR · Department Head · Project Manager · Finance · Admin · Unsupported role · Story 6.1 expansion coverage.

---

## 2. Definition-of-Done Scorecard

| # | Criterion | Verdict | Evidence |
|---|---|---|---|
| 1 | No hard waits (`sleep` / `waitForTimeout`) | ✅ PASS | No `tokio::time::sleep` in the file. The 5-second `slack` in `dashboard_response_includes_recent_generated_at` ([dashboard_tests.rs:793](src/backend/tests/dashboard_tests.rs:793)) is a *bounded clock-drift tolerance*, not a wait — acceptable. |
| 2 | No flow-control conditionals | ✅ PASS | Only one `if let Some(t) = token` in `get_dashboard` ([dashboard_tests.rs:219](src/backend/tests/dashboard_tests.rs:219)) — that's parameter handling, not assertion branching. |
| 3 | Each test < 300 lines | ✅ PASS | Longest single test ≈ 55 lines (`dept_head_only_sees_own_department` at [dashboard_tests.rs:328-379](src/backend/tests/dashboard_tests.rs:328)). File total of 873 lines is fine — the DoD limit is *per test*. |
| 4 | Per-test runtime < 1.5 min | ⚠️ ADVISORY | Each test is individually fast, but `#[sqlx::test]` runs the full migration set + the CTC helper does real AES-256-GCM. Across 18 tests this multiplies. See Finding F1. |
| 5 | Self-cleaning | ✅ PASS | `#[sqlx::test]` gives each test its own DB instance; no cross-test pollution possible. |
| 6 | Explicit assertions (no hidden `expect()` in helpers) | ✅ PASS | All `assert_eq!` / `assert!` live in test bodies. Helpers return `Uuid` / `String` / tuples; assertions stay above. |
| 7 | Unique test data | ✅ PASS | `test_email(prefix)` uses `Uuid::new_v4()` ([dashboard_tests.rs:19](src/backend/tests/dashboard_tests.rs:19)). Even department names that repeat (`"Engineering"`) are safe due to per-test DB. |
| 8 | Parallel-safe | ✅ PASS | Per-test PgPool — strongest possible isolation. |

---

## 3. Data-Factories Scorecard

| # | Criterion | Verdict | Evidence |
|---|---|---|---|
| F-1 | Factory functions with sensible defaults & explicit overrides | ✅ PASS | `create_user_with_role`, `create_department`, `create_resource_in_dept`, `create_project_with_pm`, `create_allocation`, `create_ctc_for_resource`, `add_ctc_revision`, `insert_cash_flow_entry`, `insert_export_request`, `insert_audit_log` — all single-purpose with required IDs as parameters. |
| F-2 | API/DB setup (no UI for setup) | ✅ PASS | All setup via direct `sqlx::query!` against PgPool. The only HTTP call in setup is `login_token` — required to obtain a JWT. |
| F-3 | Schema-evolution-friendly | ✅ PASS | Factories use SQL fragments tied to current schema; column additions require helper updates in one place. |
| F-4 | No hardcoded production-collision IDs | ✅ PASS | All IDs UUID-generated. No `id: 1` antipatterns. |
| F-5 | Composition / specialization | ⚠️ MINOR | `create_project_with_pm` ([dashboard_tests.rs:124](src/backend/tests/dashboard_tests.rs:124)) and `create_project_with_status` ([dashboard_tests.rs:578](src/backend/tests/dashboard_tests.rs:578)) overlap. See Finding F2. |

---

## 4. Test-Levels Framework

- **Selected level:** integration (API + DB through full Axum router). ✅ correct choice — the unit under test is *role-scoping behavior across handler + service + SQL filter*, which is end-to-end-by-design.
- **Unit-level gap:** `dashboard_service.rs` has zero inline `#[cfg(test)] mod tests`. Per the traceability doc, this is acceptable because all logic is reachable through API tests. **No action required** — but if a pure helper (e.g., `MISSING_CTC_SAMPLE_LIMIT` slicing, date-window math) ever grows non-trivial, add a fast unit test alongside it.

---

## 5. Strengths Worth Preserving

1. **Test-ID dividers in source.** `// ── 6.1-INT-011 Security: DH without department_id → 403 ──` style headers ([dashboard_tests.rs:597-868](src/backend/tests/dashboard_tests.rs:597)) provide first-class traceability hooks. Keep this convention in future stories.
2. **Sensitive-field allow-list pattern.** `hr_recent_changes_do_not_expose_encrypted_fields` ([dashboard_tests.rs:289-323](src/backend/tests/dashboard_tests.rs:289)) iterates an explicit blocklist (`encrypted_components`, `encrypted_daily_rate`, `ciphertext`, `key_version`, `encryption_algorithm`, `encryption_version`, `base_salary`, `daily_rate`). This is exemplary for any future endpoint touching encrypted data.
3. **Audit-action allow-list in `finance_audit_alerts_count_recent_security_events`** ([dashboard_tests.rs:846-857](src/backend/tests/dashboard_tests.rs:846)) — defends the documented contract by *failing if an unexpected action appears*, not just by counting expected ones. Strong contract-testing posture.
4. **Negative-path completeness.** Auth gate (401), malformed bearer (401), unsupported role (403), DH without department (403) — all four failure modes explicit.
5. **Empty-state assertions.** `project_manager_with_no_projects_returns_empty_state` and `finance_validation_returns_no_data_when_no_payroll` prevent the regression where an empty list becomes a hard error.
6. **Cross-section leak guards.** Every role-positive test also asserts the *other* role sections are `null` (`assert!(body["finance"].is_null())` etc.). This catches accidental cross-role data exposure in one assertion.

---

## 6. Findings & Recommendations

### F1 — Setup cost from `#[sqlx::test]` × CTC encryption (ADVISORY / LOW)

**Where:** `create_ctc_for_resource` ([dashboard_tests.rs:81-110](src/backend/tests/dashboard_tests.rs:81)) instantiates `DefaultCtcCryptoService` and runs two real AES-256-GCM encryptions per call. Six tests invoke it.

**Why it matters:** When `#[sqlx::test]` runs the migration suite per test (18 fresh DBs) and each CTC-touching test does extra crypto, the file's wall-clock cost grows superlinearly. Not yet a DoD violation, but a regression risk as Story 6.x grows the suite.

**Suggested action:** For tests that only need the *presence* of a CTC row (not the encrypted payload integrity), introduce a lightweight `seed_minimal_ctc(pool, resource_id, user_id)` helper that writes literal `'ciphertext-redacted'` like `add_ctc_revision` already does ([dashboard_tests.rs:113-122](src/backend/tests/dashboard_tests.rs:113)). Reserve `create_ctc_for_resource` for tests that meaningfully exercise the encrypted path. **Priority:** LOW.

### F2 — Two project factories with overlapping responsibility (LOW)

**Where:**
- `create_project_with_pm(pool, name, pm_id)` ([dashboard_tests.rs:124](src/backend/tests/dashboard_tests.rs:124)) — defaults `status='Active'`, dates `CURRENT_DATE..CURRENT_DATE+90`.
- `create_project_with_status(pool, name, pm_id, status)` ([dashboard_tests.rs:578](src/backend/tests/dashboard_tests.rs:578)) — accepts status, widens window to `-60..+60`.

**Suggested action:** Collapse into a single factory with an optional status (or a `ProjectOpts` struct with defaults). This is the `createUser({ role: 'admin' })` pattern from `data-factories.md`. **Priority:** LOW.

### F3 — Overly permissive validation_status check in `finance_dashboard_has_cash_and_audit_state` ([dashboard_tests.rs:485-490](src/backend/tests/dashboard_tests.rs:485)) (LOW)

```rust
let validation_status = finance["ctc_validation"]["status"].as_str().unwrap();
assert!(
    matches!(validation_status, "no_data" | "ok" | "error"),
    "unexpected validation status: {}",
    validation_status
);
```

The test seeds no payroll → the status is deterministically `"no_data"`. The 3-way `matches!` softens the assertion *and* duplicates what `finance_validation_returns_no_data_when_no_payroll` already proves.

**Suggested action:** either tighten to `assert_eq!(validation_status, "no_data")`, or drop the assertion entirely from this test since the sibling test owns that contract. **Priority:** LOW.

### F4 — `set_test_env()` called at the top of every test (NIT)

**Where:** every `#[sqlx::test]` body starts with `set_test_env();` ([dashboard_tests.rs:10-17](src/backend/tests/dashboard_tests.rs:10)).

**Suggested action:** acceptable as-is (cheap, explicit, parallel-safe). If repetition ever bothers a reader, a `ctor::ctor` or `std::sync::Once` could centralize it — but env-vars set inside async tests are notoriously fiddly under cargo test threading, so the current pattern is defensible. **Priority:** NIT only.

### F5 — Document the inline `'ciphertext-redacted'` choice (NIT)

`add_ctc_revision` writes literal strings into `encrypted_components` / `encrypted_daily_rate` ([dashboard_tests.rs:113-122](src/backend/tests/dashboard_tests.rs:113)). This is safe because the dashboard endpoint never decrypts these rows — but a future reader might mistake it for a bug.

**Suggested action:** one-line comment above the helper: `// dashboard endpoint never decrypts these; literal placeholders avoid the AES round-trip`. **Priority:** NIT.

### F6 — Cross-AC leak guards exist but only check `is_null()` (OBSERVATION)

E.g. `assert!(body["hr"].is_null());` ([dashboard_tests.rs:375](src/backend/tests/dashboard_tests.rs:375)). This catches the "section populated" case. It does *not* catch a regression where the response shape gains a *new* sibling section that leaks data — because new fields would not be `null`-checked.

**Suggested action:** none required — current coverage matches the documented contract. If `RoleDashboardResponse` ever gains a new role section, the corresponding tests will need new `is_null()` lines. Worth a one-line comment on the response struct: `// Adding a new section here? Update all role-isolation tests in dashboard_tests.rs.`. **Priority:** OBSERVATION.

---

## 7. Cross-Reference With Traceability Gate

The Story 6.1 traceability report ([6-1-role-based-dashboard-traceability.md](_bmad-output/test-artifacts/traceability/6-1-role-based-dashboard-traceability.md)) issued **PASS_WITH_ADVISORY** with three carried items, none of which are addressable inside `dashboard_tests.rs`:

| Advisory item | Owner |
|---|---|
| Frontend `/dashboard` has no automated E2E / component coverage | Future Leptos test-harness story |
| 30-second polling deferred to Story 6.2 | Story 6.2 |
| `login.rs::role_dashboard_path()` has no automated frontend test | Future frontend harness |

This review confirms the *backend* test file itself has no quality-of-DoD reason to block the gate.

---

## 8. Action Items Summary

| ID | Priority | Action | File / Location |
|---|---|---|---|
| F1 | LOW | Add lightweight CTC seed helper to avoid AES per test | `dashboard_tests.rs:81` |
| F2 | LOW | Merge `create_project_with_pm` + `create_project_with_status` into one factory | `dashboard_tests.rs:124,578` |
| F3 | LOW | Tighten or remove the 3-way `matches!` on `ctc_validation.status` | `dashboard_tests.rs:486` |
| F4 | NIT | Optional: deduplicate `set_test_env()` via `Once` (defer until pain appears) | every test |
| F5 | NIT | One-line comment on `add_ctc_revision` explaining placeholder ciphertext | `dashboard_tests.rs:112` |
| F6 | OBS | Add a "update isolation tests on shape change" comment on `RoleDashboardResponse` | `routes/dashboard.rs` (response struct) |

**None of the above are required to merge or release Story 6.1.** They are LOW/NIT refinements for future hygiene.

---

## 9. Verdict

```
🚨 TEST-REVIEW GATE: PASS

📊 Quality Scorecard:
  Test Quality DoD:        8/8 PASS (1 advisory on cumulative runtime)
  Data Factories:          5/5 PASS (1 minor consolidation)
  Test Levels:             OK (integration is the right level)
  Negative-path coverage:  STRONG (401, 401-malformed, 403-role, 403-orphan-DH)
  Security boundaries:     STRONG (encrypted-field allow-list + audit-action allow-list)
  Isolation guarantees:    STRONG (#[sqlx::test] per-test DB)

✅ Decision:
  Tests are deterministic, isolated, explicit, focused, and parallel-safe.
  18/18 cases trace cleanly to Story 6.1 ACs and supporting contracts.
  Findings are LOW/NIT — none block landing.

📂 Traceability gate (parallel artefact): PASS_WITH_ADVISORY
📂 This file: _bmad-output/test-artifacts/test-reviews/6-1-dashboard-tests-review.md
```
