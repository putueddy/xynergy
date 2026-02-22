# Deep-Dive: Settings & Audit System

## Overview

This document provides a comprehensive analysis of the Settings (Holiday Management) and Audit Logging modules in Xynergy. These modules provide:

1. **Holiday Management** - Configurable non-working days that affect resource capacity calculations
2. **Audit Logging Infrastructure** - Comprehensive change tracking across all system entities

**Key Characteristics:**
- **Language:** Rust
- **Framework:** Axum (backend)
- **Data Access:** sqlx with compile-time checked SQL
- **Scope:** Supporting infrastructure (not core business logic, but critical for compliance)
- **Integration:** Deeply integrated with all mutation operations across the system

**Why These Modules Matter:**
- **Holiday Management:** Defines business calendar - directly impacts capacity calculations in the allocation system
- **Audit Logging:** Provides accountability and compliance trail for all data changes

---

## File Breakdown

### 1. `/src/backend/src/routes/holiday.rs` (272 lines)

**Purpose:** Holiday CRUD API endpoints for managing organizational holidays

**Structure:**
```
holiday.rs
├── Data Structures (lines 13-36)
│   ├── Holiday - Response with date as String (ISO format)
│   ├── CreateHolidayRequest - Date as String for parsing
│   └── UpdateHolidayRequest - All fields optional
├── API Handlers (lines 38-259)
│   ├── get_holidays() - List all holidays, ordered by date (lines 39-57)
│   ├── get_holiday() - Get single holiday by ID (lines 60-82)
│   ├── create_holiday() - Create with date parsing & audit (lines 85-129)
│   ├── update_holiday() - Update with partial fields (lines 132-219)
│   └── delete_holiday() - Delete with audit trail (lines 223-259)
└── Route Registration (lines 265-272)
```

**Key Implementation Details:**
- **Date Type Handling:** Uses `String` in API/structs, converts to `chrono::NaiveDate` for database
- **Date Format:** Enforces ISO 8601 format `%Y-%m-%d` (e.g., "2024-12-25")
- **Type Casting in SQL:** Uses `date::TEXT` cast to return dates as strings from PostgreSQL
- **Audit Integration:** All mutations logged via `log_audit()` service

### 2. `/src/backend/src/services/audit_log.rs` (67 lines)

**Purpose:** Core audit logging service used across all modules

**Structure:**
```
audit_log.rs
├── Public Functions (lines 10-66)
│   ├── log_audit() - Persist audit entry to database (lines 10-32)
│   ├── audit_payload() - Build before/after change structure (lines 34-39)
│   └── user_id_from_headers() - Extract user from JWT token (lines 41-66)
└── Dependencies
    ├── jsonwebtoken - JWT decoding
    └── Authorization header parsing
```

**Key Implementation Details:**
- **JWT Decoding:** Extracts `sub` claim (user ID) from Bearer token
- **Error Handling:** Distinguishes auth errors from internal errors
- **Payload Structure:** Standardized `{ "before": {...}, "after": {...} }` format

### 3. `/src/backend/src/routes/audit_log.rs` (91 lines)

**Purpose:** Query endpoint for retrieving audit trail data

**Structure:**
```
audit_log.rs
├── Query/Response Types (lines 13-28)
│   ├── AuditLogQuery - Optional limit parameter (default: 50, max: 200)
│   └── AuditLogResponse - Includes denormalized user_name
├── API Handlers (lines 30-87)
│   └── get_audit_logs() - Paginated query with user info JOIN (lines 30-87)
└── Route Registration (lines 89-91)
```

**Key Implementation Details:**
- **Pagination:** Configurable limit with bounds checking (1-200)
- **User Denormalization:** LEFT JOIN with users table to get human-readable names
- **Name Construction:** Combines first_name + last_name, handles nulls gracefully

### 4. `/src/backend/src/services/mod.rs` (7 lines)

**Purpose:** Service module exports

**Content:**
- Re-exports `audit_payload`, `log_audit`, `user_id_from_headers` from `audit_log` module
- Declares other service modules (mostly stubs: `allocation_service.rs`, `project_service.rs`, etc. are 1 byte each)

---

## API Endpoints

### Holiday Endpoints

| Method | Endpoint | Handler | Description |
|--------|----------|---------|-------------|
| GET | `/holidays` | `get_holidays` | List all holidays, sorted by date ASC |
| GET | `/holidays/:id` | `get_holiday` | Get single holiday by UUID |
| POST | `/holidays` | `create_holiday` | Create holiday with date validation |
| PUT | `/holidays/:id` | `update_holiday` | Update holiday (partial updates supported) |
| DELETE | `/holidays/:id` | `delete_holiday` | Delete holiday with audit trail |

