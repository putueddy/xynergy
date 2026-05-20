# Story 5.3: CTC Validation Reports

Status: ready-for-dev

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

- [ ] **Task 1: Define the report contract and data source boundary** (AC: #1, #2, #3, #4)
  - [ ] Introduce the MVP payroll comparison source as a local staging/import table owned by this story rather than leaving the source undefined. Recommended table shape:
    - [ ] `resource_id UUID`
    - [ ] `effective_date DATE`
    - [ ] `base_salary BIGINT`
    - [ ] allowance/BPJS comparison columns needed by the report
    - [ ] `import_batch_id UUID`
    - [ ] `imported_at TIMESTAMP`
  - [ ] Add indexes needed for report performance, at minimum on `(effective_date)` and `(resource_id, effective_date)` for the payroll staging source.
  - [ ] Add explicit backend DTOs for report filters and response payloads, including summary metrics and mismatch rows.
  - [ ] Keep canonical mismatch fields explicit: `employee_id`, `employee_name`, `field_name`, `xynergy_value`, `payroll_value`, `variance_amount`, `status`, plus optional BPJS-specific metadata.
  - [ ] Add summary fields that make skipped data visible to Finance: `total_compared`, `total_matches`, `total_discrepancies`, `match_rate_pct`, and `excluded_count`.
  - [ ] Reject inverted date ranges early with `AppError::Validation`.
  - [ ] Validate the payroll source before comparison begins: fail fast if the selected range has no staging/import data, if required employees are missing, or if the import batch is stale/incomplete for the requested period.

- [ ] **Task 2: Build backend validation-report service by extending existing compliance foundations** (AC: #1, #2, #3, #4)
  - [ ] Prefer extending shared helpers in `src/backend/src/services/compliance_report.rs` first; only create `src/backend/src/services/ctc_validation_report.rs` if a separate orchestration file keeps responsibilities clearer without duplicating BPJS/decryption logic.
  - [ ] Reuse `DefaultCtcCryptoService` and existing encrypted CTC read patterns from `compliance_report.rs`; do not duplicate decryption logic.
  - [ ] Reuse `calculate_bpjs()` and existing JKK risk-tier handling for AC #4; do not reimplement BPJS formulas.
  - [ ] Compare decrypted Xynergy CTC values against payroll-source values field by field and compute:
    - [ ] `total_compared`
    - [ ] `total_matches`
    - [ ] `total_discrepancies`
    - [ ] `match_rate_pct`
    - [ ] `excluded_count`
  - [ ] Return stable, deterministic mismatch ordering (employee name ASC, field name ASC).
  - [ ] Do not silently skip records that cannot be decrypted or lack payroll data; surface them through `excluded_count` and, where useful, an explicit excluded/skipped reason list.
  - [ ] Keep the existing BPJS-only `/api/v1/ctc/compliance-report` behavior intact; the new validation report should be additive and may call shared compliance helpers internally.

- [ ] **Task 3: Expose finance-facing API endpoints with audit coverage** (AC: #1, #2, #3, #4)
  - [ ] Add endpoint(s) under existing CTC route space, e.g.:
    - [ ] `GET /api/v1/ctc/validation-report?start_date=YYYY-MM-DD&end_date=YYYY-MM-DD`
    - [ ] paginated mismatch query support such as `limit` / `offset`, or a dedicated details endpoint, so large reports do not require a single oversized payload
  - [ ] Restrict access to `finance` and `admin`; do not allow `department_head` or `project_manager`.
  - [ ] Reuse finance/admin authorization style from `cash_flow.rs` and existing claims extraction helpers.
  - [ ] Log report generation to `audit_logs` with an action such as `ctc_validation_report_generated`, capturing date range and summary counts only.
  - [ ] Keep sensitive values out of audit payloads beyond what is necessary for traceability. Allowed payload fields should be limited to report metadata such as date range, `total_compared`, `total_matches`, `total_discrepancies`, `excluded_count`, and `match_rate_pct`.
  - [ ] Do not rename or remove `/api/v1/ctc/compliance-report`; this story adds `/api/v1/ctc/validation-report` as a finance-facing reconciliation endpoint.

- [ ] **Task 4: Add Finance -> CTC Validation UI using existing page patterns** (AC: #1, #2, #3, #4)
  - [ ] Prefer a dedicated finance page/route (for example `src/frontend/src/pages/ctc_validation.rs` and `/finance/ctc-validation`) rather than overloading the HR-oriented completeness dashboard.
  - [ ] Reuse existing `authenticated_get` helpers and finance page guard patterns from `cash_flow.rs`.
  - [ ] Reuse patterns from `ctc_completeness.rs`, but do not copy that page wholesale; it mixes HR completeness and compliance concerns and already contains response-parsing assumptions that should not be propagated into the new finance report page.
  - [ ] Include filter controls for start date and end date, plus a report trigger button.
  - [ ] Render summary cards for Match Rate %, Compared Records, Discrepancy Count, Excluded Count, and BPJS Error Count.
  - [ ] Render a mismatch table with keyboard-accessible drill-down/expand behavior for AC #3; if the table is interactive, use explicit expand/collapse controls with proper `aria-expanded`, `aria-controls`, and visible focus states.
  - [ ] If BPJS sample selection is UI-driven, provide a finance-safe selector or sample toggle without exposing unrelated HR-only completeness actions.
  - [ ] If report details are paginated, keep summary metrics stable while only the detail region reloads.

- [ ] **Task 5: Wire routes, navigation, and local visual conventions** (AC: #1)
  - [ ] Register the new finance page in `src/frontend/src/pages/mod.rs` and `src/frontend/src/lib.rs`.
  - [ ] Add a Finance sidebar entry for CTC Validation alongside Cash Flow.
  - [ ] Keep styling aligned with the current Huly-inspired finance surfaces and the native table/card patterns already used in `cash_flow.rs`, `projects.rs`, and `ctc_completeness.rs`.
  - [ ] Preserve WCAG-required table parity and clear labels; if any chart or visual summary is added later, keep equivalent tabular data in the DOM.

- [ ] **Task 6: Add integration and regression coverage** (AC: #1, #2, #3, #4)
  - [ ] Create `src/backend/tests/ctc_validation_report_tests.rs`.
  - [ ] Required tests:
    - [ ] finance can fetch validation report
    - [ ] admin can fetch validation report
    - [ ] hr / department_head / project_manager denied with `403`
    - [ ] inverted date range returns `400`
    - [ ] report returns summary counts and mismatch rows deterministically
    - [ ] missing or stale payroll staging data returns a clear validation error
    - [ ] BPJS validation reuses current formula outputs and flags mismatches correctly
    - [ ] excluded/skipped records are counted and reported deterministically
    - [ ] report generation creates an audit log entry
  - [ ] Re-run high-risk suites:
    - [ ] `src/backend/tests/ctc_validation_tests.rs`
    - [ ] `src/backend/tests/audit_tests.rs`
    - [ ] `src/backend/tests/cash_flow_tests.rs`
  - [ ] Verify existing `/api/v1/ctc/compliance-report` behavior and audit action remain unchanged after Story 5.3 lands.

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

- Status: ready-for-dev
- Completion note: Ultimate context engine analysis completed - comprehensive developer guide created for Story 5.3 with direct reuse paths into existing CTC compliance, audit, and finance UI infrastructure.

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

### File List

- `_bmad-output/implementation-artifacts/5-3-ctc-validation-reports.md` - this story artifact
- `_bmad-output/implementation-artifacts/sprint-status.yaml` - story lifecycle tracking updated to `ready-for-dev`
