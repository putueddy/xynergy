-- Add authentication security columns to users table
-- Created: 2026-02-22

-- Add columns for account lockout and refresh token
ALTER TABLE users 
    ADD COLUMN IF NOT EXISTS login_attempts INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS locked_until TIMESTAMP WITH TIME ZONE,
    ADD COLUMN IF NOT EXISTS refresh_token_hash VARCHAR(255),
    ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMP WITH TIME ZONE;

-- Create index for locked accounts lookup
CREATE INDEX IF NOT EXISTS idx_users_locked_until ON users(locked_until) 
    WHERE locked_until IS NOT NULL;

-- Create index for refresh token lookup (for token rotation)
CREATE INDEX IF NOT EXISTS idx_users_refresh_token ON users(refresh_token_hash) 
    WHERE refresh_token_hash IS NOT NULL;