### Audit Log Endpoints

| Method | Endpoint | Handler | Description |
|--------|----------|---------|-------------|
| GET | `/audit-logs` | `get_audit_logs` | Query audit trail with optional limit |

**Query Parameters:**
- `limit` (optional): Number of records to return (1-200, default: 50)

---

## Data Structures

### Holiday Domain

```rust
// API Response / Database mapping
pub struct Holiday {
    pub id: Uuid,
    pub name: String,
    pub date: String,  // ISO 8601 format: "2024-12-25"
    pub description: Option<String>,
}

// Creation request
pub struct CreateHolidayRequest {
    pub name: String,
    pub date: String,  // Must be valid ISO date
    pub description: Option<String>,
}
```

**Date Type Design Decision:**
- API uses `String` to accept/return "YYYY-MM-DD" format
- Internally parsed to `chrono::NaiveDate` for database storage
- PostgreSQL stores as `DATE` type, cast to TEXT for API

**SQL Type Cast Pattern:**
```rust
sqlx::query_as!(
    Holiday,
    r#"
    SELECT 
        id,
        name,
        date::TEXT as "date!",  // Cast DATE → TEXT
        description
    FROM holidays
    "#
)
```

### Audit Log Domain

```rust
// Query parameters
pub struct AuditLogQuery {
    pub limit: Option<i64>,  // None = default 50
}

// Response with user info
pub struct AuditLogResponse {
    pub id: Uuid,
    pub user_id: Option<Uuid>,      // Who made the change
    pub user_name: Option<String>,   // Human-readable name (JOINed)
    pub action: String,               // "create", "update", "delete"
    pub entity_type: String,          // "project", "holiday", etc.
    pub entity_id: Option<Uuid>,     // Target entity
    pub changes: serde_json::Value,   // { "before": {...}, "after": {...} }
    pub created_at: DateTime<Utc>,
}
```

### Audit Payload Structure

```rust
// Standardized change tracking format
{
  "before": {
    "name": "Old Project Name",
    "status": "active"
  },
  "after": {
    "name": "New Project Name",
    "status": "completed"
  }
}

// For create operations
{
  "before": null,
  "after": { /* new entity state */ }
}

// For delete operations
{
  "before": { /* entity state before deletion */ },
  "after": null
}
```

---

## Special Features & Business Logic

### 1. Date Parsing and Validation

**Location:** `holiday.rs:90-91`, `holiday.rs:156-163`

**Implementation:**
```rust
let date = chrono::NaiveDate::parse_from_str(&req.date, "%Y-%m-%d")
    .map_err(|e| AppError::Validation(format!("Invalid date format: {}", e)))?;
```

**Business Rules:**
- **Format:** Strict ISO 8601 date format (YYYY-MM-DD)
- **No Timezone:** Uses `NaiveDate` (date-only, no time component)
- **Validation Error:** Returns user-friendly "Invalid date format" message

**Pattern for Optional Date Updates:**
```rust
let date = req.date
    .clone()
    .map(|d| {
        chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d")
            .map_err(|e| AppError::Validation(format!("Invalid date format: {}", e)))
    })
    .transpose()?;  // Option<Result<T, E>> → Result<Option<T>, E>
```

### 2. Audit Logging Service

**Location:** `services/audit_log.rs:10-39`

**Core Function - log_audit():**
```rust
pub async fn log_audit(
    pool: &PgPool,
    user_id: Option<Uuid>,
    action: &str,
    entity_type: &str,
    entity_id: Uuid,
    changes: Value,
) -> Result<()>
```

**Usage Pattern (from holiday.rs):**
```rust
// 1. Extract user
let user_id = user_id_from_headers(&headers)?;

// 2. Build change payload
let audit_changes = audit_payload(
    Some(json!({ "name": old_name, ... })),  // before
    Some(json!({ "name": new_name, ... })),  // after
);

// 3. Persist audit entry
log_audit(
    &pool,
    user_id,
    "update",
    "holiday",
    holiday.id,
    audit_changes,
).await?;
```

**Audit Trail Coverage:**
- **User Tracking:** Optional user_id (for unauthenticated actions or system events)
- **Entity Tracking:** Type + ID for precise change location
- **Temporal Ordering:** created_at timestamp (UTC)
- **Change Diff:** Full before/after state capture

