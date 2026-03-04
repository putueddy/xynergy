-- Rollback Story 4.1: Project Budget Setup
-- Drop constraints first, then columns.

ALTER TABLE projects DROP CONSTRAINT IF EXISTS chk_budget_sum;
ALTER TABLE projects DROP CONSTRAINT IF EXISTS chk_budget_nonneg;

ALTER TABLE projects
  DROP COLUMN IF EXISTS budget_overhead_idr,
  DROP COLUMN IF EXISTS budget_hardware_idr,
  DROP COLUMN IF EXISTS budget_software_idr,
  DROP COLUMN IF EXISTS budget_hr_idr,
  DROP COLUMN IF EXISTS total_budget_idr,
  DROP COLUMN IF EXISTS client;
