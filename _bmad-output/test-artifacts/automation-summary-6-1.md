---
story: '6-1-role-based-dashboard'
stepsCompleted:
  - 'step-01-preflight-and-context'
  - 'step-02-identify-targets'
  - 'step-03-generate-tests (sequential mode)'
  - 'step-04-validate-and-summarize'
lastStep: 'step-04-validate-and-summarize'
lastSaved: '2026-05-21'
mode: 'BMad-Integrated'
executionMode: 'sequential (inline generation, no subagent dispatch)'
detectedStack: 'fullstack'
testStackType: 'auto -> fullstack (Rust workspace + Leptos frontend)'
inputDocuments:
  - '_bmad-output/implementation-artifacts/6-1-role-based-dashboard.md'
  - '_bmad-output/implementation-artifacts/sprint-status.yaml'
  - '_bmad-output/project-context.md'
  - 'src/backend/src/routes/dashboard.rs'
  - 'src/backend/src/services/dashboard_service.rs'
  - 'src/backend/tests/dashboard_tests.rs'
  - 'migrations/20260130111339_initial_schema.sql'
  - 'migrations/20260222133000_audit_hash_chain.up.sql'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/test-levels-framework.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/test-priorities-matrix.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/data-factories.md'
  - '.claude/skills/bmad-testarch-automate/resources/knowledge/test-quality.md'
---

# Test Automation Expansion - Story 6.1 Role-Based Dashboard

## 1. Preflight & Context

### Stack Detection

- `test_stack_type: auto` → resolved to **fullstack** (Rust workspace `Cargo.toml` + Leptos `src/frontend/Cargo.toml` + Tailwind `src/frontend/package.json`)
- No browser test indicators (`playwright.config.*`, `cypress.config.*`) found
- Profile: **API/backend-only** (matches Story 5.3 / 5.4 precedent)

### Execution Mode

**BMad-Integrated.** Story `6-1-role-based-dashboard.md` is in `review` state with all eight tasks checked off; backend route, service, frontend page, and a 10-test integration suite at `src/backend/tests/dashboard_tests.rs` are already shipped. Goal of this expansion run is to widen coverage on data isolation, list bounds, empty-state, and security-boundary cases that the initial pass did not exercise.

### Test Framework Verified

- Pattern: `#[sqlx::test(migrations = "../../migrations")]` (axum router exercised via `tower::ServiceExt::oneshot`)
- JSON body assertions via `serde_json::Value`
- Test DB: `postgres://xynergy:xynergy@localhost:5432/xynergy` (live execution confirmed for this run)
- Required env vars set inside test bodies via `set_test_env()`: `JWT_SECRET`, `CTC_ACTIVE_KEY_VERSION`, `CTC_ENCRYPTION_KEY_V1`

### Knowledge Loaded (core tier)

- `test-levels-framework.md` — integration test selection for route/service composition
- `test-priorities-matrix.md` — P0/P1/P2 assignment by risk × impact
- `data-factories.md` — factory pattern (already followed by existing helpers in `dashboard_tests.rs`)
- `test-quality.md` — deterministic, isolated, bounded scope per test

Extended/specialized fragments not loaded — no UI tests, no contract tests, no email-auth flows in scope.

---

## 2. Coverage Plan

### Acceptance Criteria → Test Mapping

| AC | Scenario | Pre-existing | New | Level |
|----|----------|--------------|-----|-------|
| #1 HR | role section, no sensitive fields, missing-CTC sample bound | 2 | +1 | INT |
| #2 Dept Head | department scoping, utilization/overalloc, orphan claims, past allocations | 2 | +2 | INT |
| #3 PM | ownership filter, empty state, status filter | 1 | +2 | INT |
| #4 Finance | cash + audit + export + no-data, audit signal flow | 2 | +1 | INT |
| Cross | unauth, unsupported role, admin section, malformed token, generated_at contract | 3 | +2 | INT |

### Existing Coverage (Story 6.1, initial pass) — 10 tests

| ID | Test | Priority | AC |
|----|------|----------|----|
| 6.1-INT-001 | `unauthenticated_dashboard_returns_401` | P0 | All |
| 6.1-INT-002 | `hr_dashboard_returns_hr_section_only` | P0 | #1 |
| 6.1-INT-003 | `hr_recent_changes_do_not_expose_encrypted_fields` | P0 | #1 |
| 6.1-INT-004 | `dept_head_only_sees_own_department` | P0 | #2 |
| 6.1-INT-005 | `dept_head_response_has_utilization_and_overallocation` | P1 | #2 |
| 6.1-INT-006 | `project_manager_only_sees_owned_projects` | P0 | #3 |
| 6.1-INT-007 | `finance_dashboard_has_cash_and_audit_state` | P1 | #4 |
| 6.1-INT-008 | `finance_validation_returns_no_data_when_no_payroll` | P1 | #4 |
| 6.1-INT-009 | `admin_dashboard_returns_admin_section` | P1 | Admin |
| 6.1-INT-010 | `unsupported_role_returns_403` | P0 | All |

### Expansion Coverage (this run) — 8 new tests

