CREATE TABLE IF NOT EXISTS ctc_records (
    resource_id UUID PRIMARY KEY REFERENCES resources(id) ON DELETE CASCADE,
    components JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_by UUID REFERENCES users(id),
    reason TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_ctc_records_updated_at ON ctc_records(updated_at DESC);

CREATE TABLE IF NOT EXISTS audit_export_requests (
    id UUID PRIMARY KEY,
    requested_by UUID NOT NULL REFERENCES users(id),
    status VARCHAR(40) NOT NULL,
    note TEXT NOT NULL,
    approved_by UUID REFERENCES users(id),
    approved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_audit_export_requests_status ON audit_export_requests(status);
CREATE INDEX IF NOT EXISTS idx_audit_export_requests_created ON audit_export_requests(created_at DESC);
