# Story 3.1: Team View with Blended Rates

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **Department Head**,
I want **to see my team members with their blended daily rates**,
so that **I can make informed assignment decisions without seeing sensitive CTC components**.

## Acceptance Criteria

1. **Given** I am logged in as Department Head **when** I navigate to "My Team" **then** I see a list of employees in my department with: Name, Role, Current Allocations, Blended Daily Rate.
2. **Given** I view the team list **when** I look at an employee's daily rate **then** I see the blended rate (calculated from CTC) **and** I do NOT see base salary, allowances, or BPJS components.
3. **Given** an employee has no CTC data **when** I view the team list **then** they are marked with "CTC Missing" status **and** I cannot assign them to projects until CTC is complete.

## Tasks / Subtasks

- [x] **Task 1: Create team view backend endpoint** (AC: #1, #2, #3)
  - [x] 1.1 Create `GET /api/v1/team` endpoint in new file `src/backend/src/routes/team.rs`
  - [x] 1.2 Endpoint returns `Vec<TeamMemberResponse>` with fields: `resource_id`, `name`, `role`, `department_name`, `daily_rate: Option<i64>` (IDR whole number), `ctc_status` ("Active" | "Missing"), `total_allocation_pct: f64`, `active_assignments: Vec<AssignmentSummary>`
  - [x] 1.3 `AssignmentSummary` includes: `project_name`, `allocation_pct`, `start_date`, `end_date`
  - [x] 1.4 Query joins `resources` → LEFT JOIN `ctc_records` (status='Active') → LEFT JOIN `allocations` → LEFT JOIN `projects`. Use single query with JOINs (no N+1)
  - [x] 1.5 **CRITICAL: Never expose CTC component values** — only return `daily_rate` from `ctc_records.daily_rate` column. The query must NOT select base_salary, allowances, bpjs_*, thr_* columns
  - [x] 1.6 CTC data is encrypted at rest. `daily_rate` is stored as DECIMAL(12,2) in plaintext (not encrypted) — it is a derived value, safe for non-HR consumers. Verify this by checking `ctc_records` schema. If `daily_rate` IS encrypted, decrypt it via `ctc_crypto.rs` service layer, then convert to i64
  - [x] 1.7 Filter by department using RLS: set `app.current_department_id` session variable (existing pattern from `rls_department_isolation.up.sql`). Department Head only sees own department; HR sees all
  - [x] 1.8 Authorization: Department Head + HR + Admin roles. Use `require_role()` middleware pattern. Extract user ID via `user_id_from_headers(&headers)?`
  - [x] 1.9 Calculate `total_allocation_pct` by summing active allocations (where `end_date >= CURRENT_DATE` or end_date is NULL)
  - [x] 1.10 Register route in `routes/mod.rs` and wire in `lib.rs` under `/api/v1/team`
  - [x] 1.11 Add audit log entry for team view access: action=`team_view`, entity_type=`department`

- [x] **Task 2: Create blended rate service** (AC: #2)
  - [x] 2.1 Create `src/backend/src/services/team_service.rs` — thin service layer for team queries
  - [x] 2.2 `get_team_members(pool, department_id, user_role) → Result<Vec<TeamMember>>` function
  - [x] 2.3 Export via `services/mod.rs`
  - [x] 2.4 Convert `daily_rate` BigDecimal → i64 using existing `bd_to_i64()` helper (from Story 2.3 — returns `Result`, not silent 0 fallback)
  - [x] 2.5 For employees without CTC: `daily_rate = None`, `ctc_status = "Missing"`
  - [x] 2.6 Sort result by employee name (alphabetical) by default

- [x] **Task 3: Add CTC-missing assignment guard enhancement** (AC: #3)
  - [x] 3.1 Story 2.4 already added CTC-required guard in `POST /api/v1/allocations` (allocation.rs). **Verify** this guard exists and works — do NOT re-implement
  - [x] 3.2 The team view endpoint must include `ctc_status` field so the frontend can disable the "Assign" button for CTC-missing employees
  - [x] 3.3 Add integration test: attempt `POST /allocations` for resource without CTC → verify 400 rejection still works (regression test)

- [x] **Task 4: Build "My Team" frontend page** (AC: #1, #2, #3)
  - [x] 4.1 Create `src/frontend/src/pages/team.rs` — Department Head team dashboard
  - [x] 4.2 **Layout** (Stripe Financial design direction):
    - Summary cards row: Total Team Members | Avg Daily Rate | Total Allocation % | CTC Missing Count
    - Data table: columns = Name | Role | Daily Rate (Rp) | Current Allocation % | Projects | Status
    - Status badge: green "Active" (has CTC) or red "CTC Missing" (no CTC)
  - [x] 4.3 **Daily Rate display**: Format as IDR with thousand separators using JetBrains Mono font, right-aligned. Example: `Rp 1,200,000`. Employees without CTC show `—` dash
  - [x] 4.4 **Current Allocation**: Show total % with color coding: green (≤80%), yellow (81-99%), red (100%), dark red (>100% overallocated)
  - [x] 4.5 **Projects column**: Comma-separated project names with allocation % each. Tooltip on hover shows full assignment details (project name, dates, %)
  - [x] 4.6 **CTC Missing row styling**: Muted/grayed text, "Assign" button disabled with tooltip "CTC data required — contact HR"
  - [x] 4.7 Wire to `GET /api/v1/team` endpoint via `spawn_local` + `reqwest` pattern
  - [x] 4.8 Use Leptos signals for state: `create_signal(Vec::<TeamMemberResponse>::new())` for team data, `create_resource` for async fetch
  - [x] 4.9 Follow existing component patterns from `ctc_completeness.rs` (612 lines) — summary cards + data table layout
  - [x] 4.10 Add page route `/team` in `lib.rs`
  - [x] 4.11 Add "My Team" navigation link in `components/mod.rs` — visible for DeptHead + HR + Admin roles (follow existing nav guard pattern)

- [x] **Task 5: Add sorting and filtering to team view** (AC: #1)
  - [x] 5.1 Client-side sorting by: Name (default), Daily Rate, Allocation %, Status
  - [x] 5.2 Client-side filter by: All | Active (has CTC) | CTC Missing
  - [x] 5.3 Implement as Leptos signals reacting to dropdown/button state changes

- [x] **Task 6: Integration and unit tests** (AC: #1, #2, #3)
  - [x] 6.1 **Backend integration tests** in `src/backend/tests/team_tests.rs`:
    - `GET /team` as DeptHead → returns only own department employees
    - `GET /team` as HR → returns employees from all departments (or filtered)
    - `GET /team` as PM → 403 Forbidden
    - `GET /team` response includes `daily_rate` for employees with CTC
    - `GET /team` response shows `ctc_status: "Missing"` for employees without CTC
    - `GET /team` response does NOT include `base_salary`, `allowances`, `bpjs_*`, `thr_*` fields (security assertion)
    - Verify `total_allocation_pct` is correctly calculated
  - [x] 6.2 **Unit tests** in `team_service.rs`:
    - `bd_to_i64` conversion of daily rate
    - Team member sorting
  - [x] 6.3 Follow `#[sqlx::test(migrations = "../../migrations")]` pattern for integration tests
  - [x] 6.4 Keep all existing test suites green (80+ tests from Epics 1-2)

## Dev Notes

### Developer Context (Critical)

- **Epics 1 and 2 are fully done (10 stories complete).** This is the FIRST story in Epic 3 "Department Resource Assignment". You are building the foundation that Stories 3.2–3.5 will extend (assignment interface, cost preview, overallocation, budget utilization).
- **The CTC infrastructure is complete**: `ctc_records` table has `daily_rate` DECIMAL(12,2) column (calculated as `total_monthly_ctc / working_days_per_month`). This value is the "blended rate" — it includes base salary + all allowances + BPJS employer contributions + THR accrual, divided by working days. **Department Heads see ONLY this number.**
- **CTC sensitive columns ARE encrypted** (Story 2.0): `base_salary`, `hra_allowance`, `medical_allowance`, `transport_allowance`, `meal_allowance`, `bpjs_kesehatan`, `bpjs_ketenagakerjaan`, `thr_monthly_accrual`, `total_monthly_ctc`. However, `daily_rate` may or may not be encrypted — **CHECK** the `extend_ctc_records` migration and `ctc_crypto.rs` to determine if `daily_rate` needs decryption. If it does, decrypt in service layer only.
- **RLS is active on `ctc_records`** and `resources` tables (Story 1.3). The team view query MUST set the PostgreSQL session variables `app.current_role` and `app.current_department_id` BEFORE querying, or RLS will block access. Follow the same pattern used in `ctc.rs` route handlers.
- **The allocation CTC guard already exists** (Story 2.4): `POST /api/v1/allocations` rejects resources without active CTC. Do NOT re-implement — just verify it works and reference it in the frontend (disable Assign button).
- `allocations` table has: `resource_id`, `project_id`, `percentage` (BigDecimal), `start_date`, `end_date`. Join to compute current total allocation.
- `resources` table has: `id`, `name`, `role`, `department_id`, `status`. Join to `departments` for department name.
- `projects` table has: `id`, `name`, `status`. Join to allocations for project name display.

### Technical Requirements

- Response DTO must NEVER include CTC component fields. Create a dedicated `TeamMemberResponse` struct that only exposes: `resource_id`, `name`, `role`, `department_name`, `daily_rate`, `ctc_status`, `total_allocation_pct`, `active_assignments`.
- `daily_rate` in response should be `Option<i64>` (None when CTC missing). Convert from BigDecimal using `bd_to_i64()` which returns `Result` — handle errors gracefully.
- Use a single SQL query with JOINs (no N+1). Pattern from Story 2.4: batch pre-fetch with JOINs, never loop-and-query.
- All monetary values displayed as whole IDR numbers (no decimals) — consistent with entire codebase.
- Department filtering via RLS session variables — do NOT add `WHERE department_id = $1` manually; let PostgreSQL RLS handle isolation.
- `total_allocation_pct` must be calculated from active allocations only: `WHERE (a.end_date >= CURRENT_DATE OR a.end_date IS NULL)`.
- Frontend must use Tailwind CSS 3.4 with Stripe Financial design direction: clean data tables, right-aligned monospace numbers, status badges.

### Architecture Compliance

- Keep Axum route conventions: `/api/v1/team` nested route, handler returns `Result<Json<Vec<TeamMemberResponse>>>`.
- Keep handlers thin — query logic in `team_service.rs`, handler just extracts auth, calls service, returns JSON.
- Use `AppError` error mapping; no production `unwrap()`/`expect()`.
- Maintain audit hash-chain integration with `pg_advisory_xact_lock(88889999)` serialization for audit entries.
- Error responses follow existing format: `{ "error": "...", "message": "...", "details": {...} }`.
- New route file must export `team_routes() → Router<PgPool>` and be registered in `routes/mod.rs` + wired in `lib.rs`.

### Library / Framework Requirements

- Rust 1.75+, Axum 0.7, sqlx 0.7 (compile-time checked queries), PostgreSQL 15+.
- bigdecimal 0.4 — use string-based `bd_to_i64()` for rate conversion.
- chrono 0.4 — `NaiveDate` for date comparisons in allocation filtering.
- serde 1.0 — `#[derive(Debug, Serialize)]` on `TeamMemberResponse`, `#[derive(Debug, Deserialize)]` on request structs.
- validator 0.16 — if query params are needed (e.g., department filter).
- Frontend: Leptos 0.6 (CSR + Hydrate), Tailwind CSS 3.4, reqwest for API calls, web_sys for console logging.

### File Structure Requirements

- **Backend new files:**
  - `src/backend/src/routes/team.rs` — Team view endpoint handlers
  - `src/backend/src/services/team_service.rs` — Team query logic
  - `src/backend/tests/team_tests.rs` — Integration tests
- **Backend modified files:**
  - `src/backend/src/routes/mod.rs` — Add `pub mod team; pub use team::team_routes;`
  - `src/backend/src/services/mod.rs` — Add `pub mod team_service;`
  - `src/backend/src/lib.rs` — Wire `.nest("/api/v1/team", team_routes())`
- **Frontend new files:**
  - `src/frontend/src/pages/team.rs` — My Team dashboard page
- **Frontend modified files:**
  - `src/frontend/src/pages/mod.rs` — Add `pub mod team;`
  - `src/frontend/src/lib.rs` — Add `<Route path="/team" view=TeamPage/>`
  - `src/frontend/src/components/mod.rs` — Add "My Team" nav link for DeptHead/HR/Admin
- **No new migrations expected** — all required tables (`resources`, `departments`, `ctc_records`, `allocations`, `projects`) already exist from Epics 1-2.

### Testing Requirements

- Integration tests in `src/backend/tests/team_tests.rs` using `#[sqlx::test(migrations = "../../migrations")]`.
- **RBAC tests**: DeptHead gets own department, HR gets all, PM gets 403.
- **Security test**: Response body must NOT contain fields: `base_salary`, `hra_allowance`, `medical_allowance`, `transport_allowance`, `meal_allowance`, `bpjs_kesehatan`, `bpjs_ketenagakerjaan`, `thr_monthly_accrual`, `total_monthly_ctc`. Assert via serde_json field presence check.
- **CTC missing test**: Employee without CTC record appears with `ctc_status: "Missing"` and `daily_rate: null`.
- **Allocation aggregation test**: Employee with 2 active allocations (40% + 30%) shows `total_allocation_pct: 70.0`.
- Keep all 80+ existing tests green.

### Previous Story Intelligence (Epic 2)

- **Story 2.4 (latest completed)**: CTC validation engine, completeness service, compliance reports. Established patterns: dedicated service files under `services/`, exported via `mod.rs`, called from thin route handlers. Frontend follows `ctc_completeness.rs` (612 lines) pattern: summary cards + data table layout.
- **Story 2.3**: Eliminated N+1 queries using batch pre-fetch with `HashSet`/`HashMap`. Use same pattern — single JOIN-based SQL query.
- **Story 2.1**: BPJS calculator, daily rate calculation, BigDecimal handling. `daily_rate = total_monthly_ctc / working_days_per_month`.
- **Story 2.0**: CTC encryption (AES-256-GCM). Sensitive CTC fields are encrypted. `daily_rate` derivation needs verification — check if it's a plaintext column or encrypted.
- **Code review common findings (from Story 2.3)**: frontend JSON key mismatches (CRITICAL), missing transaction wrapping (HIGH), wrong migration defaults (HIGH), N+1 queries (MEDIUM), missing amount assertions in tests (MEDIUM). Proactively avoid these.
- **BigDecimal precision**: Always use string parsing (`"0.04".parse::<BigDecimal>()`), never `try_from(f64)`. Use `bd_to_i64()` which returns `Result` for integer conversion.

### Git Intelligence Summary

- Recent commits: `9e6f561` (Story 2.4), `5ed6700` (Story 2.3), `efbf01f` (Story 2.2), `37d8ab6` (Story 2.0), `983fade` (Story 2.1).
- Convention: `feat:` / `fix:` / `chore:` prefixes, terse commit messages.
- Changes concentrate in routes/, services/, migrations, and integration tests.
- Frontend compiles clean with only pre-existing `auth.rs` warning.

### Existing Code Reference Map

| What | Where | Why relevant |
|------|-------|-------------|
| Resources table schema | `migrations/*_create_resources.up.sql` | Source of employee data: id, name, role, department_id |
| Departments table schema | `migrations/*_create_departments.up.sql` | Department isolation for RLS |
| CTC records with daily_rate | `migrations/20260222140000_extend_ctc_records.up.sql` | `daily_rate` DECIMAL(12,2) — the blended rate |
| Allocations table | `migrations/*_create_allocations.up.sql` | resource_id, project_id, percentage, start_date, end_date |
| Projects table | `migrations/*_create_projects.up.sql` | project name for assignment display |
| RLS policies | `rls_department_isolation.up.sql` | `app.current_role` + `app.current_department_id` session vars |
| Auth middleware | `src/backend/src/routes/*.rs` | `user_id_from_headers(&headers)?` + `require_role()` pattern |
| CTC route handlers | `src/backend/src/routes/ctc.rs` | Handler signature patterns, auth extraction, audit logging |
| CTC encryption | `src/backend/src/services/ctc_crypto.rs` | AES-256-GCM — check if daily_rate needs decryption |
| Key provider | `src/backend/src/services/key_provider.rs` | `EnvKeyProvider` for `CTC_ENCRYPTION_KEY_V1` |
| Audit hash chain | `src/backend/src/services/audit_log.rs` | `compute_entry_hash()` + advisory lock pattern |
| BigDecimal helpers | `src/backend/src/services/ctc_calculator.rs` | `bd_to_i64()` — returns Result for safe conversion |
| CTC completeness UI | `src/frontend/src/pages/ctc_completeness.rs` | Layout template: summary cards + data table (612 lines) |
| THR page UI | `src/frontend/src/pages/thr.rs` | Alternative layout reference (903 lines) |
| Nav guard pattern | `src/frontend/src/components/mod.rs` | Role-based navigation link visibility |
| Frontend data fetch | `src/frontend/src/pages/ctc.rs` | `spawn_local` + `reqwest` + Leptos signals pattern |
| Allocation CTC guard | `src/backend/src/routes/allocation.rs` | CTC-required validation before allocation creation |
| CTC completeness service | `src/backend/src/services/ctc_completeness.rs` | JOIN-based completeness queries — pattern to follow |
| Allocation route | `src/backend/src/routes/allocation.rs` | Existing allocation endpoints for reference |

### Project Structure Notes

- No structural conflicts. Story 3.1 adds 2 new backend files + 1 new frontend page — clean addition to existing architecture.
- The team view is a read-only endpoint (GET) with no mutations. Simpler than CTC management stories.
- This story lays groundwork for Stories 3.2 (assignment interface), 3.3 (cost preview), 3.4 (overallocation warnings), 3.5 (budget dashboard). Design the `TeamMemberResponse` struct with future extensibility in mind.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.1: Team View with Blended Rates]
- [Source: _bmad-output/planning-artifacts/prd.md#FR9 — Department Heads can view their team members with blended daily rates]
- [Source: _bmad-output/planning-artifacts/prd.md#FR14 — Project Managers can view assigned resources with blended rates (no CTC component details)]
- [Source: _bmad-output/planning-artifacts/prd.md#FR15 — System prevents assignment of resources without CTC data]
- [Source: _bmad-output/planning-artifacts/architecture.md#Decision: CTC Data Model — daily_rate stored on ctc_records]
- [Source: _bmad-output/planning-artifacts/architecture.md#Decision: Daily Rate Calculation Strategy — pre-calculated and cached]
- [Source: _bmad-output/planning-artifacts/architecture.md#API Design Decisions — Enhanced Resource Assignment routes]
- [Source: _bmad-output/planning-artifacts/architecture.md#Decision: CTC Data Protection — defense-in-depth encryption]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Core User Experience — cost-aware assignment moment]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Design Direction: Stripe Financial — clean data tables, status badges]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Color System — green/yellow/red for budget/allocation status]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Typography — JetBrains Mono for numbers, right-aligned]
- [Source: _bmad-output/project-context.md#Critical Implementation Rules — AppError, BigDecimal, audit logging]
- [Source: _bmad-output/implementation-artifacts/2-4-ctc-validation-and-compliance.md — Previous story learnings, code reference map]
- [Source: _bmad-output/implementation-artifacts/2-4-ctc-validation-and-compliance.md — CTC completeness service JOIN patterns]

### Story Creation Completion Note

- Ultimate context engine analysis completed — comprehensive developer guide created with full cross-epic intelligence from Epic 1 (RBAC/RLS) and Epic 2 (CTC management), exact code references, architecture compliance, UX design direction (Stripe Financial), security guardrails (never expose CTC components to non-HR), and existing code reuse patterns.

## Dev Agent Record

### Agent Model Used

claude-opus-4-6

### Debug Log References

### Completion Notes List

- All 6 tasks implemented across backend and frontend
- Backend: team endpoint (`team.rs`), service layer (`team_service.rs`), integration tests (`team_tests.rs`)
- Frontend: team page with summary cards, data table, sorting/filtering, tooltip on projects, disabled Assign for CTC-missing
- Code review found 4 HIGH, 5 MEDIUM, 2 LOW issues; all HIGH and MEDIUM fixed:
  - M1: Audit log entity_id corrected to use department_id instead of always user_id
  - H3: CTC guard regression test added
  - M2: daily_rate value assertion (1200000) added to integration test
  - M5: Plaintext daily_rate fallback test added (950000)
  - H4: Unit tests added to team_service.rs (bd_to_i64_safe + sorting)
  - M3: Tooltip on projects column with full assignment details (dates, %)
  - M4: Disabled Assign button for CTC-missing rows with tooltip
  - H1: Story file updated — all tasks marked complete, Dev Agent Record populated
  - H2: Status changed from ready-for-dev to review
- Encrypted daily_rate path and plaintext fallback path both covered by tests
- Single JOIN query (no N+1) verified
- CTC component fields never exposed in response (security assertion in integration tests)
- **Test fix (session 2):** Replaced invalid 64-char hex encryption key with base64-encoded 32-byte key (`QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=`) across all 10 occurrences in `team_tests.rs`; added `CTC_ACTIVE_KEY_VERSION=v1` env var
- **Test fix (session 2):** Changed `ctc_guard_rejects_allocation_without_ctc` test from `hr` role to `admin` role — HR gets 403 from auth before reaching CTC guard
- **Test results (session 2):** 10/10 team integration tests pass, 6/6 team_service unit tests pass, 45/45 lib tests pass, frontend compiles clean
- **Code review (session 3):** Found 1 HIGH, 4 MEDIUM, 2 LOW issues; all HIGH and MEDIUM fixed:
  - H1: Removed 9999-12-31 sentinel date — NULL end_dates now display as "Ongoing" in tooltips
  - M1: Eliminated redundant department_id query — reads from RLS session variable instead of re-querying users table
  - M2: Fixed summary card label from "Avg Allocation" to "Avg Allocation %" for clarity
  - M3: Fixed test data inconsistency — plaintext daily_rate now matches encrypted value (1200000)
  - M4: Strengthened HR test — added resource-specific assertions verifying cross-department visibility
- CTC component fields never exposed in response (security assertion in integration tests)

### File List

**New files:**
- `src/backend/src/routes/team.rs` — Team view endpoint handler with auth, RLS, audit logging
- `src/backend/src/services/team_service.rs` — Team query service with bd_to_i64_safe, encryption handling, unit tests
- `src/backend/tests/team_tests.rs` — Integration tests (RBAC, security, CTC guard regression, rate assertions)
- `src/frontend/src/pages/team.rs` — My Team dashboard page (summary cards, data table, sort/filter)

**Modified files:**
- `src/backend/src/routes/mod.rs` — Added `pub mod team; pub use team::team_routes;`
- `src/backend/src/services/mod.rs` — Added `pub mod team_service;`
- `src/backend/src/lib.rs` — Wired `/api/v1/team` route
- `src/frontend/src/pages/mod.rs` — Added `pub mod team;`
- `src/frontend/src/lib.rs` — Added `<Route path="/team" view=TeamPage/>`
- `src/frontend/src/components/mod.rs` — Added "My Team" nav link for DeptHead/HR/Admin roles

### Change Log

- 2026-03-01: Initial implementation — all 6 tasks complete (backend endpoint, service layer, CTC guard verification, frontend page, sorting/filtering, tests)
- 2026-03-01: Code review fixes — audit entity_id, CTC guard regression test, rate assertions, unit tests, tooltip/disabled-Assign UI, story file updates
- 2026-03-01: Test fixes — encryption key format (hex→base64), CTC guard test role (hr→admin). All tests green.
- 2026-03-01: Code review pass — fixed sentinel date in tooltip, redundant query, label mismatch, test data inconsistency, HR test assertions. Status → done.
- 2026-03-03: Post-deployment bug fixes — `resource_type` filter changed from `'human'` to `'employee'` in team_service.rs (2 occurrences); department scoping fixed in `get_team()` to use `resolve_department_id()` for all roles (admin/hr no longer bypass department filter)