| ID | Test | Priority | AC | Gap Closed |
|----|------|----------|----|-----------|
| 6.1-INT-011 | `dept_head_without_department_returns_403` | P0 | #2 | Security: DH JWT without `department_id` must not silently default to all-departments |
| 6.1-INT-012 | `project_manager_with_no_projects_returns_empty_state` | P1 | #3 | Empty-state correctness — empty arrays, no panic, no `null` cards |
| 6.1-INT-013 | `project_manager_excludes_non_active_projects` | P0 | #3 | Status filter — `Completed`/`Closed`/`Cancelled` projects must not appear |
| 6.1-INT-014 | `hr_pending_updates_sample_bounded_to_limit` | P1 | #1 | Bound enforcement — `sample.len()` capped at `MISSING_CTC_SAMPLE_LIMIT=5` while `missing_count` reflects full population |
| 6.1-INT-015 | `dept_head_upcoming_excludes_past_allocations` | P1 | #2 | Time-window correctness — past `end_date` allocations excluded from "upcoming" |
| 6.1-INT-016 | `dashboard_response_includes_recent_generated_at` | P2 | All | Contract guarantee — `generated_at` present and within the request window, parseable as RFC3339 UTC |
| 6.1-INT-017 | `finance_audit_alerts_count_recent_security_events` | P1 | #4 | Real audit signal — seeded `ACCESS_DENIED` / `LOGIN_FAILED` / `LOGIN_BLOCKED` rows surface in counts and `recent` list; allow-list enforced |
| 6.1-INT-018 | `invalid_bearer_token_returns_401` | P1 | All | Security boundary — malformed JWT rejected, not silently treated as anonymous |

### Coverage Scope

**critical-paths.** Targeted at the security and correctness invariants the initial 10-test suite did not exercise:

- Security boundary (orphan DH claim, malformed token)
- Data isolation (status filter on PM dashboard)
- List bounds (HR sample capped)
- Time-window correctness (DH upcoming excludes past)
- Audit signal flow (Finance audit alerts populated from real `audit_logs` rows)
- Response contract (`generated_at` shape and freshness)
- Empty-state robustness (PM with no projects)

### Test Level Justification

All eight tests are **integration** because each exercises the full route → service → SQL → response stack, including JWT extraction, role gating, and DB-backed scoping. Unit tests on `build_dashboard` would not surface the JWT/HTTP boundary regressions these guard against, and there is no UI E2E framework wired up for this repo, so a backend integration test is the highest-fidelity level available.

---

## 3. Files Updated

- `src/backend/tests/dashboard_tests.rs` (added 8 tests + 2 helper fns: `insert_audit_log`, `create_project_with_status`)

No new files. No frontend changes. No production code changes — tests only.

---

## 4. Validation

### Test Execution (live DB)

```
DATABASE_URL=postgres://xynergy:xynergy@localhost:5432/xynergy \
  cargo test --package xynergy-backend --test dashboard_tests
```

Result: **18 passed; 0 failed** (10 pre-existing + 8 new) — finished in 4.82s.

### Regression Suites (high-risk, story-required)

| Suite | Result |
|-------|--------|
| `team_tests` | 10/10 passed |
| `project_pl_tests` | 10/10 passed |
| `cash_flow_dashboard_tests` | 28/28 passed |
| `ctc_validation_report_tests` | 36/36 passed |
| `compliance_audit_report_tests` | 48/48 passed |

No regressions. Pre-existing `sqlx-postgres` future-incompat warning is unrelated and was present before this expansion.

### Quality Checklist

- [x] All new tests deterministic (no `sleep`, no random data without controlled seeds)
- [x] All new tests isolated (`#[sqlx::test]` per-test transactional DB, unique emails via `Uuid::new_v4`)
- [x] Each test under ~50 lines, focused on one invariant
- [x] No duplicate coverage with existing 10 tests
- [x] Helpers (`insert_audit_log`, `create_project_with_status`) reuse the existing factory style
- [x] Negative/security cases assert explicit status codes and error payloads (no silent passes)
- [x] Bounded list assertions check both count and contents
- [x] No CLI sessions opened, no orphaned browser instances
- [x] Test artifacts written to `_bmad-output/test-artifacts/` only

---

## 5. Assumptions & Risks

### Assumptions

- Local PostgreSQL at `postgres://xynergy:xynergy@localhost:5432/xynergy` is the canonical test DB (matches all other story automation runs).
- The `audit_logs` insert path is open for direct test seeding (the append-only trigger added in `20260222133000_audit_hash_chain.up.sql` blocks UPDATE/DELETE only — INSERTs without `entry_hash` are accepted).
- `Cancelled` / `Closed` / `Completed` are realistic non-active project statuses worth excluding; the production SQL filter accepts only `Active`/`active`/`planning`/`Planning`.

### Risks

- **Low** — Test 6.1-INT-016 allows a 5-second slack on either side of the request window for `generated_at` to absorb clock drift between the harness and the server task. If a future change moves `Utc::now()` outside `build_dashboard`, the slack may need to widen.
- **Low** — Test 6.1-INT-017 seeds plain `audit_logs` rows without hash-chain backfill. If a future migration changes `audit_logs` to require `entry_hash NOT NULL`, the helper must be updated; today the column is nullable.
- **None observed** — No flakiness across local runs.

### Out of Scope

- Frontend E2E coverage of dashboard rendering — no Playwright/Cypress harness exists in this repo.
- 30-second polling behavior — explicitly deferred to Story 6.2 per the story doc.
- Cross-role data leakage permutations beyond what existing tests already enforce — current suite asserts the four `is_null()` invariants per role.

---

## 6. Next Recommended Workflow

- **`/bmad-testarch-trace`** — produce a traceability matrix mapping the 18 dashboard tests to ACs #1-#4 and the security/contract invariants, and emit a quality-gate decision for Story 6.1.
- Optionally **`/bmad-testarch-test-review`** — adversarial pass over the new 8 tests (Acceptance Auditor + Edge Case Hunter) before merging the expansion.

---

*Generated by Master Test Architect via `bmad-testarch-automate` (Create mode) — 2026-05-21.*
