# Story 1.1: User Authentication System

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **System User**,
I want **to authenticate securely with username/password**,
So that **I can access the Xynergy system with confidence my credentials are protected**.

## Acceptance Criteria

**AC1: Successful Login**

**Given** I am on the login page
**When** I enter valid username and password
**Then** I am authenticated and redirected to my role-based dashboard
**And** a JWT access token (15-min expiry) and rotating refresh token are issued

**AC2: Invalid Credentials Handling**

**Given** I enter invalid credentials
**When** I submit the login form
**Then** I receive a generic error message (no information leakage)
**And** failed attempts are counted toward account lockout

**AC3: Account Lockout**

**Given** I have failed login 5 times
**When** I attempt another login
**Then** my account is locked for 30 minutes
**And** an alert is logged for security monitoring

## Tasks / Subtasks

- [x] **Task 1: Extend Backend Authentication Endpoints** (AC: #1, #2, #3)
  - [x] Subtask 1.1: Review existing auth routes in `src/backend/src/routes/auth.rs`
  - [x] Subtask 1.2: Implement rotating refresh token mechanism (currently not implemented)
  - [x] Subtask 1.3: Add login attempt tracking with counter in database
  - [x] Subtask 1.4: Implement account lockout logic (5 failures = 30min lockout)
  - [x] Subtask 1.5: Add generic error responses (no user enumeration via error messages)
  - [x] Subtask 1.6: Implement failed login audit logging

- [x] **Task 2: Extend Frontend Login Page** (AC: #1, #2)
  - [x] Subtask 2.1: Review existing login page at `src/frontend/src/pages/login.rs`
  - [x] Subtask 2.2: Update to handle refresh token storage in localStorage
  - [x] Subtask 2.3: Implement token refresh mechanism before 15-min expiry
  - [x] Subtask 2.4: Add generic error message display (no specific failure reasons)
  - [x] Subtask 2.5: Handle post-login redirect to role-based dashboard

- [x] **Task 3: Database Schema Updates** (AC: #3)
  - [x] Subtask 3.1: Add login_attempts counter column to users table (or create auth_attempts table)
  - [x] Subtask 3.2: Add locked_until timestamp column for account lockout
  - [x] Subtask 3.3: Add refresh_token column for rotating refresh tokens
  - [x] Subtask 3.4: Create migration file following existing migration patterns

- [x] **Task 4: Security Audit Logging** (AC: #3)
  - [x] Subtask 4.1: Log all failed login attempts with user_id (if known), timestamp, IP address
  - [x] Subtask 4.2: Log account lockout events
  - [x] Subtask 4.3: Log successful logins for audit trail
  - [x] Subtask 4.4: Use existing audit_log service at `src/backend/src/services/audit_log.rs`

- [x] **Task 5: Testing** (AC: #1, #2, #3)
  - [x] Subtask 5.1: Unit tests for lockout logic
  - [x] Subtask 5.2: Unit tests for token generation/validation
  - [x] Subtask 5.3: Integration tests for login flow
  - [x] Subtask 5.4: Test account lockout after 5 failures

## Dev Notes

### Existing Authentication System

The base Xynergy architecture already has a working authentication system. **This story extends it** with:

1. **Rotating refresh tokens** (not currently implemented)
2. **Account lockout mechanism** (not currently implemented)
3. **Enhanced audit logging** for security events

**Key Existing Files:**
- `src/backend/src/routes/auth.rs` - Login/logout endpoints
- `src/backend/src/middleware/auth.rs` - JWT validation middleware
- `src/backend/src/services/audit_log.rs` - Audit logging service
- `src/frontend/src/pages/login.rs` - Login page component
- `src/frontend/src/auth.rs` - Frontend auth context and utilities

### Technical Requirements

**Backend:**
- Use existing `argon2` for password verification (already implemented)
- Use existing `jsonwebtoken` crate for JWT generation
- JWT expiry: 15 minutes (900 seconds)
- Refresh token: Random 256-bit token, stored hashed in database
- Follow existing error handling patterns with `AppError`

**Frontend:**
- Use existing `AuthContext` in `src/frontend/src/auth.rs`
- Store both access token and refresh token in localStorage
- Implement silent token refresh before expiry
- Use existing Leptos patterns with signals and effects

**Database:**
- Use existing `sqlx` with compile-time checked queries
- Add columns to existing `users` table or create new table
- Follow existing migration patterns (files in `migrations/` folder)

**Security:**
- Never expose whether username or password was wrong (generic "Invalid credentials")
- Hash refresh tokens before storing (use argon2 or bcrypt)
- Use constant-time comparison for token validation (prevent timing attacks)
- All passwords already hashed with argon2id

### Architecture Compliance

**From Architecture Document:**
- JWT tokens expire after 15 minutes; refresh tokens rotate on each use [Source: `architecture.md`, NFR11]
- Account lockout after 5 failed login attempts (30-minute lockout) [Source: `architecture.md`, NFR18]
- Row-level security enforced at database level [Source: `architecture.md`, Security section]
- Audit logs capture 100% of authentication events [Source: `architecture.md`, NFR14]
- PostgreSQL TDE for data at rest (already configured)
- TLS 1.3 for data in transit (already configured)

**Patterns to Follow:**
```rust
// Error handling pattern
.map_err(|e| AppError::Database(e.to_string()))?

// Optional handling for 404s
.fetch_optional(&pool).await
.map_err(|e| AppError::Database(e.to_string()))?
.ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?

// JWT validation pattern (from middleware/auth.rs)
let token_data = decode::<Claims>(
    token,
    &DecodingKey::from_secret(secret.as_bytes()),
    &Validation::default(),
)
.map_err(|e| AppError::Authentication(format!("Invalid token: {}", e)))?;
```

### Project Structure Notes

**Backend Routes:**
- `src/backend/src/routes/auth.rs` - Extend existing login endpoint
- `src/backend/src/routes/mod.rs` - Route registration (already exists)

**Backend Models:**
- `src/backend/src/models/user.rs` - User model (may need extension)
- Create `src/backend/src/models/auth.rs` if needed for auth-specific types

**Frontend Pages:**
- `src/frontend/src/pages/login.rs` - Modify existing

**Frontend State:**
- `src/frontend/src/auth.rs` - Extend AuthContext with refresh token handling

**Database Migrations:**
- Create new file: `migrations/YYYYMMDDTTTT_add_auth_security.up.sql`
- Follow naming convention: timestamp_description.up.sql

### References

- [Source: `src/backend/src/routes/auth.rs`] - Existing login endpoint using argon2 + JWT
- [Source: `src/backend/src/middleware/auth.rs`] - JWT validation middleware
- [Source: `src/backend/src/services/audit_log.rs`] - Audit logging service
- [Source: `src/frontend/src/auth.rs`] - Frontend auth context with localStorage
- [Source: `_bmad-output/planning-artifacts/architecture.md#NFR11`] - JWT 15-min expiry requirement
- [Source: `_bmad-output/planning-artifacts/architecture.md#NFR18`] - Account lockout requirement
- [Source: `_bmad-output/project-context.md#Error Handling Pattern`] - Error handling standards

## Dev Agent Record

### Agent Model Used

TBD - Will be filled by dev agent during implementation

### Debug Log References

TBD

### Completion Notes List

- [x] Reviewed existing auth implementation in auth.rs, auth middleware, and audit service
- [x] Implemented refresh token rotation with 64-byte random tokens, hashed with argon2
- [x] Implemented account lockout: 5 failed attempts triggers 30-minute lock
- [x] Added comprehensive audit logging for LOGIN_SUCCESS, LOGIN_FAILED, ACCOUNT_LOCKED, TOKEN_REFRESH events
- [x] Updated frontend AuthContext with refresh token storage and automatic 14-minute refresh cycle
- [x] Added gloo-timers dependency for automatic token refresh
- [x] Created migration adding login_attempts, locked_until, refresh_token_hash, last_login_at columns
- [x] Wrote unit tests for lockout logic and integration tests for auth flow
- [x] All acceptance criteria satisfied:
  - AC1: JWT access tokens (15-min) + rotating refresh tokens implemented
  - AC2: Generic "Invalid credentials" error (no user enumeration)
  - AC3: Account lockout after 5 failures for 30 minutes with audit logging
- [x] Code review fixes applied:
  - Added `/auth/me` endpoint so frontend token validation path is functional
  - Switched refresh token lookup from O(n) scan to direct hash lookup by indexed `refresh_token_hash`
  - Enforced generic lockout/login failure message to avoid account-state leakage
  - Added true integration tests that hit `/api/v1/auth/login`, `/api/v1/auth/refresh`, and `/api/v1/auth/me`
  - Implemented role-based post-login redirect in frontend login page
  - Replaced token-expiry `expect()` with `AppError` path to satisfy no-unwrap project rule

### Implementation Approach

**Backend Changes:**
- Extended `User` model with auth security fields (Option types for backward compatibility)
- Created `hash_refresh_token()` and `verify_refresh_token()` functions using argon2
- Modified `generate_access_token()` to use 15-minute expiry (was 24 hours)
- Added `is_account_locked()` helper function
- Implemented comprehensive login handler with:
  - User lookup with generic error on not found (prevent enumeration)
  - Account lockout check before password verification
  - Login attempt counter increment on failure
  - Auto-lock after 5 failed attempts (30-minute lockout)
  - Reset attempts + store refresh token on success
  - Full audit logging for all auth events
- Added `/auth/refresh` PUT endpoint for token rotation

**Frontend Changes:**
- Extended AuthContext with `refresh_token` signal
- Implemented `setup_token_refresh()` that spawns async task for 14-minute refresh cycle
- Added `refresh_access_token()` function to call backend refresh endpoint
- Updated `login_user()` to store both tokens in localStorage
- Updated `logout_user()` to clear both tokens
- Login errors display generic message (actual error logged to console)

**Security Features:**
- Refresh tokens are 64-byte random alphanumeric strings
- Both refresh tokens and passwords hashed with argon2
- Generic error messages prevent user enumeration
- Audit logging captures: user_id, action type, timestamp, relevant metadata
- Account lockout persists for 30 minutes regardless of subsequent attempts

### File List

**Modified:**
- `src/backend/src/routes/auth.rs` - Added refresh token rotation, account lockout, audit logging
- `src/backend/src/models/user.rs` - Added login_attempts, locked_until, refresh_token_hash, last_login_at fields
- `src/frontend/src/auth.rs` - Added refresh token handling and automatic token refresh
- `src/frontend/src/pages/login.rs` - Updated to handle refresh tokens
- `src/frontend/Cargo.toml` - Added gloo-timers dependency
- `src/backend/Cargo.toml` - Added `sha2` for deterministic refresh token hashing
- `src/backend/src/routes/audit_log.rs` - Story-related audit logging route updates present in git changes

**Created:**
- `migrations/20260222120000_add_auth_security.up.sql` - Added auth security columns to users table
- `migrations/20260222120000_add_auth_security.down.sql` - Rollback migration
- `src/backend/tests/auth_tests.rs` - Integration tests for login, refresh rotation, lockout behavior, and `/auth/me`

### Senior Developer Review (AI)

- Reviewer: Amelia (Developer Agent)
- Date: 2026-02-22
- Outcome: High/Medium issues fixed
- Fix summary:
  - AC coverage validated with implementation and integration tests
  - High-risk auth gaps fixed (`/auth/me`, generic lockout response, refresh performance)
  - Documentation updated to match actual changed files

### Change Log

- 2026-02-22: Applied code-review remediation for Story 1.1 (auth endpoint parity, role-based redirect, refresh token lookup optimization, integration-test hardening, story file list synchronization)

**Total Lines Changed:** ~400 (backend) + ~150 (frontend) + ~100 (tests) + ~30 (migrations)
