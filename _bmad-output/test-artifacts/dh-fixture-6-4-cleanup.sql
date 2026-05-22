-- Cleanup for DH Fixture (Story 6.4 manual verification)
-- Paired with: dh-fixture-6-4.sql
-- Generated 2026-05-22T11:00:00Z
--
-- Removes the team-shaped fixture data in FK-safe order:
--   1. allocations (5 fixture rows)
--   2. ctc_records (3 fixture rows)
--   3. resources (4 fixture rows)
--   4. project (1 fixture row)
--   5. department_budgets (fixture rows for the fixture department)
--   6. NULL departments.head_id (releases user FK on the dept)
--   7. Disable and detach the DH test login (releases dept FK on the user)
--   8. Department
--
-- NOT removed (intentionally — compliance):
--   - The DH user (`dh-6-4@xynergy.test`, id 44444444-…) stays in `users` because
--     `audit_logs.user_id` points at it and `audit_logs` has a database-level
--     immutability trigger (`prevent_audit_log_modification`) that prohibits any
--     DELETE/UPDATE — this is correct append-only behavior per project policy.
--   - Audit log breadcrumbs from the live verification (login, etc.) are kept.
--
--   The leftover user is orphaned, has no department/resources/allocations, and
--   its known fixture login is disabled by randomizing the email while keeping
--   a parseable Argon2 hash. Re-running `dh-fixture-6-4.sql` will
--   restore the fixture email/password and re-attach it to a fresh fixture dept via
--   ON CONFLICT, so future re-runs are idempotent.
--
-- Safe to run repeatedly. Reports a sanity row at the end.

BEGIN;

DO $$
BEGIN
  IF current_setting('xynergy.fixture_allow_seed', true) IS DISTINCT FROM 'local-only' THEN
    RAISE EXCEPTION 'Refusing to clean dh-fixture-6-4: fixture cleanup should only run against a local dev DB. Set xynergy.fixture_allow_seed=local-only in the same psql session to confirm.';
  END IF;
END
$$;

-- 1. Allocations
DELETE FROM allocations WHERE id IN (
  '55555555-5555-5555-5555-555555555501',
  '55555555-5555-5555-5555-555555555502',
  '55555555-5555-5555-5555-555555555503',
  '55555555-5555-5555-5555-555555555504',
  '55555555-5555-5555-5555-555555555505'
);

-- 2. CTC records
DELETE FROM ctc_records WHERE resource_id IN (
  '33333333-3333-3333-3333-333333333301',
  '33333333-3333-3333-3333-333333333302',
  '33333333-3333-3333-3333-333333333304'
);

-- 3. Resources
DELETE FROM resources WHERE id IN (
  '33333333-3333-3333-3333-333333333301',
  '33333333-3333-3333-3333-333333333302',
  '33333333-3333-3333-3333-333333333303',
  '33333333-3333-3333-3333-333333333304'
);

-- 4. Project
DELETE FROM projects WHERE id = '22222222-2222-2222-2222-222222222222';

-- 5. Department budget
DELETE FROM department_budgets WHERE department_id = '11111111-1111-1111-1111-111111111111';

-- 6. Break departments.head_id FK pointing at the DH user
UPDATE departments SET head_id = NULL WHERE id = '11111111-1111-1111-1111-111111111111';

-- 7. Break users.department_id FK so we can delete the department, and disable
-- the known fixture login that the setup script intentionally creates.
UPDATE users
   SET department_id = NULL,
       email = 'dh-6-4-disabled-' || uuid_generate_v4()::text || '@xynergy.test',
       password_hash = '$argon2id$v=19$m=19456,t=2,p=1$TuU9ec2LemkiXej31B5K/Q$2QSjGT39zeovfY+blru6GatkuTqSxuwwfgCQZUYRaD4'
 WHERE id = '44444444-4444-4444-4444-444444444444';

-- 8. Department
DELETE FROM departments WHERE id = '11111111-1111-1111-1111-111111111111';

COMMIT;

-- Sanity: every "remaining" count should be 0 except dh_user_orphaned (=1, intentional).
SELECT
  (SELECT count(*) FROM users         WHERE id = '44444444-4444-4444-4444-444444444444') AS dh_user_orphaned,
  (SELECT count(*) FROM departments   WHERE id = '11111111-1111-1111-1111-111111111111') AS dept_remaining,
  (SELECT count(*) FROM resources     WHERE id IN (
    '33333333-3333-3333-3333-333333333301',
    '33333333-3333-3333-3333-333333333302',
    '33333333-3333-3333-3333-333333333303',
    '33333333-3333-3333-3333-333333333304'
  )) AS resources_remaining,
  (SELECT count(*) FROM projects      WHERE id = '22222222-2222-2222-2222-222222222222') AS project_remaining,
  (SELECT count(*) FROM allocations   WHERE id IN (
    '55555555-5555-5555-5555-555555555501',
    '55555555-5555-5555-5555-555555555502',
    '55555555-5555-5555-5555-555555555503',
    '55555555-5555-5555-5555-555555555504',
    '55555555-5555-5555-5555-555555555505'
  )) AS allocations_remaining,
  (SELECT count(*) FROM ctc_records   WHERE resource_id IN (
    '33333333-3333-3333-3333-333333333301',
    '33333333-3333-3333-3333-333333333302',
    '33333333-3333-3333-3333-333333333304'
  )) AS ctc_remaining,
  (SELECT count(*) FROM department_budgets WHERE department_id = '11111111-1111-1111-1111-111111111111') AS budget_remaining;
