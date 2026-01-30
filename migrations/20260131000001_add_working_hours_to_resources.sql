-- Migration: Add working_hours column to resources table
-- Created: 2026-01-31

-- Add working_hours column with default 8 hours
ALTER TABLE resources 
ADD COLUMN working_hours DECIMAL(4, 2) NOT NULL DEFAULT 8.0;

-- Add work_start_time and work_end_time for future admin configuration
ALTER TABLE resources 
ADD COLUMN work_start_time TIME NOT NULL DEFAULT '08:00:00';

ALTER TABLE resources 
ADD COLUMN work_end_time TIME NOT NULL DEFAULT '17:00:00';

-- Update existing resources to have 8 hours working time
UPDATE resources SET working_hours = 8.0 WHERE working_hours IS NULL;

-- Create index for better query performance
CREATE INDEX idx_allocations_resource_dates ON allocations(resource_id, start_date, end_date);
