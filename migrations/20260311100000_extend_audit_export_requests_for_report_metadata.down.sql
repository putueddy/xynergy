DROP INDEX IF EXISTS idx_audit_export_requests_report_type;

ALTER TABLE audit_export_requests
    DROP COLUMN IF EXISTS watermark,
    DROP COLUMN IF EXISTS filters,
    DROP COLUMN IF EXISTS report_type;
