# Deep-Dive: Resource & Department Management API

**Generated:** 2026-02-01  
**Scope:** Backend resource and department management routes  
**Files Analyzed:** 2 (resource.rs, department.rs) - 636 LOC  

---

## 1. Overview

This analysis covers two core REST API route modules for the Xynergy project:

- **`resource.rs`** (291 lines): Manages resources (people, equipment, assets) that can be assigned to projects. Features capacity tracking with BigDecimal/f64 conversion and JSON skill management.

- **`department.rs`** (345 lines): Manages organizational departments with head assignment validation and referential integrity checks.

**Architecture Pattern:** Axum-based REST API with PostgreSQL (sqlx), full CRUD operations, audit logging, and COALESCE-based partial updates.

---

## 2. File-by-File Breakdown

### 2.1 resource.rs - Resource Management Module

#### Imports & Dependencies
```rust
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::{audit_payload, log_audit, user_id_from_headers};
```

#### Exported Structures

| Export | Type | Description |
|--------|------|-------------|
| `ResourceResponse` | Struct | API response with f64 capacity |
| `CreateResourceRequest` | Struct | POST /resources body |
| `UpdateResourceRequest` | Struct | PUT /resources/:id body (all optional) |
| `resource_routes` | Function | Route builder |

#### Core Functions

**`bigdecimal_to_f64(bd: Option<BigDecimal>) -> Option<f64>`**
- Converts database BigDecimal to API f64
- Uses string parsing: `bd.to_string().parse::<f64>().ok()`
- **Risk:** Precision loss in conversion chain

**`f64_to_bigdecimal(f: Option<f64>) -> Option<BigDecimal>`**
- Converts API f64 to database BigDecimal
- Uses `BigDecimal::try_from(v).ok()`

**`get_resources(State) -> Result<Json<Vec<ResourceResponse>>>`**
- Lists all resources ordered by name
- Converts BigDecimal capacity to f64 for response

**`get_resource(State, Path) -> Result<Json<ResourceResponse>>`**
- Single resource lookup by UUID
- Returns `AppError::NotFound` if missing

**`create_resource(State, HeaderMap, Json) -> Result<Json<ResourceResponse>>`**
- Creates new resource with audit logging
- Converts f64 capacity to BigDecimal for storage
- Captures actor ID from headers for audit trail

**`update_resource(State, HeaderMap, Path, Json) -> Result<Json<ResourceResponse>>`**
- Partial updates using COALESCE for all fields
- Fetches existing resource to build audit before/after
- Updates: name, resource_type, capacity, department_id, skills

**`delete_resource(State, HeaderMap, Path) -> Result<Json<Value>>`**
- Hard deletes resource
- Logs audit with before state only
- Returns success message JSON

---

### 2.2 department.rs - Department Management Module

#### Imports & Dependencies
```rust
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::services::{audit_payload, log_audit, user_id_from_headers};
```

#### Exported Structures

| Export | Type | Description |
|--------|------|-------------|
| `Department` | Struct | Response with head info (id, name, head_id, head_name) |
| `CreateDepartmentRequest` | Struct | POST /departments body |
| `UpdateDepartmentRequest` | Struct | PUT /departments/:id body |
| `department_routes` | Function | Route builder |

#### Core Functions

**`get_departments(State) -> Result<Json<Value>>`**
- Lists all departments with LEFT JOIN to users for head_name
- Returns JSON array (not typed struct)

**`get_department(State, Path) -> Result<Json<Value>>`**
- Single department lookup with head info
- Returns `AppError::NotFound` if missing

**`create_department(State, HeaderMap, Json) -> Result<Json<Value>>`**
- Validates head_id exists in users table
- Creates department with audit logging
- Fetches head_name for response

**`update_department(State, HeaderMap, Path, Json) -> Result<Json<Value>>`**
- Validates head_id if provided
- Partial updates using COALESCE
- Fetches head_name for response

**`delete_department(State, HeaderMap, Path) -> Result<Json<Value>>`**
- **Protection:** Checks if department has assigned users
- Returns 400 if users exist
- Hard delete with audit logging

**`get_department_head_candidates(State) -> Result<Json<Value>>`**
- Returns users with role = 'admin' or 'project_manager'
- Used for head selection dropdown
- Formats name as "First Last"

---

## 3. API Endpoints Reference

### Resource Routes (`/api/v1/resources`)

| Method | Path | Handler | Auth | Description |
|--------|------|---------|------|-------------|
| GET | `/resources` | get_resources | Yes | List all resources |
| POST | `/resources` | create_resource | Yes | Create new resource |
| GET | `/resources/:id` | get_resource | Yes | Get resource by UUID |
| PUT | `/resources/:id` | update_resource | Yes | Update resource (partial) |
| DELETE | `/resources/:id` | delete_resource | Yes | Delete resource |

### Department Routes (`/api/v1/departments`)

