-- Add P&L settings for target margin alerting
ALTER TABLE projects
    ADD COLUMN IF NOT EXISTS target_margin_pct NUMERIC(5,2) NOT NULL DEFAULT 40.00,
    ADD COLUMN IF NOT EXISTS margin_alert_threshold_pct NUMERIC(5,2) NOT NULL DEFAULT 5.00;

-- Add constraints
ALTER TABLE projects
    ADD CONSTRAINT chk_target_margin_range CHECK (target_margin_pct >= 0 AND target_margin_pct <= 100),
    ADD CONSTRAINT chk_margin_alert_threshold_range CHECK (margin_alert_threshold_pct >= 0 AND margin_alert_threshold_pct <= 100);
