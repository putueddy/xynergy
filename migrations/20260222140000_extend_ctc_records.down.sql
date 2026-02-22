-- Revert ctc_records extension
ALTER TABLE ctc_records 
    DROP COLUMN IF EXISTS base_salary,
    DROP COLUMN IF EXISTS hra_allowance,
    DROP COLUMN IF EXISTS medical_allowance,
    DROP COLUMN IF EXISTS transport_allowance,
    DROP COLUMN IF EXISTS meal_allowance,
    DROP COLUMN IF EXISTS bpjs_kesehatan,
    DROP COLUMN IF EXISTS bpjs_ketenagakerjaan,
    DROP COLUMN IF EXISTS thr_monthly_accrual,
    DROP COLUMN IF EXISTS total_monthly_ctc,
    DROP COLUMN IF EXISTS daily_rate,
    DROP COLUMN IF EXISTS working_days_per_month,
    DROP COLUMN IF EXISTS effective_date,
    DROP COLUMN IF EXISTS status,
    DROP COLUMN IF EXISTS created_by,
    DROP COLUMN IF EXISTS created_at;

DROP INDEX IF EXISTS idx_ctc_records_resource_id;
DROP INDEX IF EXISTS idx_ctc_records_status;
DROP INDEX IF EXISTS idx_ctc_records_effective_date;
