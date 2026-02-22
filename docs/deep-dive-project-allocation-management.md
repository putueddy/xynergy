# Deep-Dive: Project & Allocation Management

## Overview

This document provides a comprehensive analysis of the Project and Allocation management modules in Xynergy. These modules form the **core business logic** of the application, implementing:

1. **Project Lifecycle Management** - CRUD operations for project entities
2. **Resource Allocation System** - Complex date-based allocation with capacity validation
3. **Business Rule Enforcement** - Working hours, holidays, weekend handling, and over-allocation prevention

**Key Characteristics:**
- Language: Rust
- Framework: Axum (backend)
- Data Access: sqlx with compile-time checked SQL
- Core Complexity: Allocation module (799 lines) has sophisticated capacity calculation logic
- Business Impact: Directly manages project timelines and resource assignments

---

## File Breakdown

### 1. `/src/backend/src/routes/project.rs` (259 lines)

**Purpose:** Project CRUD API endpoints

**Structure:**
```
project.rs
├── Data Structures (lines 15-47)
│   ├── ProjectResponse - API response format
│   ├── CreateProjectRequest - POST request body
│   └── UpdateProjectRequest - PUT request body (all fields optional)
├── API Handlers (lines 50-249)
│   ├── get_projects() - List all projects (lines 50-61)
│   ├── get_project() - Get single project by ID (lines 64-80)
│   ├── create_project() - Create with audit logging (lines 83-125)
│   ├── update_project() - Update with audit diff (lines 128-211)
│   └── delete_project() - Delete with audit trail (lines 214-249)
└── Route Registration (lines 252-259)
```

**Key Implementation Details:**
- **Date Handling:** Uses `chrono::NaiveDate` for project start/end dates (no timezone, date-only)
- **Audit Logging:** Every mutation logs before/after state via `audit_payload()` helper
- **User Extraction:** Actions authenticated via `user_id_from_headers()`
- **Update Pattern:** Uses SQL `COALESCE($1, name)` for partial updates (cleaner than explicit branching)

### 2. `/src/backend/src/routes/allocation.rs` (799 lines)

**Purpose:** Complex resource allocation system with capacity validation

**Structure:**
```
allocation.rs
├── Data Structures (lines 17-68)
│   ├── AllocationResponse - Includes denormalized project/resource names
│   ├── CreateAllocationRequest
│   ├── UpdateAllocationRequest
│   ├── DailyAllocation (internal) - Tracks hours per day
│   └── AssignmentInfo (internal) - Links allocation to hours
├── Type Conversions (lines 70-78)
│   ├── bigdecimal_to_f64() - SQL BigDecimal → Rust f64
│   └── f64_to_bigdecimal() - Rust f64 → SQL BigDecimal
├── Date Utilities (lines 80-101)
│   ├── is_weekend() - Check if Saturday/Sunday
│   └── get_holidays_in_range() - Query holiday table
├── Resource Capacity (lines 103-276)
│   ├── get_resource_working_hours() - Resource capacity lookup
│   ├── get_existing_allocations() - Query overlapping allocations
│   └── calculate_daily_allocations() - Core allocation algorithm
├── Capacity Validation (lines 278-409)
│   ├── check_resource_capacity() - Validate no over-allocation
│   └── validate_allocation_dates() - Ensure dates within project bounds
├── API Handlers (lines 411-781)
│   ├── get_allocations() - List all with joins
│   ├── get_allocations_by_project() - Filter by project
│   ├── get_allocations_by_resource() - Filter by resource
│   ├── create_allocation() - Create with validation
│   ├── update_allocation() - Update with validation
│   └── delete_allocation() - Delete with audit
└── Route Registration (lines 784-799)
```

### 3. `/src/backend/src/models/project.rs` (16 lines)

**Purpose:** Database model struct for projects

**Note:** This model uses `sqlx::FromRow` for direct database mapping but the API uses `ProjectResponse` (defined in routes file) for more control over serialization.

### 4. `/src/backend/src/models/allocation.rs` (1 line)

**Purpose:** Placeholder allocation model

**Note:** Currently just a stub struct. The actual allocation data structures are defined in the routes file due to the need for denormalized response fields (project_name, resource_name).

---

## API Endpoints

### Project Endpoints

