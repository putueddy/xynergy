ALTER TABLE resources ADD COLUMN IF NOT EXISTS employment_start_date DATE;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_name = 'ctc_records' AND column_name = 'thr_employment_start_date'
    ) THEN
        EXECUTE 'UPDATE resources r
                 SET employment_start_date = c.thr_employment_start_date
                 FROM ctc_records c
                 WHERE r.id = c.resource_id
                 AND c.thr_employment_start_date IS NOT NULL';

        EXECUTE 'ALTER TABLE ctc_records DROP COLUMN IF EXISTS thr_employment_start_date';
    END IF;
END $$;