### 3. JWT Authentication Integration

**Location:** `services/audit_log.rs:41-66`

**Implementation:**
```rust
pub fn user_id_from_headers(headers: &HeaderMap) -> Result<Option<Uuid>> {
    // 1. Extract Authorization header
    let auth_header = headers.get(AUTHORIZATION)
        .ok_or(None)?;  // No auth header = anonymous
    
    // 2. Parse Bearer token
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AuthError)?;
    
    // 3. Decode JWT
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    
    // 4. Extract user ID from 'sub' claim
    Uuid::parse_str(&token_data.claims.sub)
}
```

**Security Considerations:**
- **JWT_SECRET:** Environment variable required
- **Token Expiration:** Validated by jsonwebtoken library
- **Graceful Degradation:** Returns `Option<Uuid>` - allows unauthenticated contexts

### 4. Audit Log Query with User Denormalization

**Location:** `routes/audit_log.rs:30-87`

**Query Design:**
```sql
SELECT
    al.id, al.user_id, al.action, al.entity_type, 
    al.entity_id, al.changes, al.created_at,
    u.first_name, u.last_name
FROM audit_logs al
LEFT JOIN users u ON al.user_id = u.id
ORDER BY al.created_at DESC
LIMIT $1
```

**Key Features:**
- **LEFT JOIN:** Includes audit entries even if user was deleted
- **Limit Bounds:** Enforced in code: `limit.max(1).min(200)`
- **Name Construction:** 
  ```rust
  let full_name = format!("{} {}", first_name, last_name)
      .trim()
      .to_string();
  if full_name.is_empty() { None } else { Some(full_name) }
  ```
- **Null Safety:** Handles missing user data gracefully

### 5. Holiday Impact on Allocation System

**Cross-Module Dependency:**

Holidays defined in `holiday.rs` are consumed by `allocation.rs` via the `get_holidays_in_range()` function:

```rust
// In allocation.rs
async fn get_holidays_in_range(
    pool: &PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<NaiveDate>> {
    sqlx::query_scalar!(
        "SELECT date FROM holidays WHERE date >= $1 AND date <= $2",
        start_date,
        end_date
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}
```

**Business Impact:**
- Holidays reduce available working days
- Affects capacity calculations in allocation validation
- No limit on number of holidays (could impact performance with many holidays)

---

## Error Handling

### Holiday Module Error Patterns

**Validation Errors:**
```rust
// Invalid date format
AppError::Validation(format!("Invalid date format: {}", e))

// Holiday not found
AppError::NotFound(format!("Holiday {} not found", id))
```

**Status Code Mapping:**
- `create_holiday()` → 201 Created
- `delete_holiday()` → 204 No Content
- Validation errors → 400 Bad Request
- Not found → 404 Not Found

### Audit Service Error Patterns

**Authentication Errors:**
```rust
// Missing/invalid header
AppError::Authentication("Invalid authorization header format".to_string())

// JWT decoding failure
AppError::Authentication(format!("Invalid token: {}", e))

// Missing secret
AppError::Internal("JWT_SECRET not set".to_string())
```

**Database Errors:**
```rust
AppError::Database(e.to_string())  // Generic SQL error
```

---

## Relationships

### Audit System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Audit Trail Flow                         │
└─────────────────────────────────────────────────────────────┘

┌─────────────┐     ┌──────────────┐     ┌──────────────┐
│   Client    │────▶│  API Handler │────▶│  Database    │
│  Request    │     │              │     │  (entity)    │
└─────────────┘     └──────────────┘     └──────────────┘
                            │
                            │ Extract user from JWT
                            ▼
                     ┌──────────────┐
                     │  user_id_    │
                     │ from_headers │
                     └──────────────┘
                            │
                            │ Build audit payload
                            ▼
                     ┌──────────────┐
                     │audit_payload │
                     └──────────────┘
                            │
                            │ Persist audit
                            ▼
                     ┌──────────────┐
                     │   log_audit  │────▶┌──────────────┐
                     └──────────────┘     │ audit_logs   │
                                          │   table      │
                                          └──────────────┘

Audit Query Flow:
┌─────────────┐     ┌──────────────────┐     ┌──────────────┐
│   Client    │────▶│  get_audit_logs  │────▶│  audit_logs  │
│   Query     │     │                  │     │  LEFT JOIN   │
└─────────────┘     └──────────────────┘     │   users      │
                                             └──────────────┘
