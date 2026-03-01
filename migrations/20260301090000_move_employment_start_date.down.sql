ALTER TABLE ctc_records ADD COLUMN IF NOT EXISTS thr_employment_start_date DATE;
UPDATE ctc_records c SET thr_employment_start_date = r.employment_start_date FROM resources r WHERE c.resource_id = r.id AND r.employment_start_date IS NOT NULL;
ALTER TABLE resources DROP COLUMN IF EXISTS employment_start_date;
