-- Rollback Story 4.2: Non-Resource Cost Entry
DROP INDEX IF EXISTS idx_project_expenses_project_category;
DROP INDEX IF EXISTS idx_project_expenses_project_date;
DROP INDEX IF EXISTS idx_project_expenses_project_id;
DROP TABLE IF EXISTS project_expenses;
