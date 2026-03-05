-- Remove P&L settings columns
ALTER TABLE projects
    DROP CONSTRAINT IF EXISTS chk_margin_alert_threshold_range,
    DROP CONSTRAINT IF EXISTS chk_target_margin_range;

ALTER TABLE projects
    DROP COLUMN IF EXISTS margin_alert_threshold_pct,
    DROP COLUMN IF EXISTS target_margin_pct;