| Method | Path | Handler | Auth | Description |
|--------|------|---------|------|-------------|
| GET | `/departments` | get_departments | Yes | List all departments |
| POST | `/departments` | create_department | Yes | Create department |
| GET | `/departments/:id` | get_department | Yes | Get department by UUID |
| PUT | `/departments/:id` | update_department | Yes | Update department |
| DELETE | `/departments/:id` | delete_department | Yes | Delete (checks users) |
| GET | `/departments/head-candidates` | get_department_head_candidates | Yes | Get eligible heads |

---

## 4. Data Structures

### Resource Module

```rust
// Response
pub struct ResourceResponse {
    pub id: Uuid,
    pub name: String,
    pub resource_type: String,
    pub capacity: Option<f64>,  // Converted from BigDecimal
    pub department_id: Option<Uuid>,
    pub skills: Option<serde_json::Value>,
}

// Create Request
pub struct CreateResourceRequest {
    pub name: String,
    pub resource_type: String,
    pub capacity: Option<f64>,
    pub department_id: Option<Uuid>,
    pub skills: Option<serde_json::Value>,
}

// Update Request (all fields optional)
pub struct UpdateResourceRequest {
    pub name: Option<String>,
    pub resource_type: Option<String>,
    pub capacity: Option<f64>,
    pub department_id: Option<Uuid>,
    pub skills: Option<serde_json::Value>,
}
```

### Department Module

```rust
// Response
pub struct Department {
    pub id: Uuid,
    pub name: String,
    pub head_id: Option<Uuid>,
    pub head_name: Option<String>,  // From JOIN
}

// Create Request
pub struct CreateDepartmentRequest {
    pub name: String,
    pub head_id: Option<Uuid>,
}

// Update Request (all fields optional)
pub struct UpdateDepartmentRequest {
    pub name: Option<String>,
    pub head_id: Option<Uuid>,
}
```

---

## 5. Special Features

### 5.1 BigDecimal Conversion (Resource Only)

**Problem:** PostgreSQL `NUMERIC` → Rust `BigDecimal`, but APIs use `f64`

**Solution:** Bidirectional conversion functions

```rust
// DB → API
fn bigdecimal_to_f64(bd: Option<BigDecimal>) -> Option<f64> {
    bd.and_then(|d| d.to_string().parse::<f64>().ok())
}

// API → DB
fn f64_to_bigdecimal(f: Option<f64>) -> Option<BigDecimal> {
    f.and_then(|v| BigDecimal::try_from(v).ok())
}
```

**Conversion Chain:**
```
API (f64) → String → BigDecimal → PostgreSQL (NUMERIC)
PostgreSQL (NUMERIC) → BigDecimal → String → f64 → API
```

**Risks:**
- Precision loss in BigDecimal → String → f64 conversion
- NaN/Infinity values may fail
- No input validation for range

### 5.2 COALESCE Partial Updates

Both modules use SQL `COALESCE` for partial updates:

```sql
UPDATE resources 
SET name = COALESCE($1, name),
    resource_type = COALESCE($2, resource_type),
    capacity = COALESCE($3, capacity),
    department_id = COALESCE($4, department_id),
    skills = COALESCE($5, skills)
WHERE id = $6
```

**Benefits:**
- Single endpoint for full/partial updates
- `None` values preserve existing data
- Reduces API surface

**Limitations:**
- Cannot set field to `NULL` explicitly
- Must use explicit `Some(null)` in JSON

### 5.3 Audit Logging

Both modules integrate with centralized audit service:

```rust
// Create: Log after state
let audit_changes = audit_payload(None, Some(json!({"name": ..., ...})));
log_audit(&pool, user_id, "create", "resource", id, audit_changes).await?;

// Update: Log before and after
let audit_changes = audit_payload(
    Some(json!({"name": before_name, ...})),
    Some(json!({"name": after_name, ...}))
);
log_audit(&pool, user_id, "update", "resource", id, audit_changes).await?;

// Delete: Log before state only
let audit_changes = audit_payload(Some(json!({...})), None);
log_audit(&pool, user_id, "delete", "resource", id, audit_changes).await?;
```

**User Extraction:**
```rust
let user_id = user_id_from_headers(&headers)?;
```

### 5.4 Department Head Validation

Departments validate head_id exists:

```rust
if let Some(head_id) = req.head_id {
    let user_exists = sqlx::query!("SELECT id FROM users WHERE id = $1", head_id)
        .fetch_optional(&pool)
        .await?;
    
    if user_exists.is_none() {
        return Err(AppError::Validation(format!("User {} not found", head_id)));
    }
}
```

### 5.5 Delete Protection (Department)

Departments block deletion if users assigned:

```rust
let user_count = sqlx::query!(
    "SELECT COUNT(*) as count FROM users WHERE department_id = $1",
    id
).fetch_one(&pool).await?;

if user_count.count.unwrap_or(0) > 0 {
    return Err(AppError::Validation(
        "Cannot delete department with assigned users..."
    ));
}
```

