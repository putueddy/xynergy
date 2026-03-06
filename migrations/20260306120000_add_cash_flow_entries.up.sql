CREATE TABLE IF NOT EXISTS cash_flow_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entry_type TEXT NOT NULL CHECK (entry_type IN ('cash_in', 'cash_out')),
    category TEXT NOT NULL,
    amount_idr BIGINT NOT NULL CHECK (amount_idr > 0),
    entry_date DATE NOT NULL,
    description TEXT NOT NULL,
    project_id UUID NULL REFERENCES projects(id) ON DELETE SET NULL,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT valid_category CHECK (
        (entry_type = 'cash_in' AND category IN ('client_payment', 'interest', 'other_income'))
        OR
        (entry_type = 'cash_out' AND category IN ('payroll', 'vendor_payment', 'expense', 'tax'))
    )
);

CREATE INDEX IF NOT EXISTS idx_cash_flow_entries_date ON cash_flow_entries (entry_date DESC);
CREATE INDEX IF NOT EXISTS idx_cash_flow_entries_project_date ON cash_flow_entries (project_id, entry_date DESC);
CREATE INDEX IF NOT EXISTS idx_cash_flow_entries_type_date ON cash_flow_entries (entry_type, entry_date DESC);
