-- Migration: Create holidays table and global_settings table
-- Created: 2026-01-31

-- Create holidays table
CREATE TABLE holidays (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    date DATE NOT NULL,
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create index on date for faster queries
CREATE INDEX idx_holidays_date ON holidays(date);

-- Create unique constraint on date (no duplicate holidays)
CREATE UNIQUE INDEX idx_holidays_date_unique ON holidays(date);

-- Create global_settings table
CREATE TABLE global_settings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    key VARCHAR(255) NOT NULL UNIQUE,
    value TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Insert default global settings
INSERT INTO global_settings (key, value, description) VALUES
    ('work_start_time', '08:00', 'Default work start time (HH:MM)'),
    ('work_end_time', '17:00', 'Default work end time (HH:MM)'),
    ('break_duration_minutes', '60', 'Break duration in minutes'),
    ('working_hours_per_day', '8.0', 'Total working hours per day');

-- Insert sample holidays for 2026
INSERT INTO holidays (name, date, description) VALUES
    ('New Year', '2026-01-01', 'Tahun Baru 2026'),
    ('Nyepi', '2026-03-19', 'Hari Raya Nyepi'),
    ('Good Friday', '2026-04-03', 'Wafat Isa Almasih'),
    ('Labour Day', '2026-05-01', 'Hari Buruh Internasional'),
    ('Ascension Day', '2026-05-14', 'Kenaikan Isa Almasih'),
    ('Pancasila Day', '2026-06-01', 'Hari Lahir Pancasila'),
    ('Independence Day', '2026-08-17', 'Hari Kemerdekaan Indonesia'),
    ('Christmas', '2026-12-25', 'Hari Raya Natal');
