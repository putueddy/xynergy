-- Extend ctc_records with full component breakdown and derived calculations
ALTER TABLE ctc_records 
    ADD COLUMN IF NOT EXISTS base_salary NUMERIC(12,0) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS hra_allowance NUMERIC(12,0) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS medical_allowance NUMERIC(12,0) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS transport_allowance NUMERIC(12,0) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS meal_allowance NUMERIC(12,0) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS bpjs_kesehatan NUMERIC(12,0) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS bpjs_ketenagakerjaan NUMERIC(12,0) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS thr_monthly_accrual NUMERIC(12,0) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS total_monthly_ctc NUMERIC(12,0) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS daily_rate NUMERIC(12,2) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS working_days_per_month INTEGER NOT NULL DEFAULT 22,
    ADD COLUMN IF NOT EXISTS effective_date DATE NOT NULL DEFAULT CURRENT_DATE,
    ADD COLUMN IF NOT EXISTS status VARCHAR(20) NOT NULL DEFAULT 'Active',
    ADD COLUMN IF NOT EXISTS created_by UUID REFERENCES users(id),
    ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP;

-- Add indexes for common queries
CREATE INDEX IF NOT EXISTS idx_ctc_records_resource_id ON ctc_records(resource_id);
CREATE INDEX IF NOT EXISTS idx_ctc_records_status ON ctc_records(status);
CREATE INDEX IF NOT EXISTS idx_ctc_records_effective_date ON ctc_records(effective_date);

-- Add constraint to ensure status values are valid
ALTER TABLE ctc_records 
    ADD CONSTRAINT chk_ctc_status 
    CHECK (status IN ('Active', 'Inactive', 'Pending'));

-- Note: base_salary > 0 constraint removed to maintain backward compatibility
-- with existing CTC update endpoint that only sets components JSON
-- The create endpoint (Story 2.1) enforces positive base_salary at API level