| Method | Endpoint | Handler | Description |
|--------|----------|---------|-------------|
| GET | `/projects` | `get_projects` | List all projects, sorted by start_date DESC |
| GET | `/projects/:id` | `get_project` | Get single project by UUID |
| POST | `/projects` | `create_project` | Create new project with audit log |
| PUT | `/projects/:id` | `update_project` | Update project (partial updates supported) |
| DELETE | `/projects/:id` | `delete_project` | Delete project with audit trail |

### Allocation Endpoints

| Method | Endpoint | Handler | Description |
|--------|----------|---------|-------------|
| GET | `/allocations` | `get_allocations` | List all allocations with project/resource names |
| GET | `/allocations/project/:project_id` | `get_allocations_by_project` | Filter allocations by project |
| GET | `/allocations/resource/:resource_id` | `get_allocations_by_resource` | Filter allocations by resource |
| POST | `/allocations` | `create_allocation` | Create allocation with capacity validation |
| PUT | `/allocations/:id` | `update_allocation` | Update allocation with re-validation |
| DELETE | `/allocations/:id` | `delete_allocation` | Delete allocation with audit |

---

## Data Structures

### Project Domain

```rust
// Response sent to clients
pub struct ProjectResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub start_date: NaiveDate,      // Project start
    pub end_date: NaiveDate,        // Project end
    pub status: String,             // e.g., "active", "completed"
    pub project_manager_id: Option<Uuid>,
}

// Database model
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: String,
    pub project_manager_id: Uuid,   // NOT optional in DB
}
```

**Note on project_manager_id:** The database model requires a project manager, but the API response makes it optional, suggesting business rules may allow unassigned projects during creation.

### Allocation Domain

```rust
// Response with denormalized names for UI convenience
pub struct AllocationResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub resource_id: Uuid,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub allocation_percentage: f64,     // 0-100%
    pub include_weekend: bool,          // Work weekends?
    pub project_name: String,           // JOINed from projects
    pub resource_name: String,          // JOINed from resources
}

// Creation request
pub struct CreateAllocationRequest {
    pub project_id: Uuid,
    pub resource_id: Uuid,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub allocation_percentage: f64,
    pub include_weekend: bool,
}
```

**Internal Structures for Capacity Calculation:**

```rust
struct DailyAllocation {
    date: NaiveDate,
    allocated_hours: f64,
    assignments: Vec<AssignmentInfo>,
}

struct AssignmentInfo {
    allocation_id: Uuid,
    project_id: Uuid,
    hours: f64,
}
```

---

## Special Features & Business Logic

### 1. Resource Capacity Validation System

**Location:** `allocation.rs:191-409`

**Purpose:** Prevent over-allocation of resources across multiple projects

**Algorithm Overview:**

```rust
// Pseudo-code of the capacity validation flow
calculate_daily_allocations(resource_id, new_allocation) {
    1. Get resource's daily working hours (default: 8.0)
    2. Query holidays within date range
    3. Fetch ALL existing allocations for resource that overlap
    4. Build HashMap<NaiveDate, DailyAllocation>
    5. For each existing allocation:
       - Iterate each day from start to end
       - Skip weekends (if !include_weekend) and holidays
       - Add hours: daily_capacity * (percentage / 100.0)
    6. Add new allocation hours to same map
    7. Return map for capacity check
}
```

**Key Business Rules:**
- **Working Hours:** Configurable per resource (stored in `resources.working_hours`)
- **Holidays:** Automatically excluded from capacity calculation
- **Weekends:** Excluded unless `include_weekend: true`
- **Percentage-Based:** 100% = full working day, can combine (e.g., 50% + 50% = 100%)
- **Date Overlap Detection:** Complex SQL query handles partial overlaps

**Date Overlap Query Pattern:**
```rust
// This query finds any allocation that overlaps with the target date range
WHERE resource_id = $1
AND (
    (start_date <= $2 AND end_date >= $2) OR  -- Overlaps start
    (start_date <= $3 AND end_date >= $3) OR  -- Overlaps end
    (start_date >= $2 AND end_date <= $3)     -- Contained within
)
```

### 2. Project Date Validation

**Location:** `allocation.rs:378-409`

**Purpose:** Ensure allocations don't extend beyond project boundaries

**Business Rule:** Allocation dates must be within the parent project's date range

**Error Messages:** User-friendly validation errors:
- "Allocation start date ({date}) cannot be before project start date ({date})"
- "Allocation end date ({date}) cannot be after project end date ({date})"

### 3. Audit Trail Integration

**Pattern Used:** All mutation handlers follow the same audit pattern