```

### Holiday Integration

```
┌─────────────┐         ┌─────────────────┐
│   Holiday   │────────▶│   Allocation    │
│   Module    │  date   │   Capacity      │
│ (settings)  │   list  │   Calculation   │
└─────────────┘         └─────────────────┘
       │                         │
       │                         │
       ▼                         ▼
┌─────────────┐           ┌──────────────┐
│  holidays   │           │  Working     │
│   table     │           │  days check  │
└─────────────┘           └──────────────┘
```

### Module Dependencies

**holiday.rs imports:**
- `crate::error::AppError` - Error types
- `crate::services::{audit_payload, log_audit, user_id_from_headers}` - Audit integration

**audit_log.rs (routes) imports:**
- `crate::error::{AppError, Result}` - Error types
- Standard Axum/sqlx/chrono

**audit_log.rs (services) imports:**
- `crate::error::{AppError, Result}` - Error types
- `crate::routes::Claims` - JWT claims structure
- `jsonwebtoken` - JWT decoding

---

## Implementation Guidance

### When Adding New Holiday Fields

**Step 1: Update Data Structures**
```rust
// In Holiday struct (line 14)
pub struct Holiday {
    // ... existing fields
    pub country_code: Option<String>,  // New field
}

// In CreateHolidayRequest
pub struct CreateHolidayRequest {
    // ... existing fields
    pub country_code: Option<String>,
}
```

**Step 2: Update SQL Queries**
All queries need to include the new field:
```rust
sqlx::query_as!(
    Holiday,
    r#"
    INSERT INTO holidays (name, date, description, country_code)
    VALUES ($1, $2, $3, $4)
    RETURNING id, name, date::TEXT as "date!", description, country_code
    "#
)
```

**Step 3: Update Audit Payloads**
Include new field in before/after tracking:
```rust
let audit_changes = audit_payload(
    Some(json!({
        "country_code": existing.country_code,
        // ... other fields
    })),
    Some(json!({
        "country_code": req.country_code.clone().or(existing.country_code),
        // ... other fields
    })),
);
```

### When Extending Audit Logging

**Adding New Entity Types:**

No changes needed to audit service - it's generic. Just use consistent entity_type strings:

```rust
// In your new module
log_audit(
    &pool,
    user_id,
    "create",           // action
    "invoice",          // entity_type ← new type
    invoice.id,
    audit_changes,
).await?;
```

**Adding Structured Change Tracking:**

Current payload is free-form JSON. For more structure:

```rust
// Option 1: Keep JSON but add metadata
let audit_changes = json!({
    "before": old_state,
    "after": new_state,
    "metadata": {
        "reason": req.change_reason,
        "ip_address": extract_ip(&headers),
    }
});
```

**Enhancing Audit Queries:**

Common query additions:
```rust
// Filter by entity type
pub struct AuditLogQuery {
    pub limit: Option<i64>,
    pub entity_type: Option<String>,  // Filter by table
    pub user_id: Option<Uuid>,        // Filter by user
    pub start_date: Option<DateTime<Utc>>,  // Date range
}

// Updated SQL
WHERE ($2::TEXT IS NULL OR al.entity_type = $2)
  AND ($3::UUID IS NULL OR al.user_id = $3)
  AND ($4::TIMESTAMPTZ IS NULL OR al.created_at >= $4)
```

### Audit Log Retention Strategy

**Current State:** No retention policy implemented

**Considerations:**
- Audit logs grow indefinitely
- Query performance degrades over time
- Compliance requirements may mandate retention periods

**Implementation Options:**

1. **Archiving Strategy:**
```sql
-- Monthly archive job
INSERT INTO audit_logs_archive 
SELECT * FROM audit_logs 
WHERE created_at < NOW() - INTERVAL '1 year';

