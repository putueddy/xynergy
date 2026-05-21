## Deferred from: code review of 5-4-compliance-audit-reports (2026-05-21)

- Extract shared CTC revision decrypt/diff helpers for `routes/ctc.rs` and `services/compliance_audit_report.rs`; Claude routed this as non-blocking hygiene for Story 5.4 because the two call sites intentionally diverge on scope, redaction, baseline, and output shape.
- Add sealed export artifact integrity for the post-approval/download workflow: materialized rows or content hashes should be produced when the future approval step renders bytes. Claude routed this out of Story 5.4 MVP because this story only creates pending approval metadata and does not implement approved downloads.
- Optimize CTC Change Log pagination for high-volume revision windows. Current interactive windows are capped at 90 days, so Claude routed cursor/streaming decryption as a deferred performance refactor rather than a Story 5.4 blocker.
- Make cash-flow entry creation atomic with its audit log. Current `create_cash_flow_entry` inserts the business row and then writes the audit entry in a separate transaction; a future cash-flow hardening pass should wrap both with the transaction-aware audit logger.
- Add inverted date-range validation to `GET /cash-flow/entries`. The dashboard rejects inverted ranges, but the list endpoint currently returns an empty 200 response.
