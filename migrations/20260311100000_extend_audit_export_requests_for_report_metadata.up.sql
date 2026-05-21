-- Extend audit_export_requests to capture report-specific export metadata
-- needed for the Story 5.4 four-eyes approval workflow.
ALTER TABLE audit_export_requests
    ADD COLUMN IF NOT EXISTS report_type VARCHAR(50),
    ADD COLUMN IF NOT EXISTS filters JSONB,
    ADD COLUMN IF NOT EXISTS watermark JSONB;

CREATE INDEX IF NOT EXISTS idx_audit_export_requests_report_type
    ON audit_export_requests(report_type);
