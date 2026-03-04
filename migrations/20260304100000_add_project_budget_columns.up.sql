-- Story 4.1: Project Budget Setup
-- Add budget columns and client field to projects table.
-- Uses typed BIGINT columns (not JSONB) for DB-level constraint enforcement
-- and sqlx compile-time safety.

ALTER TABLE projects
  ADD COLUMN IF NOT EXISTS client TEXT,
  ADD COLUMN IF NOT EXISTS total_budget_idr BIGINT NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS budget_hr_idr BIGINT NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS budget_software_idr BIGINT NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS budget_hardware_idr BIGINT NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS budget_overhead_idr BIGINT NOT NULL DEFAULT 0;

-- Constraint: categories must sum to total
ALTER TABLE projects ADD CONSTRAINT chk_budget_sum
  CHECK (budget_hr_idr + budget_software_idr + budget_hardware_idr + budget_overhead_idr = total_budget_idr);

-- All budget values non-negative
ALTER TABLE projects ADD CONSTRAINT chk_budget_nonneg
  CHECK (total_budget_idr >= 0 AND budget_hr_idr >= 0 AND budget_software_idr >= 0
         AND budget_hardware_idr >= 0 AND budget_overhead_idr >= 0);
