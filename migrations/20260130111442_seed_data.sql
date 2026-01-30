-- Seed data for Xynergy
-- Sample departments, users, resources, and projects

-- Insert departments
INSERT INTO departments (id, name) VALUES
    (uuid_generate_v4(), 'Engineering'),
    (uuid_generate_v4(), 'Design'),
    (uuid_generate_v4(), 'Marketing'),
    (uuid_generate_v4(), 'Product');

-- Insert admin user (password: 'admin123' - hashed with argon2)
INSERT INTO users (id, email, password_hash, first_name, last_name, role, department_id) 
SELECT 
    uuid_generate_v4(),
    'admin@xynergy.com',
    '$argon2id$v=19$m=65536,t=3,p=4$gGVeWz5mM4uJz9v4nNq4vA$7kFQKz9v4nNq4vAgGVeWz5mM4uJz9v4nNq4vAgGVeWz5mM4uJz9v4nNq4vA', -- placeholder hash
    'Admin',
    'User',
    'admin',
    id
FROM departments WHERE name = 'Engineering';

-- Insert sample resources
INSERT INTO resources (id, name, resource_type, capacity, department_id, skills) 
SELECT 
    uuid_generate_v4(),
    'John Developer',
    'human',
    1.0,
    d.id,
    '["Rust", "PostgreSQL", "React"]'::jsonb
FROM departments d WHERE d.name = 'Engineering'
UNION ALL
SELECT 
    uuid_generate_v4(),
    'Jane Designer',
    'human',
    1.0,
    d.id,
    '["Figma", "UI/UX", "Tailwind"]'::jsonb
FROM departments d WHERE d.name = 'Design'
UNION ALL
SELECT 
    uuid_generate_v4(),
    'Conference Room A',
    'room',
    10.0,
    d.id,
    '["projector", "video-conf"]'::jsonb
FROM departments d WHERE d.name = 'Engineering';

-- Insert sample project
INSERT INTO projects (id, name, description, start_date, end_date, status, project_manager_id)
SELECT 
    uuid_generate_v4(),
    'Xynergy Platform Launch',
    'Initial launch of the Xynergy resource management platform',
    '2026-02-01'::date,
    '2026-06-30'::date,
    'planning',
    u.id
FROM users u WHERE u.email = 'admin@xynergy.com';
