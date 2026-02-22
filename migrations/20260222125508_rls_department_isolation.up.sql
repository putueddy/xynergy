-- Enable and configure RLS for resources table to isolate by department

ALTER TABLE resources ENABLE ROW LEVEL SECURITY;
ALTER TABLE resources FORCE ROW LEVEL SECURITY;

-- 1. Admin gets access to all records
CREATE POLICY admin_all_policy ON resources
    FOR ALL
    USING (current_setting('app.current_role', true) = 'admin');

-- 2. HR gets read access to all records (for cross-department CTC visibility)
CREATE POLICY hr_read_policy ON resources
    FOR SELECT
    USING (current_setting('app.current_role', true) = 'hr');

-- 3. Department Head gets access only to their own department's records
CREATE POLICY dept_head_policy ON resources
    FOR SELECT
    USING (
        current_setting('app.current_role', true) = 'department_head' 
        AND 
        department_id::text = current_setting('app.current_department_id', true)
    );

-- 4. Other operational roles are also constrained by department isolation
CREATE POLICY standard_roles_policy ON resources
    FOR SELECT
    USING (
        current_setting('app.current_role', true) IN ('project_manager', 'finance')
        AND department_id::text = current_setting('app.current_department_id', true)
    );

-- 5. Fallback policies for INSERT, UPDATE, DELETE to not break existing writes
-- (Access control for mutations happens at the application layer)
CREATE POLICY insert_all_policy ON resources FOR INSERT WITH CHECK (true);
CREATE POLICY update_all_policy ON resources FOR UPDATE USING (true) WITH CHECK (true);
CREATE POLICY delete_all_policy ON resources FOR DELETE USING (true);
