-- Create ctc_revisions table for append-only audit trail and timeline
CREATE TABLE IF NOT EXISTS ctc_revisions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource_id UUID NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    revision_number INTEGER NOT NULL,
    
    -- Encryption metadata (copied from the ctc_records state at revision time)
    key_version VARCHAR(50) NOT NULL,
    encryption_version VARCHAR(50) NOT NULL,
    encryption_algorithm VARCHAR(100) NOT NULL,
    encrypted_at TIMESTAMPTZ NOT NULL,
    
    -- Encrypted payloads only
    encrypted_components TEXT NOT NULL,
    encrypted_daily_rate TEXT, -- Text, can be null if not computed or matching ctc_records
    
    -- Unencrypted operational columns
    effective_date_policy VARCHAR(50) NOT NULL DEFAULT 'pro_rata',
    effective_date DATE NOT NULL,
    working_days_per_month INTEGER NOT NULL,
    status VARCHAR(20) NOT NULL,
    
    -- Audit fields
    changed_by UUID NOT NULL REFERENCES users(id),
    reason TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- A resource cannot have duplicate revision numbers
    CONSTRAINT uq_ctc_revisions_resource_number UNIQUE (resource_id, revision_number)
);

-- Index for timeline queries
CREATE INDEX IF NOT EXISTS idx_ctc_revisions_timeline ON ctc_revisions(resource_id, created_at DESC);
