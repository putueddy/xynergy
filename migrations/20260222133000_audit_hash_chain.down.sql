DROP TRIGGER IF EXISTS audit_logs_append_only ON audit_logs;
DROP FUNCTION IF EXISTS prevent_audit_log_modification();
DROP INDEX IF EXISTS idx_audit_logs_entry_hash;

ALTER TABLE audit_logs 
    DROP COLUMN IF EXISTS entry_hash,
    DROP COLUMN IF EXISTS previous_hash;
