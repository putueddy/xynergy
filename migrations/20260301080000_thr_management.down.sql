-- Rollback Story 2.3: THR Management

-- Drop thr_accruals table (includes all constraints and indexes)
DROP TABLE IF EXISTS thr_accruals;

-- Remove THR-specific index on ctc_records
DROP INDEX IF EXISTS idx_ctc_records_thr_eligible;

-- Remove THR configuration columns from ctc_records
ALTER TABLE ctc_records
    DROP CONSTRAINT IF EXISTS chk_thr_calculation_basis,
    DROP COLUMN IF EXISTS thr_eligible,
    DROP COLUMN IF EXISTS thr_calculation_basis;
