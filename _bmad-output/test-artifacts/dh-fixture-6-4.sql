-- DH Fixture for Story 6.4 — Team Utilization Dashboard manual verification
-- Generated 2026-05-22. Idempotent (uses ON CONFLICT / fixed UUIDs).
--
-- After applying, log in as:
--   email:    dh-6-4@xynergy.test
--   password: admin123
--
-- Provisions:
--   - 1 department (DH-6-4 Fixture Dept) with the DH as head_id
--   - 1 department_head user scoped to that department
--   - 1 project (DH-6-4 Demo Project) for the assignment deep-link flow
--   - 4 employee resources scoped to the fixture department:
--       * 33...01 "Underutilized Maya"   — CTC Active,  30% in current month (UNDERUTILIZED < 50%)
--       * 33...02 "Healthy Arjun"        — CTC Active,  85% in current month (HEALTHY)
--       * 33...03 "CTC-Missing Pria"     — NO CTC row,  20% in current month (UNDERUTILIZED but assign DISABLED)
--       * 33...04 "Overallocated Sari"   — CTC Active, 70%+70% overlap     (OVERALLOCATED → clamp 0% available)
--   - 1 department_budget for the current database month with 80% alert threshold

BEGIN;

DO $$
BEGIN
  IF current_setting('xynergy.fixture_allow_seed', true) IS DISTINCT FROM 'local-only' THEN
    RAISE EXCEPTION 'Refusing to seed dh-fixture-6-4: this fixture creates a known weak credential and must only run against a local dev DB. Set xynergy.fixture_allow_seed=local-only in the same psql session to confirm.';
  END IF;
END
$$;

-- 1. Department --------------------------------------------------------------
INSERT INTO departments (id, name)
VALUES ('11111111-1111-1111-1111-111111111111', 'DH-6-4 Fixture Dept')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name;

-- 2. Department Head user ----------------------------------------------------
-- password 'admin123' hashed with the backend's argon2 default params
INSERT INTO users (id, email, password_hash, first_name, last_name, role, department_id)
VALUES (
  '44444444-4444-4444-4444-444444444444',
  'dh-6-4@xynergy.test',
  '$argon2id$v=19$m=19456,t=2,p=1$TuU9ec2LemkiXej31B5K/Q$2QSjGT39zeovfY+blru6GatkuTqSxuwwfgCQZUYRaD4',
  'DH',
  'Fixture',
  'department_head',
  '11111111-1111-1111-1111-111111111111'
)
ON CONFLICT (id) DO UPDATE SET
  email = EXCLUDED.email,
  password_hash = EXCLUDED.password_hash,
  role = EXCLUDED.role,
  department_id = EXCLUDED.department_id;

-- 3. Make the DH the head of the fixture department -------------------------
UPDATE departments
   SET head_id = '44444444-4444-4444-4444-444444444444'
 WHERE id = '11111111-1111-1111-1111-111111111111';

