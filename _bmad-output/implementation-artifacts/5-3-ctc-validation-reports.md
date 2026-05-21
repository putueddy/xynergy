# Story 5.3: CTC Validation Reports

Status: done

## Story

As a **Finance Controller**,
I want **to generate CTC validation reports comparing system data to payroll records**,
so that **I can ensure data accuracy for compliance**.

## Acceptance Criteria

1. **Given** I navigate to Finance -> CTC Validation **when** I select a date range and run the report **then** the system compares Xynergy CTC data vs payroll system records.
2. **Given** the comparison completes **when** I view the results **then** I see: Match Rate %, Discrepancy Count, List of mismatches.
3. **Given** discrepancies are found **when** I click on a mismatch **then** I see: Employee, Field, Xynergy Value, Payroll Value, Variance.
4. **Given** I run the BPJS validation **when** I select employees to sample **then** the system verifies calculations match regulations **and** flags any calculation errors.

## Scope Boundary

- In scope: finance/admin CTC validation reporting, payroll-vs-Xynergy discrepancy comparison, BPJS sample validation workflow, date-range filtering, mismatch drill-down UI, report summary metrics, audit logging for report generation, and integration tests.
- In scope for MVP payroll comparison source: a local finance-owned staging/import table introduced by this story for payroll reconciliation inputs; this story does not require live payroll-system API integration.
- Not in scope: new CTC CRUD, new cash-flow analytics, four-eyes export workflow implementation itself, external ERP automation, compliance audit report bundle generation (Story 5.4), or redesigning existing HR completeness flows.
- Dependencies from prior stories: Story 2.4 already shipped BPJS compliance recalculation and the shared CTC completeness/compliance page; Story 1.4 already shipped audit report infrastructure; Stories 5.1 and 5.2 already established finance route/page/sidebar patterns.

## Tasks / Subtasks