```rust
// 1. Capture "before" state for updates
let audit_changes = audit_payload(
    Some(json!({ "field": old_value, ... })),  // before
    Some(json!({ "field": new_value, ... })),  // after
);

// 2. Execute database operation
let result = sqlx::query!(...).fetch_one(&pool).await?;

// 3. Log audit entry
log_audit(&pool, user_id, "create|update|delete", "entity", id, audit_changes).await?;
```

**Audit Context Extracted from Headers:**
- User ID from JWT token in Authorization header
- Timestamp automatically added
- Full before/after state captured as JSON

### 4. BigDecimal Handling for Percentages

**Location:** `allocation.rs:70-78`

**Challenge:** PostgreSQL `DECIMAL` type maps to `sqlx::types::BigDecimal`, but API uses `f64`

**Conversion Functions:**
```rust
fn bigdecimal_to_f64(bd: sqlx::types::BigDecimal) -> f64 {
    bd.to_string().parse().unwrap_or(0.0)  // String intermediate to preserve precision
}

fn f64_to_bigdecimal(f: f64) -> sqlx::types::BigDecimal {
    sqlx::types::BigDecimal::try_from(f).unwrap_or_default()
}
```

**Note:** String conversion may lose some precision; consider using BigDecimal throughout if exact decimal arithmetic is critical.

---

## Error Handling

### Error Types Used

**From `crate::error` module:**
- `AppError::Database(String)` - SQL errors with message
- `AppError::NotFound(String)` - Resource doesn't exist (404)
- `AppError::Validation(String)` - Business rule violation (400)

### Common Error Patterns

**1. Not Found Check:**
```rust
.fetch_optional(&pool)
.await
.map_err(|e| AppError::Database(e.to_string()))?
.ok_or_else(|| AppError::NotFound(format!("Project {} not found", id)))?
```

**2. Validation Error:**
```rust
if !has_capacity {
    return Err(AppError::Validation(message));
}
```

**3. Database Error:**
```rust
.fetch_all(&pool)
.await
.map_err(|e| AppError::Database(e.to_string()))?
```

### Validation Error Messages

**Capacity Validation:**
- Success: "Resource has sufficient capacity. Daily capacity: {hours} hours"
- Failure: "Resource over-allocated on: {date} ({hours}h allocated), {date2}... Daily capacity: {hours} hours"

**Date Validation:**
- "Allocation start date ({date}) cannot be before project start date ({date})"
- "Allocation end date ({date}) cannot be after project end date ({date})"

---

## Relationships

### Project Ecosystem

```
┌─────────────┐
│   Project   │
│  (project)  │
└──────┬──────┘
       │
       │ has many
       ▼
┌─────────────┐
│ Allocation  │
│(allocation) │◄──────┐
└──────┬──────┘       │
       │              │
       │ belongs to   │
       ▼              │
┌─────────────┐       │
│   Resource  │       │
│  (resource) │───────┘ (indirect via allocation)
└─────────────┘
```

### Dependencies on Other Modules

**allocation.rs imports from:**
- `crate::error::{AppError, Result}` - Error types
- `crate::services::{audit_payload, log_audit, user_id_from_headers}` - Audit utilities

**project.rs imports from:**
- Same error and service modules as allocation

**Database Dependencies:**
- `projects` table - Project data
- `allocations` table - Allocation records
- `resources` table - Resource working hours
- `holidays` table - Holiday dates

---

## Implementation Guidance

### When Modifying Project Routes

**Considerations:**
1. **Date Ranges:** Always ensure `start_date <= end_date` (currently not validated)
2. **Status Values:** Status is a free-form String; consider enum validation
3. **Project Manager:** Currently optional in API but required in DB; ensure consistency

**Example Enhancement - Date Validation:**
```rust
if req.start_date > req.end_date {
    return Err(AppError::Validation(
        "Project start date must be before end date".to_string()
    ));
}
```

### When Modifying Allocation Routes

**Critical Areas:**

1. **Capacity Algorithm Performance:**
   - Current: Loads ALL days into memory as HashMap
   - Optimization: For large date ranges, consider date-bucketing or streaming
   - Benchmark with: Resource with 100+ allocations spanning multiple years

2. **Concurrency:**
   - Race condition: Two simultaneous allocation creates for same resource
   - Current: No explicit locking, relies on application-level coordination
   - Consider: Database constraints or row-level locking for high-volume scenarios

