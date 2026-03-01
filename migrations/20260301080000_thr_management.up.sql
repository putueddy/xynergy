-- Story 2.3: THR (Tunjangan Hari Raya) Management
-- Extends CTC records with THR configuration and creates accrual tracking table

-- 1. Add THR configuration columns to ctc_records
ALTER TABLE ctc_records
    ADD COLUMN IF NOT EXISTS thr_eligible BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS thr_calculation_basis VARCHAR(20) NOT NULL DEFAULT 'full';

-- Constrain THR calculation basis values
ALTER TABLE ctc_records
    ADD CONSTRAINT chk_thr_calculation_basis
    CHECK (thr_calculation_basis IN ('full', 'prorated'));

-- 2. Create thr_accruals table for monthly accrual tracking
-- Accruals are idempotent per resource per period (YYYY-MM)
CREATE TABLE IF NOT EXISTS thr_accruals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource_id UUID NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    accrual_period VARCHAR(7) NOT NULL,  -- 'YYYY-MM' format for idempotency

    -- Operational columns (unencrypted, needed for queries/reports)
    service_months_at_accrual INTEGER NOT NULL,
    calculation_basis VARCHAR(20) NOT NULL,

    -- Encrypted amounts (THR-sensitive data under CTC encryption model)
    encrypted_accrual_amount TEXT NOT NULL,
    encrypted_annual_entitlement TEXT NOT NULL,

    -- Encryption metadata (consistent with ctc_records/ctc_revisions pattern)
    key_version VARCHAR(50) NOT NULL,
    encryption_version VARCHAR(50) NOT NULL,
    encryption_algorithm VARCHAR(100) NOT NULL,
    encrypted_at TIMESTAMPTZ NOT NULL,

    -- Audit fields
    accrued_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Idempotency: one accrual per resource per period
    CONSTRAINT uq_thr_accruals_resource_period UNIQUE (resource_id, accrual_period)
);

-- 3. Constraint for calculation basis in accruals
ALTER TABLE thr_accruals
    ADD CONSTRAINT chk_thr_accrual_basis
    CHECK (calculation_basis IN ('full', 'prorated'));

-- 4. Encryption consistency check (all-or-nothing, matching CTC pattern)
ALTER TABLE thr_accruals
    ADD CONSTRAINT chk_thr_accrual_encryption_consistent
    CHECK (
        encrypted_accrual_amount IS NOT NULL
        AND encrypted_annual_entitlement IS NOT NULL
        AND key_version IS NOT NULL
        AND encryption_version IS NOT NULL
        AND encryption_algorithm IS NOT NULL
        AND encrypted_at IS NOT NULL
    );

-- 5. Indexes for period-based accrual and report queries
CREATE INDEX IF NOT EXISTS idx_thr_accruals_resource_period
    ON thr_accruals(resource_id, accrual_period DESC);
CREATE INDEX IF NOT EXISTS idx_thr_accruals_period
    ON thr_accruals(accrual_period);
CREATE INDEX IF NOT EXISTS idx_thr_accruals_resource_id
    ON thr_accruals(resource_id);

-- 6. Index for THR eligibility queries on ctc_records
CREATE INDEX IF NOT EXISTS idx_ctc_records_thr_eligible
    ON ctc_records(thr_eligible) WHERE thr_eligible = true;
