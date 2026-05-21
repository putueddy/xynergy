-- Payroll validation staging table for CTC validation reports (Story 5.3)
-- Stores payroll-side records imported as batches for finance reconciliation
-- against Xynergy CTC data. Currency values are IDR whole numbers (BIGINT).

CREATE TABLE IF NOT EXISTS payroll_validation_staging (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    import_batch_id UUID NOT NULL,
    resource_id UUID NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    effective_date DATE NOT NULL,
    base_salary BIGINT NOT NULL CHECK (base_salary >= 0),
    hra_allowance BIGINT NOT NULL DEFAULT 0 CHECK (hra_allowance >= 0),
    medical_allowance BIGINT NOT NULL DEFAULT 0 CHECK (medical_allowance >= 0),
    transport_allowance BIGINT NOT NULL DEFAULT 0 CHECK (transport_allowance >= 0),
    meal_allowance BIGINT NOT NULL DEFAULT 0 CHECK (meal_allowance >= 0),
    bpjs_kesehatan_employer BIGINT NOT NULL DEFAULT 0 CHECK (bpjs_kesehatan_employer >= 0),
    bpjs_ketenagakerjaan_employer BIGINT NOT NULL DEFAULT 0 CHECK (bpjs_ketenagakerjaan_employer >= 0),
    imported_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    imported_by UUID NULL REFERENCES users(id),
    notes TEXT NULL,
    UNIQUE (resource_id, effective_date, import_batch_id)
);

CREATE INDEX IF NOT EXISTS idx_payroll_validation_staging_effective_date
    ON payroll_validation_staging (effective_date);
CREATE INDEX IF NOT EXISTS idx_payroll_validation_staging_resource_date
    ON payroll_validation_staging (resource_id, effective_date);
CREATE INDEX IF NOT EXISTS idx_payroll_validation_staging_batch
    ON payroll_validation_staging (import_batch_id);
