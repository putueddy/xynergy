-- Revert authentication security columns
-- Created: 2026-02-22

-- Drop indexes
DROP INDEX IF EXISTS idx_users_locked_until;
DROP INDEX IF EXISTS idx_users_refresh_token;

-- Drop columns
ALTER TABLE users 
    DROP COLUMN IF EXISTS login_attempts,
    DROP COLUMN IF EXISTS locked_until,
    DROP COLUMN IF EXISTS refresh_token_hash,
    DROP COLUMN IF EXISTS last_login_at;