3. **Decimal Precision:**
   - Current: f64 → String → BigDecimal conversion
   - Risk: Floating point errors on percentage calculations
   - Better: Use BigDecimal throughout or integer basis points (e.g., 5000 = 50.00%)

4. **Weekend/Holiday Logic:**
   - Weekend detection: `date.weekday()` - Saturday/Sunday
   - Holiday lookup: Single query per validation
   - Caching opportunity: Holiday list rarely changes; cache in memory

### Adding New Allocation Constraints

**Pattern for New Validation:**
```rust
async fn validate_new_rule(pool: &PgPool, allocation: &NewAllocation) -> Result<()> {
    // Query supporting data
    let data = sqlx::query!("...", params).fetch_optional(pool).await?;
    
    // Check business rule
    if !rule_satisfied {
        return Err(AppError::Validation("Rule violated: ...".to_string()));
    }
    
    Ok(())
}
```

**Insert in create_allocation before database insert:**
```rust
validate_allocation_dates(&pool, ...).await?;
validate_new_rule(&pool, &req).await?;  // Add here
check_resource_capacity(...).await?;
```

---

## Architecture Observations

### Strengths

1. **Comprehensive Audit Trail:** Every mutation logged with full context
2. **Business Logic Isolation:** Complex allocation logic separated into focused functions
3. **Type Safety:** Rust's type system prevents many runtime errors
4. **SQL Safety:** sqlx compile-time checked queries catch typos early
5. **Partial Update Support:** COALESCE pattern allows flexible PUT requests
6. **Denormalized Responses:** Allocation includes project/resource names to reduce API calls

### Potential Improvements

1. **Code Duplication:** Three nearly identical `get_allocations*` functions; could be parameterized
2. **Error Context:** Generic "Database error" messages; could include query context
3. **Testing:** No visible unit tests for complex allocation algorithm
4. **Documentation:** Allocation algorithm deserves inline comments for maintainability
5. **Performance:** No pagination on `get_allocations`; will degrade with large datasets
6. **Transactions:** Each query is separate; consider transactions for multi-step operations

### Design Decisions

**Why allocation.rs is so large (799 lines):**
- Complex business logic for capacity validation
- Helper functions for date/holiday handling
- Multiple query patterns (by project, by resource, all)
- Type conversion utilities (BigDecimal ↔ f64)

**Why ProjectResponse ≠ Project model:**
- Response types may differ from DB schema (e.g., optional fields)
- Allows API evolution independent of database
- Enables denormalization (e.g., joined names in AllocationResponse)

**Why COALESCE for updates:**
```rust
"UPDATE projects SET name = COALESCE($1, name), ..."
```
- Single query handles partial updates
- No conditional SQL generation needed
- Cleaner than building dynamic SQL strings

---

## Related Documentation

- **Auth & User Management:** `deep-dive-auth-user-management.md`
- **Resource & Department Management:** `deep-dive-resource-department-management.md`
- **API Overview:** See `docs/index.md` for API endpoint summary
- **Database Schema:** Refer to migration files in `/migrations/`

---

## Quick Reference

### Key Files for Common Tasks

| Task | File | Function |
|------|------|----------|
| Add project field | `project.rs` | Update all 3 request/response structs |
| Change allocation logic | `allocation.rs` | `calculate_daily_allocations()` |
| Add holiday type | `allocation.rs` | `get_holidays_in_range()` |
| Modify audit format | `allocation.rs` + `project.rs` | `audit_payload()` calls |
| Add capacity constraint | `allocation.rs` | New validator function |

### SQL Table References

```sql
-- Projects
CREATE TABLE projects (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL,
    description TEXT,
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    status VARCHAR NOT NULL,
    project_manager_id UUID REFERENCES users(id)
);

-- Allocations
CREATE TABLE allocations (
    id UUID PRIMARY KEY,
    project_id UUID REFERENCES projects(id),
    resource_id UUID REFERENCES resources(id),
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    allocation_percentage DECIMAL NOT NULL,
    include_weekend BOOLEAN DEFAULT false
);

-- Resources (referenced)
CREATE TABLE resources (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL,
    working_hours DECIMAL DEFAULT 8.0
);

-- Holidays (referenced)
CREATE TABLE holidays (
    id UUID PRIMARY KEY,
    date DATE NOT NULL UNIQUE,
    name VARCHAR
);
```

---

**Document generated:** 2026-02-01  
**Last updated:** 2026-02-01  
**Version:** 1.0
