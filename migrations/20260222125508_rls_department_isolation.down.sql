-- Revert RLS
ALTER TABLE resources DISABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS admin_all_policy ON resources;
DROP POLICY IF EXISTS hr_read_policy ON resources;
DROP POLICY IF EXISTS dept_head_policy ON resources;
DROP POLICY IF EXISTS standard_roles_policy ON resources;
DROP POLICY IF EXISTS insert_all_policy ON resources;
DROP POLICY IF EXISTS update_all_policy ON resources;
DROP POLICY IF EXISTS delete_all_policy ON resources;