-- 4. Project (PM = DH so dashboard assignable-projects lookup will find it) -
INSERT INTO projects (
  id, name, description, start_date, end_date, status,
  project_manager_id,
  total_budget_idr, budget_hr_idr, budget_software_idr, budget_hardware_idr, budget_overhead_idr
)
VALUES (
  '22222222-2222-2222-2222-222222222222',
  'DH-6-4 Demo Project',
  'Fixture project for AC#3 deep-link verification',
    date_trunc('month', CURRENT_DATE)::date,
    (date_trunc('month', CURRENT_DATE)::date + INTERVAL '12 months - 1 day')::date,
    'active',
  '44444444-4444-4444-4444-444444444444',
  500000000, 300000000, 100000000, 50000000, 50000000
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  start_date = EXCLUDED.start_date,
  end_date = EXCLUDED.end_date,
  status = EXCLUDED.status,
  project_manager_id = EXCLUDED.project_manager_id,
  total_budget_idr = EXCLUDED.total_budget_idr,
  budget_hr_idr = EXCLUDED.budget_hr_idr,
  budget_software_idr = EXCLUDED.budget_software_idr,
  budget_hardware_idr = EXCLUDED.budget_hardware_idr,
  budget_overhead_idr = EXCLUDED.budget_overhead_idr;

-- 5. Resources (4 employees scoped to fixture dept) -------------------------
INSERT INTO resources (id, name, resource_type, capacity, department_id, working_hours, work_start_time, work_end_time, employment_start_date)
VALUES
  ('33333333-3333-3333-3333-333333333301', 'Underutilized Maya',  'employee', 1.0, '11111111-1111-1111-1111-111111111111', 8.0, '08:00', '17:00', '2025-01-15'),
  ('33333333-3333-3333-3333-333333333302', 'Healthy Arjun',       'employee', 1.0, '11111111-1111-1111-1111-111111111111', 8.0, '08:00', '17:00', '2024-08-01'),
  ('33333333-3333-3333-3333-333333333303', 'CTC-Missing Pria',    'employee', 1.0, '11111111-1111-1111-1111-111111111111', 8.0, '08:00', '17:00', '2026-04-10'),
  ('33333333-3333-3333-3333-333333333304', 'Overallocated Sari',  'employee', 1.0, '11111111-1111-1111-1111-111111111111', 8.0, '08:00', '17:00', '2024-02-01')
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  resource_type = EXCLUDED.resource_type,
  department_id = EXCLUDED.department_id;

-- 6. CTC records (Active) for 3 of 4. Skip CTC-Missing Pria deliberately ---
--    Leave encrypted_* columns NULL (the chk_ctc_encryption_metadata_consistent
--    constraint is satisfied when they are all NULL together).
INSERT INTO ctc_records (
  resource_id, components, daily_rate, working_days_per_month,
  effective_date, status, created_by, updated_by, reason
)
VALUES
  ('33333333-3333-3333-3333-333333333301', '{}'::jsonb, 1200000, 22, '2026-01-01', 'Active', '44444444-4444-4444-4444-444444444444', '44444444-4444-4444-4444-444444444444', 'DH-6-4 fixture'),
  ('33333333-3333-3333-3333-333333333302', '{}'::jsonb, 1500000, 22, '2026-01-01', 'Active', '44444444-4444-4444-4444-444444444444', '44444444-4444-4444-4444-444444444444', 'DH-6-4 fixture'),
  ('33333333-3333-3333-3333-333333333304', '{}'::jsonb, 1300000, 22, '2026-01-01', 'Active', '44444444-4444-4444-4444-444444444444', '44444444-4444-4444-4444-444444444444', 'DH-6-4 fixture')
ON CONFLICT (resource_id) DO UPDATE SET
  status = EXCLUDED.status,
  daily_rate = EXCLUDED.daily_rate,
  reason = EXCLUDED.reason;

-- 7. Allocations (current database month) ------------------------------------
-- Wipe and reinsert with fixed UUIDs to keep idempotent.
DELETE FROM allocations
 WHERE resource_id IN (
   '33333333-3333-3333-3333-333333333301',
   '33333333-3333-3333-3333-333333333302',
   '33333333-3333-3333-3333-333333333303',
   '33333333-3333-3333-3333-333333333304'
 );

INSERT INTO allocations (id, project_id, resource_id, start_date, end_date, allocation_percentage, created_by)
VALUES
  -- Maya: 30% all month → underutilized
  ('55555555-5555-5555-5555-555555555501', '22222222-2222-2222-2222-222222222222', '33333333-3333-3333-3333-333333333301', date_trunc('month', CURRENT_DATE)::date, (date_trunc('month', CURRENT_DATE)::date + INTERVAL '1 month - 1 day')::date, 30.00, '44444444-4444-4444-4444-444444444444'),
  -- Arjun: 85% all month → healthy
  ('55555555-5555-5555-5555-555555555502', '22222222-2222-2222-2222-222222222222', '33333333-3333-3333-3333-333333333302', date_trunc('month', CURRENT_DATE)::date, (date_trunc('month', CURRENT_DATE)::date + INTERVAL '1 month - 1 day')::date, 85.00, '44444444-4444-4444-4444-444444444444'),
  -- Pria: 20% all month → underutilized but CTC missing
  ('55555555-5555-5555-5555-555555555503', '22222222-2222-2222-2222-222222222222', '33333333-3333-3333-3333-333333333303', date_trunc('month', CURRENT_DATE)::date, (date_trunc('month', CURRENT_DATE)::date + INTERVAL '1 month - 1 day')::date, 20.00, '44444444-4444-4444-4444-444444444444'),
  -- Sari: two overlapping allocations at 70% + 70% = 140% → overallocated
  ('55555555-5555-5555-5555-555555555504', '22222222-2222-2222-2222-222222222222', '33333333-3333-3333-3333-333333333304', date_trunc('month', CURRENT_DATE)::date, (date_trunc('month', CURRENT_DATE)::date + INTERVAL '1 month - 1 day')::date, 70.00, '44444444-4444-4444-4444-444444444444'),
  ('55555555-5555-5555-5555-555555555505', '22222222-2222-2222-2222-222222222222', '33333333-3333-3333-3333-333333333304', (date_trunc('month', CURRENT_DATE)::date + INTERVAL '14 days')::date, (date_trunc('month', CURRENT_DATE)::date + INTERVAL '1 month - 1 day')::date, 70.00, '44444444-4444-4444-4444-444444444444');

-- 8. Department budget (current database month) -----------------------------
INSERT INTO department_budgets (department_id, budget_period, total_budget_idr, alert_threshold_pct)
VALUES ('11111111-1111-1111-1111-111111111111', to_char(CURRENT_DATE, 'YYYY-MM'), 1000000000, 80)
ON CONFLICT (department_id, budget_period) DO UPDATE SET
  total_budget_idr = EXCLUDED.total_budget_idr,
  alert_threshold_pct = EXCLUDED.alert_threshold_pct;

COMMIT;

-- Sanity check
SELECT
  (SELECT count(*) FROM users         WHERE id = '44444444-4444-4444-4444-444444444444') AS dh_user_present,
  (SELECT head_id::text FROM departments WHERE id = '11111111-1111-1111-1111-111111111111') AS dept_head_id,
  (SELECT count(*) FROM resources     WHERE department_id = '11111111-1111-1111-1111-111111111111') AS scoped_resources,
  (SELECT count(*) FROM ctc_records   WHERE resource_id::text LIKE '33333333-3333-3333-3333-33333333330_') AS ctc_rows,
  (SELECT count(*) FROM allocations   WHERE resource_id::text LIKE '33333333-3333-3333-3333-33333333330_') AS allocations,
  (SELECT total_budget_idr FROM department_budgets WHERE department_id = '11111111-1111-1111-1111-111111111111' AND budget_period = to_char(CURRENT_DATE, 'YYYY-MM')) AS budget_idr;
