ALTER TABLE ctc_records
    ADD COLUMN IF NOT EXISTS encrypted_daily_rate TEXT;

-- daily_rate is considered compensation-sensitive and should be scrubbed
-- after application-layer backfill writes encrypted_daily_rate.