- [x] **Task 1: Define the report contract and data source boundary** (AC: #1, #2, #3, #4)
  - [x] Introduce the MVP payroll comparison source as a local staging/import table owned by this story rather than leaving the source undefined. Recommended table shape:
    - [x] `resource_id UUID`
    - [x] `effective_date DATE`
    - [x] `base_salary BIGINT`
    - [x] allowance/BPJS comparison columns needed by the report
    - [x] `import_batch_id UUID`
    - [x] `imported_at TIMESTAMP`
  - [x] Add indexes needed for report performance, at minimum on `(effective_date)` and `(resource_id, effective_date)` for the payroll staging source.
  - [x] Add explicit backend DTOs for report filters and response payloads, including summary metrics and mismatch rows.
  - [x] Keep canonical mismatch fields explicit: `employee_id`, `employee_name`, `field_name`, `xynergy_value`, `payroll_value`, `variance_amount`, `status`, plus optional BPJS-specific metadata.
  - [x] Add summary fields that make skipped data visible to Finance: `total_compared`, `total_matches`, `total_discrepancies`, `match_rate_pct`, and `excluded_count`.
  - [x] Reject inverted date ranges early with `AppError::Validation`.
  - [x] Validate the payroll source before comparison begins: fail fast if the selected range has no staging/import data, if required employees are missing, or if the import batch is stale/incomplete for the requested period.

- [x] **Task 2: Build backend validation-report service by extending existing compliance foundations** (AC: #1, #2, #3, #4)
  - [x] Prefer extending shared helpers in `src/backend/src/services/compliance_report.rs` first; only create `src/backend/src/services/ctc_validation_report.rs` if a separate orchestration file keeps responsibilities clearer without duplicating BPJS/decryption logic.
  - [x] Reuse `DefaultCtcCryptoService` and existing encrypted CTC read patterns from `compliance_report.rs`; do not duplicate decryption logic.
  - [x] Reuse `calculate_bpjs()` and existing JKK risk-tier handling for AC #4; do not reimplement BPJS formulas.
  - [x] Compare decrypted Xynergy CTC values against payroll-source values field by field and compute:
    - [x] `total_compared`
    - [x] `total_matches`
    - [x] `total_discrepancies`
    - [x] `match_rate_pct`
    - [x] `excluded_count`
  - [x] Return stable, deterministic mismatch ordering (employee name ASC, field name ASC).
  - [x] Do not silently skip records that cannot be decrypted or lack payroll data; surface them through `excluded_count` and, where useful, an explicit excluded/skipped reason list.
  - [x] Keep the existing BPJS-only `/api/v1/ctc/compliance-report` behavior intact; the new validation report should be additive and may call shared compliance helpers internally.

- [x] **Task 3: Expose finance-facing API endpoints with audit coverage** (AC: #1, #2, #3, #4)
  - [x] Add endpoint(s) under existing CTC route space, e.g.:
    - [x] `GET /api/v1/ctc/validation-report?start_date=YYYY-MM-DD&end_date=YYYY-MM-DD`
    - [x] paginated mismatch query support such as `limit` / `offset`, or a dedicated details endpoint, so large reports do not require a single oversized payload
  - [x] Restrict access to `finance` and `admin`; do not allow `department_head` or `project_manager`.
  - [x] Reuse finance/admin authorization style from `cash_flow.rs` and existing claims extraction helpers.
  - [x] Log report generation to `audit_logs` with an action such as `ctc_validation_report_generated`, capturing date range and summary counts only.
  - [x] Keep sensitive values out of audit payloads beyond what is necessary for traceability. Allowed payload fields should be limited to report metadata such as date range, `total_compared`, `total_matches`, `total_discrepancies`, `excluded_count`, and `match_rate_pct`.
  - [x] Do not rename or remove `/api/v1/ctc/compliance-report`; this story adds `/api/v1/ctc/validation-report` as a finance-facing reconciliation endpoint.

- [x] **Task 4: Add Finance -> CTC Validation UI using existing page patterns** (AC: #1, #2, #3, #4)
  - [x] Prefer a dedicated finance page/route (for example `src/frontend/src/pages/ctc_validation.rs` and `/finance/ctc-validation`) rather than overloading the HR-oriented completeness dashboard.
  - [x] Reuse existing `authenticated_get` helpers and finance page guard patterns from `cash_flow.rs`.
  - [x] Reuse patterns from `ctc_completeness.rs`, but do not copy that page wholesale; it mixes HR completeness and compliance concerns and already contains response-parsing assumptions that should not be propagated into the new finance report page.
  - [x] Include filter controls for start date and end date, plus a report trigger button.
  - [x] Render summary cards for Match Rate %, Compared Records, Discrepancy Count, Excluded Count, and BPJS Error Count.
  - [x] Render a mismatch table with keyboard-accessible drill-down/expand behavior for AC #3; if the table is interactive, use explicit expand/collapse controls with proper `aria-expanded`, `aria-controls`, and visible focus states.
  - [x] If BPJS sample selection is UI-driven, provide a finance-safe selector or sample toggle without exposing unrelated HR-only completeness actions.
  - [x] If report details are paginated, keep summary metrics stable while only the detail region reloads.

- [x] **Task 5: Wire routes, navigation, and local visual conventions** (AC: #1)
  - [x] Register the new finance page in `src/frontend/src/pages/mod.rs` and `src/frontend/src/lib.rs`.
  - [x] Add a Finance sidebar entry for CTC Validation alongside Cash Flow.
  - [x] Keep styling aligned with the current Huly-inspired finance surfaces and the native table/card patterns already used in `cash_flow.rs`, `projects.rs`, and `ctc_completeness.rs`.
  - [x] Preserve WCAG-required table parity and clear labels; if any chart or visual summary is added later, keep equivalent tabular data in the DOM.

- [x] **Task 6: Add integration and regression coverage** (AC: #1, #2, #3, #4)
  - [x] Create `src/backend/tests/ctc_validation_report_tests.rs`.
  - [x] Required tests:
    - [x] finance can fetch validation report
    - [x] admin can fetch validation report
    - [x] hr / department_head / project_manager denied with `403`
    - [x] inverted date range returns `400`
    - [x] report returns summary counts and mismatch rows deterministically
    - [x] missing or stale payroll staging data returns a clear validation error
    - [x] BPJS validation reuses current formula outputs and flags mismatches correctly
    - [x] excluded/skipped records are counted and reported deterministically
    - [x] report generation creates an audit log entry
  - [x] Re-run high-risk suites:
    - [x] `src/backend/tests/ctc_validation_tests.rs`
    - [x] `src/backend/tests/audit_tests.rs`
    - [x] `src/backend/tests/cash_flow_tests.rs`
  - [x] Verify existing `/api/v1/ctc/compliance-report` behavior and audit action remain unchanged after Story 5.3 lands.

### Review Findings

- [x] [Review][Decision] Define BPJS employee sampling workflow — **Resolved 2026-05-20:** added optional `employee_ids` query parameter (comma-separated UUIDs, capped at 200). Default behavior remains "validate every employee in range." When sampling is used, the response carries `sampled = true` and the audit payload records `sampled` + `sampled_count` (ids are NOT logged). UI exposes a collapsible "Sample specific employees" panel with `aria-expanded`/`aria-controls`.
- [x] [Review][Decision] Define payroll source completeness and freshness rules — **Resolved 2026-05-20:** two service-layer rules, no schema change. (1) Freshness: fail with `AppError::Validation` if the most-recent `imported_at` in the requested range is older than `PAYROLL_FRESHNESS_DAYS = 7` days before `end_date`. (2) Coverage: surface `payroll_coverage_pct` (distinct payroll resources / distinct CTC resources × 100, rounded to 2 dp) in the summary and audit payload; below 90% Finance can spot the gap in the existing excluded-records panel. Schema additions (`batch_status`, `completed_at`, `expected_resource_count`) are listed under Follow-up.
- [x] [Review][Decision] Confirm multiple payroll row semantics — **Resolved 2026-05-20:** current "latest payroll baseline per resource" behavior is canonical (effective_date DESC, imported_at DESC, batch id DESC). Documented at the top of `generate_validation_report` and on the Run Report button tooltip. "Compare every payroll row in the range" deferred to a future `?mode=all` flag if Finance ever requests it.
- [x] [Review][Decision] Confirm CTC effective-date selection semantics — **Resolved 2026-05-20:** keep `effective_date BETWEEN start_date AND end_date`, identical to `compliance_report::validate_bpjs_compliance`. The two reports stay comparable. A future "active CTC as-of date X" view would be a separate `/api/v1/ctc/snapshot?as_of=...` endpoint, not a flag on validation-report.
- [x] [Review][Patch] Report payroll-only employees as excluded instead of silently ignoring them [`src/backend/src/services/ctc_validation_report.rs`]
- [x] [Review][Patch] Make same-day duplicate payroll imports deterministic with `imported_at`/batch tie-breaks and coverage [`src/backend/src/services/ctc_validation_report.rs`; `src/backend/tests/ctc_validation_report_tests.rs`]
- [x] [Review][Patch] Persist and validate BPJS `risk_tier` so tiers 2-4 do not default to tier 1 during report recalculation [`src/backend/src/routes/ctc.rs`; `src/backend/src/services/ctc_validation_report.rs`]
- [x] [Review][Patch] Flag BPJS regulation errors when either payroll or Xynergy BPJS values diverge from recalculated regulation values, and report meaningful variance [`src/backend/src/services/ctc_validation_report.rs`]
- [x] [Review][Patch] Bound mismatch pagination by default, request paginated details from the UI, and clear stale results on failed reruns [`src/backend/src/services/ctc_validation_report.rs`; `src/frontend/src/pages/ctc_validation.rs`]
- [x] [Review][Patch] Align `total_discrepancies` with the field-level mismatch list and add regression coverage [`src/backend/src/services/ctc_validation_report.rs`; `src/backend/tests/ctc_validation_report_tests.rs`]
- [x] [Review][Patch] Make Story 5.3 integration tests date-stable after 2025 [`src/backend/tests/ctc_validation_report_tests.rs`]
- [x] [Review][Patch] Update test/gate artifacts so unexecuted or decision-blocked work is not described as release-approved [`_bmad-output/test-artifacts/`]
- [x] [Review][Patch] Scope sampled payroll-source validation to selected employees and dedupe sample IDs before audit counting [`src/backend/src/routes/ctc.rs`; `src/backend/src/services/ctc_validation_report.rs`; `src/backend/tests/ctc_validation_report_tests.rs`]
- [x] [Review][Patch] Share JKK risk-tier mapping from `ctc_calculator`, include `bpjs_error_count` in audit metadata, and count missing BPJS fields when payroll violates regulation [`src/backend/src/services/ctc_calculator.rs`; `src/backend/src/services/compliance_report.rs`; `src/backend/src/services/ctc_validation_report.rs`; `src/backend/src/routes/ctc.rs`]
- [x] [Review][Patch] Preserve report summary while paginating details, fix exact-page Next detection, validate/encode sample IDs, improve a11y table metadata, and keep excluded employee IDs visible [`src/frontend/src/pages/ctc_validation.rs`]
- [x] [Review][Defer] Payroll-side risk-tier reconciliation — deferred after Claude routing: direct tier comparison requires a payroll-source contract change; tier drift still surfaces through BPJS amount variance [`migrations/20260307100000_add_payroll_validation_staging.up.sql`; `src/backend/src/services/ctc_validation_report.rs`]
- [x] [Review][Defer] Employee-side BPJS reconciliation — deferred after Claude routing: real follow-up, but requires widening staging/import/API/UI/test contracts beyond this review patch [`migrations/20260307100000_add_payroll_validation_staging.up.sql`; `src/backend/src/services/ctc_validation_report.rs`]
- [x] [Review][Defer] Derived totals and accrual-only payroll fields — deferred after Claude routing: component reconciliation implies `total_monthly_ctc`; `thr_monthly_accrual` has no payroll-batch counterpart in the MVP staging source [`src/backend/src/services/ctc_validation_report.rs`]
- [x] [Review][Patch] Run validation-report queries under the established RLS context [`src/backend/src/routes/ctc.rs:1418`]
- [x] [Review][Patch] Recalculate BPJS expected values from each side's own salary basis [`src/backend/src/services/ctc_validation_report.rs:501`]
- [x] [Review][Patch] Handle legacy CTC rows with missing encryption metadata as exclusions instead of decode errors [`src/backend/src/services/ctc_validation_report.rs:296`]
- [x] [Review][Patch] Handle missing or invalid CTC risk-tier metadata without defaulting to tier 1 or skipping non-BPJS comparisons [`src/backend/src/services/ctc_validation_report.rs:458`]
- [x] [Review][Patch] Reconcile every sampled employee id against CTC and payroll source rows [`src/backend/src/services/ctc_validation_report.rs:334`]
- [x] [Review][Patch] Compute payroll coverage from the CTC/payroll intersection instead of all payroll-only rows [`src/backend/src/services/ctc_validation_report.rs:425`]
- [x] [Review][Patch] Validate payroll freshness on selected baseline rows, not only global MAX(imported_at) [`src/backend/src/services/ctc_validation_report.rs:358`]
- [x] [Review][Patch] Tie CTC validation pagination to the successfully loaded filter/page state so failed page loads or edited filters cannot show stale rows under a new range [`src/frontend/src/pages/ctc_validation.rs:217`]
- [x] [Review][Patch] Render backend validation error envelopes as sanitized messages instead of raw JSON response bodies [`src/frontend/src/pages/ctc_validation.rs:172`]
- [x] [Review][Patch] Regenerate Tailwind output with the repo-local locked version and include new CTC validation page classes [`src/frontend/public/output.css:1`; `src/frontend/package-lock.json`]
- [x] [Review][Patch] Bring Story 5.3 traceability and gate artifacts in sync with resolved review decisions, current test inventory, and post-review verification boundaries [`_bmad-output/test-artifacts/traceability/5-3-ctc-validation-reports-traceability.md:22`]
- [x] [Review][Patch] Correct the test automation summary so generated-test counts, verification claims, and production-change notes match the final Story 5.3 review state [`_bmad-output/test-artifacts/automation-summary.md:59`]
- [x] [Review][Patch] Correct story completion/file-list notes so the artifact no longer says sprint tracking was preserved as `review` or frames completion as only context generation [`_bmad-output/implementation-artifacts/5-3-ctc-validation-reports.md:271`]

### Follow-up

- Add payroll batch state columns (`batch_status`, `completed_at`, `expected_resource_count`) if Finance wants import completeness enforced at batch-contract level instead of via current freshness + coverage metrics.
- Add direct payroll-side `risk_tier` reconciliation only if payroll imports expose a tier/classification field.
- Extend staging/import/report/UI coverage to compare employee-side BPJS contributions (`bpjs_kesehatan_employee`, `bpjs_ketenagakerjaan_employee`) after the payroll contract is widened to include deduction-side values.
- Keep derived totals and accrual-only fields out of this MVP reconciliation: `total_monthly_ctc` is implied by component reconciliation, and `thr_monthly_accrual` has no payroll-batch counterpart in the current source.
- Add `/api/v1/ctc/snapshot?as_of=...` for as-of-date CTC views if Finance needs snapshots instead of the current `BETWEEN start..end` report.
- Add `?mode=all` payroll-row comparison if Finance later needs every payroll row in range instead of the current latest-per-resource snapshot.
- Add employee-directory autocomplete for sample selection if comma-separated UUID input proves too manual.

## Dev Notes

### Developer Context

- Story 2.4 already delivered the core backend building blocks for this story: `validate_bpjs_compliance()` in `src/backend/src/services/compliance_report.rs`, the `/api/v1/ctc/compliance-report` endpoint in `src/backend/src/routes/ctc.rs`, and the shared `ctc_completeness.rs` frontend page. Story 5.3 must extend those patterns, not re-create compliance math or crypto plumbing. [Source: `_bmad-output/implementation-artifacts/2-4-ctc-validation-and-compliance.md`; `src/backend/src/services/compliance_report.rs`; `src/backend/src/routes/ctc.rs`]
- Story 1.4 already implemented audit report retrieval, export-request persistence, and tamper verification. Story 5.3 should emit report-generation audit events that fit the existing `audit_logs` model rather than inventing a parallel compliance log. [Source: `_bmad-output/implementation-artifacts/1-4-audit-logging-system.md`; `src/backend/src/routes/audit_log.rs`]
- Stories 5.1 and 5.2 established the Finance section in the sidebar and the current finance page role-guard/fetch patterns. Story 5.3 should feel like the next vertical slice in that section. [Source: `_bmad-output/implementation-artifacts/5-1-cash-flow-entry.md`; `_bmad-output/implementation-artifacts/5-2-cash-flow-dashboard.md`; `src/frontend/src/components/app_sidebar.rs`; `src/frontend/src/pages/cash_flow.rs`]
- Current repo reality differs from older planning docs on Tailwind: the active frontend package is already on Tailwind CSS 4.1.x. Follow current repo dependencies and CSS patterns, not the older Tailwind 3.4 planning assumption. [Source: `src/frontend/package.json`; `_bmad-output/planning-artifacts/ux-design-specification.md`]
- The existing finance-accessible BPJS report endpoint `/api/v1/ctc/compliance-report` is already in production use and must remain intact. Story 5.3 adds reconciliation behavior on top of that foundation rather than replacing it. [Source: `src/backend/src/routes/ctc.rs`; `src/backend/tests/ctc_validation_tests.rs`]
- `src/frontend/src/pages/ctc_completeness.rs` is a useful pattern source for tables and compliance summaries, but it should not be copied wholesale into the new finance page because it combines HR completeness concerns with compliance behavior and already assumes a response shape that does not exactly mirror the backend contract. [Source: `src/frontend/src/pages/ctc_completeness.rs`; `src/backend/src/services/compliance_report.rs`]

### Technical Requirements

- Keep all currency values as IDR whole numbers (`i64`/`BIGINT`) end-to-end. Do not introduce floating-point payroll comparisons for stored currency amounts. [Source: `_bmad-output/project-context.md`; `src/backend/src/services/compliance_report.rs`]
- Reuse existing BPJS recalculation logic from `calculate_bpjs()` and the existing JKK tier mapping; this story must not duplicate or drift from the current compliance implementation. [Source: `_bmad-output/implementation-artifacts/2-4-ctc-validation-and-compliance.md`; `src/backend/src/services/compliance_report.rs`]
- Decrypt CTC components in the service layer only. Never return ciphertext or key metadata to the frontend. [Source: `_bmad-output/implementation-artifacts/2-4-ctc-validation-and-compliance.md`; `_bmad-output/planning-artifacts/architecture.md#Decision: CTC Data Protection (MVP)`]
- Use deterministic discrepancy ordering and deterministic summary calculations so finance can compare repeated runs reliably.
- Validation/report queries must handle missing payroll-source rows explicitly. Prefer a visible mismatch or excluded-count field over silent omission.
- Keep sensitive audit payloads lean: date range, counts, and report metadata are acceptable; full payroll-vs-Xynergy value snapshots should stay in the report response only.
- Treat the payroll comparison source as part of this story's implementation scope: add a local staging/import model and validate its completeness/freshness before generating reconciliation results.
- Large mismatch sets must not force one oversized response. Use paginated details or a summary/details split while keeping summary totals stable for the selected date range.

### Architecture Compliance

- Keep Axum handlers thin and place report orchestration in a dedicated service module under `src/backend/src/services/`. [Source: `_bmad-output/project-context.md`; `_bmad-output/planning-artifacts/architecture.md`]
- Continue using existing route composition and `/api/v1/...` conventions; if the story adds a new route file, export it via `routes/mod.rs` and merge it in backend router setup. [Source: `src/backend/src/routes/mod.rs`; `_bmad-output/project-context.md`]
- Continue using `AppError` mappings and avoid `unwrap()` in production code. [Source: `_bmad-output/project-context.md`]
- Reuse existing audit infrastructure instead of inventing a second logging mechanism. [Source: `src/backend/src/routes/audit_log.rs`; `_bmad-output/implementation-artifacts/1-4-audit-logging-system.md`]
- Prefer a dedicated finance page/route over embedding more finance-specific behavior into the already large HR-oriented `ctc_completeness.rs`, unless implementation proves a shared extracted component is cleaner.
- Preserve the existing `/api/v1/ctc/compliance-report` endpoint contract and keep the new validation route additive. Shared helpers are encouraged; endpoint replacement is not.

### Library / Framework Requirements

- Stay on the repo's current major versions during this story unless the user separately requests upgrades:
  - Rust 2021 / Rust 1.75+
  - Axum 0.7
  - sqlx 0.7
  - Leptos 0.6
  - PostgreSQL 15+
  - Tailwind CSS 4.1.x in the frontend package
  [Source: `Cargo.toml`; `src/frontend/package.json`; `_bmad-output/project-context.md`]
- sqlx test guidance still supports the repo's current `#[sqlx::test(migrations = "../../migrations")]` integration style with automatic isolated databases and migrations; keep using the repo's established pattern instead of introducing a custom harness. [Source: `https://docs.rs/sqlx/latest/sqlx/attr.test.html`; `src/backend/tests/ctc_validation_tests.rs`]
- For client-side async data, keep using the repo's established Leptos resource/signal patterns. Official Leptos guidance confirms resources are the right abstraction for reactive async loading and manual `refetch()` support. [Source: `https://book.leptos.dev/async/10_resources.html`; `src/frontend/src/pages/cash_flow.rs`; `src/frontend/src/pages/projects.rs`]
- If any visual summaries become more than plain tables, follow W3C guidance for complex visual information by keeping a text/table alternative adjacent to the visualization. [Source: `https://www.w3.org/WAI/tutorials/images/complex/`]

### File Structure Requirements

- Backend likely touch points:
  - `src/backend/src/lib.rs`
  - `src/backend/src/routes/ctc.rs`
  - `src/backend/src/routes/mod.rs`
  - `src/backend/src/services/mod.rs`
  - `src/backend/src/services/compliance_report.rs` (reuse only; avoid destabilizing unless necessary)
  - `src/backend/src/services/ctc_validation_report.rs` (new recommended service)
  - `migrations/<new>_add_payroll_validation_staging.sql`
  - `src/backend/tests/ctc_validation_report_tests.rs` (new)
- Frontend likely touch points:
  - `src/frontend/src/pages/ctc_validation.rs` (new recommended page)
  - `src/frontend/src/pages/mod.rs`
  - `src/frontend/src/lib.rs`
  - `src/frontend/src/components/app_sidebar.rs`
- Existing files that should be treated as pattern references, not redesign targets:
  - `src/frontend/src/pages/ctc_completeness.rs`
  - `src/frontend/src/pages/cash_flow.rs`
  - `src/backend/src/routes/audit_log.rs`
  - `src/backend/src/services/compliance_report.rs`

### Testing Requirements

- Follow current backend integration style: `#[sqlx::test(migrations = "../../migrations")]`. [Source: `src/backend/tests/ctc_validation_tests.rs`; `https://docs.rs/sqlx/latest/sqlx/attr.test.html`]
- Cover role access, date validation, discrepancy correctness, BPJS validation correctness, payroll staging validation, excluded/skipped record handling, and audit logging.
- Re-run regression suites most likely to break from shared CTC/audit/finance code reuse:
  - `ctc_validation_tests.rs`
  - `audit_tests.rs`
  - `cash_flow_tests.rs`
- Frontend manual verification should confirm:
  - finance/admin can see the nav item and route
  - non-finance roles are redirected or blocked
  - mismatch drill-down is keyboard reachable
  - summary values match the tabular details
  - summary cards remain correct when details are paginated
  - existing BPJS compliance page behavior is unchanged

### Previous Story Intelligence (5.2)

- Story 5.2 reinforced the repo's current vertical-slice approach: backend service + route + frontend page + test file in one cohesive change. Follow that same shape here to reduce review friction. [Source: `_bmad-output/implementation-artifacts/5-2-cash-flow-dashboard.md`; recent git history]
- Story 5.2 also documented that dashboard and finance UIs should reuse existing auth helpers, native table/card patterns, and chart/table accessibility parity rather than introducing new dependencies or a competing visual system. [Source: `_bmad-output/implementation-artifacts/5-2-cash-flow-dashboard.md`; `src/frontend/src/pages/cash_flow.rs`]
- Story 5.1 review history showed the repo is sensitive to role-path mismatches. Do not fetch finance data from endpoints that only HR or PM can access. [Source: `_bmad-output/implementation-artifacts/5-1-cash-flow-entry.md`]

### Git Intelligence Summary

- Recent commits show a clear end-to-end slice cadence:
  - `238664c feat: implement Story 5.2 - Cash Flow Dashboard with backend API, frontend integration, and testing`
  - `cc39b62 feat: implement Story 5.1 - Cash Flow Entry with backend API, frontend integration, and database migration`
  - `b991c2a feat: implement Story 4.6 - Profitability Forecasting with project-level cost projections, margin analysis, and resource mix insights`
- Match that precedent: keep Story 5.3 focused, explicit, and vertically integrated across backend, frontend, and tests.

### Latest Technical Information

- Official sqlx test documentation confirms isolated per-test databases, automatic migration support, and fixture support are built into `#[sqlx::test]`; the repo's existing usage remains aligned with current guidance. [Source: `https://docs.rs/sqlx/latest/sqlx/attr.test.html`]
- Official Leptos resource guidance confirms resources/local resources are the right fit for reactive async report loading and manual refresh. This supports using the repo's existing resource-style data loading instead of imperative fetch sprawl. [Source: `https://book.leptos.dev/async/10_resources.html`]
- W3C guidance for complex images/charts recommends a short description plus adjacent long/textual/tabular representation for complex visual information. If Story 5.3 adds visual summaries, keep a first-class textual/table alternative in the page. [Source: `https://www.w3.org/WAI/tutorials/images/complex/`]
- The repo currently uses Tailwind 4.1.x in practice, so any new classes/components should be tested against the actual frontend build tooling instead of older planning assumptions. [Source: `src/frontend/package.json`]

### Project Context Reference

- Core context: `_bmad-output/project-context.md`
- Planning artifacts:
  - `_bmad-output/planning-artifacts/epics.md`
  - `_bmad-output/planning-artifacts/prd.md`
  - `_bmad-output/planning-artifacts/architecture.md`
  - `_bmad-output/planning-artifacts/ux-design-specification.md`

### References

1. `_bmad-output/planning-artifacts/epics.md` - Epic 5 / Story 5.3 narrative and acceptance criteria.
2. `_bmad-output/planning-artifacts/prd.md` - FR49, FR56-FR57, NFR19-NFR20, and reporting/compliance requirements.
3. `_bmad-output/planning-artifacts/architecture.md` - CTC protection, REST endpoint, and audit/compliance architecture guardrails.
4. `_bmad-output/planning-artifacts/ux-design-specification.md` - finance user expectations, table/chart accessibility, and Stripe-style data clarity.
5. `_bmad-output/project-context.md` - Rust/Axum/Leptos/sqlx guardrails, security rules, and file-structure conventions.
6. `_bmad-output/implementation-artifacts/2-4-ctc-validation-and-compliance.md` - existing compliance-report and completeness foundations.
7. `_bmad-output/implementation-artifacts/1-4-audit-logging-system.md` - audit report/export/verification foundations.
8. `_bmad-output/implementation-artifacts/5-1-cash-flow-entry.md` - finance access patterns and role-path lessons.
9. `_bmad-output/implementation-artifacts/5-2-cash-flow-dashboard.md` - finance page/dashboard slice patterns.
10. `src/backend/src/services/compliance_report.rs` - reusable BPJS compliance service and existing response contract.
11. `src/backend/src/routes/ctc.rs` - current compliance-report endpoint and access pattern.
12. `src/frontend/src/pages/ctc_completeness.rs` - pattern source and cautionary example for response-shape drift.
13. `src/frontend/src/pages/cash_flow.rs` - finance page/auth/fetch baseline.
14. `https://www.w3.org/WAI/tutorials/images/complex/` - accessibility guidance for complex data visuals.

## Story Completion Status

- Status: done
- Completion note: Story 5.3 implementation and review patch passes are complete. The story adds the CTC validation report backend, payroll staging source, finance UI, audit coverage, and regression tests; final live-DB execution of the expanded 36-test integration suite remains a release-evidence step because the current review environment did not expose `DATABASE_URL`.

## Dev Agent Record

### Agent Model Used

openai/gpt-5.4

### Debug Log References

- Workflow source: `_bmad/bmm/workflows/4-implementation/create-story/workflow.yaml`
- Workflow instructions: `_bmad/bmm/workflows/4-implementation/create-story/instructions.xml`
- Validation checklist: `_bmad/bmm/workflows/4-implementation/create-story/checklist.md`
- Sprint tracking source: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Explore agent session: `ses_339f71b20ffeKCWOlbQyjFu8wa`
- Librarian agent session: `ses_339f71b18ffevcLTzXSRcwBHch`

### Completion Notes List

- Story created from explicit user target `5-3`; story key resolved as `5-3-ctc-validation-reports`.
- Context synthesized from Epic 5 requirements, architecture constraints, UX requirements, project context rules, Story 2.4 compliance implementation, Story 1.4 audit infrastructure, Story 5.1/5.2 finance patterns, repository code search, and external docs for accessibility and testing patterns.
- Guidance keeps Story 5.3 on the repo's current major versions and clarifies additive behavior relative to the existing compliance-report endpoint.
- Guidance explicitly sets the MVP payroll comparison source as a local staging/import table introduced by this story, rather than leaving source definition open.
- Implementation (2026-05-20): Added `payroll_validation_staging` migration with required indexes; created `ctc_validation_report` service that reuses `DefaultCtcCryptoService`, `calculate_bpjs()`, and the JKK risk-tier helper from the compliance foundation; added `GET /api/v1/ctc/validation-report` (finance/admin only, paginated, audit-logged as `ctc_validation_report_generated`); built dedicated finance page at `/finance/ctc-validation` with summary cards, accessible mismatch table (explicit `aria-expanded`/`aria-controls`), and excluded-records panel; wired Finance sidebar entry and route registration; authored `ctc_validation_report_tests.rs` covering role access, inverted dates, missing staging data, deterministic summary/mismatch ordering, BPJS regulation-error flagging, exclusion counting, audit log creation, and the regression check that the existing `/api/v1/ctc/compliance-report` endpoint still emits its `compliance_report_generated` audit action.
- Review patch pass (2026-05-20): Applied code-review fixes for payroll-only row visibility, deterministic same-day payroll import selection, BPJS risk-tier persistence/validation, BPJS regulation-error variance/counting, bounded mismatch pagination, stale UI result clearing, field-level discrepancy count semantics, date-stable tests, and traceability artifact wording.
- Review rerun patch pass (2026-05-20): Applied fixes for sample-scoped payroll validation, sampled-ID dedupe/cap enforcement, shared JKK tier mapping, BPJS missing-field error counting, `bpjs_error_count` audit metadata, deterministic duplicate-name ordering, excluded-row response capping, frontend sample validation/encoding, stable summary during pagination, exact-page Next handling, accessible table metadata, and visible excluded employee IDs. Claude routed remaining scope questions and recommended deferring direct payroll risk-tier, employee-side BPJS, and derived/accrual field reconciliation to follow-up contract work.
- Verification after review patch: `SQLX_OFFLINE=true cargo check -p xynergy-backend` passed; `SQLX_OFFLINE=true cargo check -p xynergy-frontend --target wasm32-unknown-unknown` passed with only pre-existing `team.rs` dead-code warnings; earlier Story 5.3 integration execution passed `cargo test -p xynergy-backend --test ctc_validation_report_tests` (26/26) against live PostgreSQL; regression suites `ctc_validation_tests` (15/15), `audit_tests` (5/5), and `cash_flow_tests` (31/31) passed against live PostgreSQL; after later backend review patches, final service unit tests passed via `cargo test -p xynergy-backend ctc_validation_report::tests --lib` (9/9) and the expanded final `ctc_validation_report_tests` suite compiled with `cargo test -p xynergy-backend --test ctc_validation_report_tests --no-run`; final live-DB execution of all 36 integration tests remains pending because `DATABASE_URL` was unavailable in the current environment. `npm run build`, `cargo check -p xynergy-frontend --target wasm32-unknown-unknown --no-default-features --features hydrate`, and `git diff --check` passed after frontend review patches; `npm audit --omit=dev` reported 0 vulnerabilities.
- Decision-resolution pass (2026-05-20): Putu confirmed the four pending decisions (BPJS sampling = optional `employee_ids[]`; payroll freshness = 7-day rule + coverage warning, no schema change; multiple payroll rows = latest-per-resource snapshot; CTC effective-date = `BETWEEN start..end`). Implemented: extended `ValidationReportFilters` with `employee_ids`, added `sampled`/`payroll_coverage_pct` to `ValidationReport`, added `PAYROLL_FRESHNESS_DAYS = 7` freshness check and `compute_payroll_coverage_pct` helper to `ctc_validation_report.rs`, wired the route to accept comma-separated `employee_ids` (capped at 200), extended the audit payload with `sampled`/`sampled_count`/`payroll_coverage_pct`, added 5 new integration tests (sampling happy path, invalid-uuid rejection, audit sampled/non-sampled metadata, stale-payroll 400, coverage 50% surfaces missing employees), updated the existing audit-allowlist test for the new keys, added a collapsible sample-employee panel + sampled pill + coverage card to the finance page, and added a Follow-up section listing deferred work (`batch_status`/`completed_at`/`expected_resource_count` schema additions, `/api/v1/ctc/snapshot?as_of=…` endpoint, `?mode=all` payroll-row mode, employee-directory autocomplete).
- Backend code-review patch pass (2026-05-20): Applied all seven backend review findings: validation-report queries now run inside the established RLS transaction context; BPJS regulation checks recalculate Xynergy and payroll expected values from each side's own salary basis; legacy rows with missing encryption metadata are excluded instead of decode-failing the report; missing/invalid risk-tier metadata is surfaced without hiding plain-field discrepancies; sampled reports validate every requested employee against visible resource, CTC, and payroll source rows; payroll coverage uses the CTC/payroll intersection; payroll freshness is checked on selected baseline rows. Added backend regressions for payroll-basis BPJS checks, invalid risk-tier behavior, mixed sampled payroll gaps, selected-row freshness, payroll-only coverage, and legacy encryption metadata.
- Frontend code-review patch pass (2026-05-20): Applied all three frontend review findings: pagination now uses the successfully loaded filter snapshot and leaves the previous page intact on failed detail-page loads; changed filters disable detail pagination until the report is rerun; backend error envelopes are parsed into sanitized user-facing messages instead of rendering raw JSON; Tailwind CSS was regenerated with the repo-local 4.1.18 CLI after updating `package-lock.json` to include the CLI dependency already declared in `package.json`.
- Artifact code-review patch pass (2026-05-20): Synchronized Story 5.3 traceability/gate artifacts with resolved decisions, current 36 integration / 9 service unit test inventory, and the current conservative gate rationale: release approval still needs final live-DB execution of the expanded integration suite.

### File List

- `_bmad-output/implementation-artifacts/5-3-ctc-validation-reports.md` - story artifact with implementation outcomes and decision-resolution notes
- `_bmad-output/implementation-artifacts/sprint-status.yaml` - story lifecycle tracking synced to `done`
- `_bmad-output/test-artifacts/automation-summary.md` - test automation summary updated for final review-patch inventory and verification boundaries
- `_bmad-output/test-artifacts/traceability/5-3-ctc-validation-reports-traceability.md` - traceability report updated for resolved decisions, 36 integration / 9 unit tests, and conservative gate rationale
- `_bmad-output/test-artifacts/traceability/5-3-e2e-trace-summary.json` - machine-readable trace summary updated for current counts and blockers
- `_bmad-output/test-artifacts/traceability/5-3-gate-decision.json` - gate decision rationale updated; still `CONCERNS` pending live-DB execution of the final suite
- `migrations/20260307100000_add_payroll_validation_staging.up.sql` - payroll staging table with indexes
- `migrations/20260307100000_add_payroll_validation_staging.down.sql` - rollback migration
- `src/backend/src/services/ctc_calculator.rs` - shared JKK risk-tier mapping exported for compliance and validation services
- `src/backend/src/services/compliance_report.rs` - reuses shared JKK risk-tier mapping
- `src/backend/src/services/ctc_validation_report.rs` - validation/reconciliation report service with sampling, payroll freshness check, coverage metric, deterministic ordering, capped excluded details, and BPJS missing-field counting
- `src/backend/src/services/mod.rs` - re-exports updated for new public constants (`PAYROLL_FRESHNESS_DAYS`, `MAX_SAMPLED_EMPLOYEE_IDS`) and shared JKK helper
- `src/backend/src/routes/ctc.rs` - validation-report route accepts deduped comma-separated `employee_ids`; audit payload now records `sampled`, `sampled_count`, `payroll_coverage_pct`, and `bpjs_error_count`
- `src/frontend/src/pages/ctc_validation.rs` - finance CTC Validation page now includes collapsible sample-employee panel, sampled-run pill, payroll-coverage summary card, stable paginated details, sample validation/encoding, a11y table metadata, and excluded employee IDs
- `src/frontend/src/pages/mod.rs` - registered and re-exported new page
- `src/frontend/src/lib.rs` - registered `/finance/ctc-validation` route and frontend recursion-limit setting needed for hydrate build
- `src/frontend/src/components/app_sidebar.rs` - Finance sidebar entry for CTC Validation
- `src/frontend/package-lock.json` - locks the local Tailwind CLI/PostCSS packages already declared in `package.json`
- `src/frontend/public/output.css` - regenerated Tailwind output after Story 5.3 page classes
- `src/backend/tests/ctc_validation_report_tests.rs` - integration suite extended with sampling (happy + invalid + audit + dedupe), sample-scoped payroll validation, payroll freshness (400), and coverage (50% case) tests; existing audit-allowlist test updated for new keys
