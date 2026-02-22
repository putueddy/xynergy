-- Add hash chain columns to audit_logs
ALTER TABLE audit_logs
    ADD COLUMN previous_hash TEXT,
    ADD COLUMN entry_hash TEXT;

-- Create an index that might be useful for chain traversal
CREATE INDEX idx_audit_logs_entry_hash ON audit_logs(entry_hash);

-- Implement DB-level append-only protection for audit_logs ensuring immutable history
CREATE OR REPLACE FUNCTION prevent_audit_log_modification()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'audit_logs is append-only. UPDATE and DELETE are prohibited to ensure audit integrity.';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER audit_logs_append_only
    BEFORE UPDATE OR DELETE ON audit_logs
    FOR EACH ROW
    EXECUTE FUNCTION prevent_audit_log_modification();