DELETE FROM audit_logs 
WHERE created_at < NOW() - INTERVAL '1 year';
```

2. **Partitioning:**
```sql
-- PostgreSQL native partitioning by month
CREATE TABLE audit_logs (
    ...
    created_at TIMESTAMP NOT NULL
) PARTITION BY RANGE (created_at);
```

---

## Architecture Observations

### Strengths

1. **Consistent Audit Pattern:** All modules use same `log_audit()` function
2. **User Attribution:** JWT integration provides reliable user identification
3. **Change Transparency:** Full before/after state capture
4. **Holiday Integration:** Clean separation - holidays defined in settings, consumed by allocation
5. **Date Handling:** Explicit format validation prevents data quality issues
6. **Graceful Degradation:** LEFT JOIN in audit query handles deleted users

### Potential Improvements

1. **Code Duplication:** Update handlers copy-paste audit change construction
   - **Suggestion:** Macro or helper function for common patterns

2. **Holiday Caching:** Allocation module queries holidays on every capacity check
   - **Suggestion:** Cache holiday list in memory (rarely changes)

3. **Audit Log Size:** Changes stored as full JSON documents
   - **Suggestion:** Consider delta/diff storage for large entities

4. **Soft Deletes:** No soft delete pattern for holidays
   - **Suggestion:** Add `deleted_at` timestamp for recoverability

5. **Holiday Recurrence:** No support for recurring holidays (e.g., "Christmas every year")
   - **Suggestion:** Add recurrence rules or yearly bulk creation

6. **Audit Query Performance:** No indexes mentioned
   - **Suggestion:** Add indexes on `audit_logs(user_id, created_at)` and `audit_logs(entity_type, entity_id)`

### Design Decisions

**Why String for Holiday Date?**
```rust
pub date: String  // "2024-12-25"
```
- **Pros:** Simple JSON serialization, ISO 8601 standard
- **Cons:** Requires parsing on every operation
- **Alternative:** `chrono::NaiveDate` with custom serde

**Why Option<Uuid> for user_id in Audit?**
- Allows system-generated changes (e.g., scheduled jobs)
- Handles legacy data where user unknown
- Supports anonymous actions where applicable

**Why Separate audit_log.rs in routes vs services?**
- **routes/audit_log.rs:** Query endpoint (reads audit_logs table)
- **services/audit_log.rs:** Write service (used by all modules)
- **Separation of concerns:** Write operations are service, read is route

---

## Database Schema

### Holidays Table

```sql
CREATE TABLE holidays (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR NOT NULL,
    date DATE NOT NULL UNIQUE,  -- Unique constraint prevents duplicates
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP  -- Nullable, updated by trigger or code
);

-- Recommended indexes
CREATE INDEX idx_holidays_date ON holidays(date);
CREATE INDEX idx_holidays_date_range ON holidays(date) 
    INCLUDE (name);  -- Covering index for allocation queries
```

### Audit Logs Table

```sql
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action VARCHAR NOT NULL,        -- 'create', 'update', 'delete'
    entity_type VARCHAR NOT NULL,   -- 'project', 'holiday', etc.
    entity_id UUID,                 -- Nullable for global actions
    changes JSONB NOT NULL,         -- Before/after state
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Recommended indexes
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at DESC);
CREATE INDEX idx_audit_logs_user ON audit_logs(user_id, created_at DESC);
CREATE INDEX idx_audit_logs_entity ON audit_logs(entity_type, entity_id);
CREATE INDEX idx_audit_logs_action ON audit_logs(action, created_at DESC);

-- JSONB indexes for specific queries
CREATE INDEX idx_audit_logs_changes_gin ON audit_logs USING GIN (changes);
```

---

## Related Documentation

- **Allocation Deep-Dive:** `deep-dive-project-allocation-management.md` - Shows how holidays affect capacity calculations
- **Auth Deep-Dive:** `deep-dive-auth-user-management.md` - JWT token structure and Claims
- **API Overview:** See `docs/index.md`

---

## Quick Reference

### Key Files for Common Tasks

| Task | File | Function/Line |
|------|------|---------------|
| Add holiday field | `holiday.rs` | Update struct + all SQL queries |
| Change date format | `holiday.rs:90` | `parse_from_str()` pattern |
| Add audit logging | Your module | `log_audit()` call after mutation |
| Query audit trail | `audit_log.rs` | `get_audit_logs()` |
| Extract user ID | `services/audit_log.rs:41` | `user_id_from_headers()` |
| Build audit payload | `services/audit_log.rs:34` | `audit_payload()` |

### Audit Entry Examples

```rust
// Create operation
log_audit(
    &pool, user_id, "create", "project", 
    project.id,
    audit_payload(None, Some(json!(project)))
).await?;

// Update operation
log_audit(
    &pool, user_id, "update", "resource",
    resource.id,
    audit_payload(
        Some(json!(old_state)),
        Some(json!(new_state))
    )
).await?;

// Delete operation
log_audit(
    &pool, user_id, "delete", "holiday",
    id,
    audit_payload(Some(json!(deleted_state)), None)
).await?;
```

---

**Document generated:** 2026-02-01  
**Last updated:** 2026-02-01  
**Version:** 1.0