---

## 6. Error Handling

### Error Types

| Variant | HTTP Status | Usage |
|---------|-------------|-------|
| `AppError::Database(String)` | 500 | SQL errors, connection failures |
| `AppError::NotFound(String)` | 404 | Entity not found |
| `AppError::Validation(String)` | 400 | Business logic violations |

### Error Patterns

**Database with Context:**
```rust
.fetch_one(&pool)
.await
.map_err(|e| AppError::Database(format!("Failed to create: {}", e)))?
```

**Not Found Handling:**
```rust
.fetch_optional(&pool)
.await
.map_err(|e| AppError::Database(e.to_string()))?
.ok_or_else(|| AppError::NotFound(format!("Resource {} not found", id)))?
```

**Validation:**
```rust
if user_exists.is_none() {
    return Err(AppError::Validation(format!("User {} not found", head_id)));
}
```

---

## 7. Entity Relationships

```
┌─────────────┐         ┌─────────────┐
│ departments │◄────────│  resources  │
│   (head_id) │         │(department_ │
└─────────────┘         │    id)      │
       ▲                └─────────────┘
       │
       │                ┌─────────────┐
       └────────────────│    users    │
              (department_id)
```

- **Department → Resources:** One-to-many (nullable department_id)
- **Department → Users:** One-to-many (department_id in users, head_id in departments)
- **Users → Resources:** No direct relationship

---

## 8. Implementation Guidance

### When Modifying resource.rs

**Risks:**
- BigDecimal precision loss
- Audit payload structure changes
- Foreign key violations (department_id)

**Verification Steps:**
1. Test BigDecimal conversion with edge cases (0.1, very large numbers)
2. Verify audit logs capture all fields correctly
3. Test with invalid department_id (should fail gracefully)
4. Check capacity field handles None correctly

**Testing:**
```bash
# Create resource
curl -X POST http://localhost:3000/api/v1/resources \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Senior Developer",
    "resource_type": "human",
    "capacity": 40.5,
    "department_id": "uuid-here",
    "skills": ["rust", "react"]
  }'

# Partial update (only name)
curl -X PUT http://localhost:3000/api/v1/resources/<id> \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"name": "Lead Developer"}'
```

### When Modifying department.rs

**Risks:**
- Head validation logic errors
- Delete protection bypass
- Circular reference (department head is user, user has department)

**Verification Steps:**
1. Test head_id validation with non-existent user
2. Test delete with assigned users (should fail)
3. Test delete with no users (should succeed)
4. Verify head-candidates returns only admin/PM roles
5. Check audit captures head changes

**Testing:**
```bash
# Create department with head
curl -X POST http://localhost:3000/api/v1/departments \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Engineering",
    "head_id": "user-uuid-here"
  }'

# Try delete with users (should fail)
curl -X DELETE http://localhost:3000/api/v1/departments/<id> \
  -H "Authorization: Bearer <token>"

# Get head candidates
curl http://localhost:3000/api/v1/departments/head-candidates \
  -H "Authorization: Bearer <token>"
```

---

## 9. Architecture Observations

### Positive Patterns
- ✅ Consistent COALESCE partial update pattern
- ✅ Full audit logging on all mutations
- ✅ Type safety with UUID throughout
- ✅ Validation layer for department heads
- ✅ Delete protection with meaningful error

### Potential Improvements
- ⚠️ Resource uses typed Response; Department uses JSON (inconsistent)
- ⚠️ No FK validation for resource.department_id
- ⚠️ Resource delete doesn't check for assignments
- ⚠️ BigDecimal conversion uses string parsing (precision risk)
- ⚠️ No transaction wrapping for multi-step operations

### Security Considerations
- ✅ User ID from headers (assumes auth middleware)
- ✅ No SQL injection (parameterized queries)
- ⚠️ No explicit authorization checks visible
- ⚠️ No rate limiting visible

---

## 10. Module Comparison

| Aspect | resource.rs | department.rs |
|--------|-------------|---------------|
| Lines of Code | 291 | 345 |
| CRUD Operations | Full | Full |
| Extra Endpoints | None | `/head-candidates` |
| Typed Response | Yes (ResourceResponse) | No (serde_json::Value) |
| Complex Types | BigDecimal, JSON | None |
| Delete Protection | None | User count check |
| Input Validation | None | head_id exists |
| External References | None | users table |

---

## Summary Statistics

| Metric | Count |
|--------|-------|
| Total LOC | 636 |
| API Endpoints | 11 |
| Database Queries | 12 |
| Public Exports | 7 |
| Error Types Used | 3 |

**Key Insight:** Both modules demonstrate consistent Axum/PostgreSQL patterns with audit logging, but diverge in type safety (Resource uses typed structs, Department uses JSON) and validation depth (Department has explicit validation, Resource relies on FK constraints).
