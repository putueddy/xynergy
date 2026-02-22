---
project_name: 'xynergy'
user_name: 'Putu'
date: '2026-02-22'
sections_completed: ['technology_stack', 'language_specific_rules', 'framework_specific_rules', 'testing_rules', 'code_quality_rules', 'development_workflow_rules', 'critical_dont_miss_rules']
existing_patterns_found: 17
status: 'complete'
rule_count: 50
optimized_for_llm: true
---

# Project Context for AI Agents

_This file contains critical rules and patterns that AI agents must follow when implementing code in this project. Focus on unobvious details that agents might otherwise miss._

---

## Technology Stack & Versions

| Component | Technology | Version | Critical Notes |
|-----------|------------|---------|----------------|
| **Language** | Rust | 1.75+ | Edition 2021, workspace resolver = "2" |
| **Backend Framework** | Axum | 0.7 | Tower ecosystem integration |
| **Frontend Framework** | Leptos | 0.6 | CSR + Hydrate mode (lib), SSR (bin) |
| **Database** | PostgreSQL | 15+ | Via sqlx with compile-time checked queries |
| **Database Access** | sqlx | 0.7 | Features: runtime-tokio, postgres, uuid, chrono, migrate, bigdecimal |
| **Authentication** | JWT | jsonwebtoken 9 | Argon2id for password hashing (argon2 0.5) |
| **Decimal Math** | bigdecimal | 0.4 | For precise allocation percentage calculations |
| **Serialization** | serde | 1.0 | With derive feature enabled |
| **Async Runtime** | tokio | 1.35 | Full feature set |
| **Date/Time** | chrono | 0.4 | NaiveDate for dates, serde support |
| **UUID** | uuid | 1.6 | v4 generation, serde support |
| **Logging** | tracing | 0.1 | Structured logging with tracing-subscriber 0.3 |
| **HTTP Client** | reqwest | 0.11 | Frontend and test dependencies |
| **Validation** | validator | 0.16 | With derive feature |

### Version Constraints & Compatibility

- **Rust Edition 2021**: Required for workspace features
- **Leptos Feature Flags**: 
  - Library: `["csr", "hydrate"]` 
  - Binary: `["ssr"]`
- **sqlx Compile-Time Checking**: SQL queries validated at compile time against live database schema
- **BigDecimal Handling**: f64 conversions via helper functions (see allocation.rs patterns)

---

## Critical Implementation Rules

### Language-Specific Rules (Rust)

**Error Handling Pattern:**
```rust
// ❌ DON'T: Use unwrap in production code
let user = sqlx::query!(...).fetch_one(&pool).await.unwrap();

// ✅ DO: Use AppError with proper error mapping
let user = sqlx::query!(...).fetch_one(&pool).await
    .map_err(|e| AppError::Database(e.to_string()))?;
```

**BigDecimal Type Conversions:**
```rust
// ❌ DON'T: Direct conversion - will fail
let bd: sqlx::types::BigDecimal = percentage.into();

// ✅ DO: Use helper functions
fn bigdecimal_to_f64(bd: sqlx::types::BigDecimal) -> f64 {
    bd.to_string().parse().unwrap_or(0.0)
}
fn f64_to_bigdecimal(f: f64) -> sqlx::types::BigDecimal {
    sqlx::types::BigDecimal::try_from(f).unwrap_or_default()
}
```

**sqlx Query Patterns:**
```rust
// ❌ DON'T: Raw strings without type casting for non-standard types
"SELECT date FROM holidays"

// ✅ DO: Explicit type casts for PostgreSQL → Rust type mapping
"SELECT date::TEXT as \"date!\" FROM holidays"  // DATE → String
```

**Option Handling for 404s:**
```rust
// ✅ Standard pattern across all routes
.fetch_optional(&pool).await
.map_err(|e| AppError::Database(e.to_string()))?
.ok_or_else(|| AppError::NotFound(format!("Entity {} not found", id)))?
```

**Partial Update Pattern (COALESCE):**
```rust
// ✅ Use COALESCE for partial updates instead of dynamic SQL
"UPDATE projects 
 SET name = COALESCE($1, name),
     status = COALESCE($2, status)
 WHERE id = $3"
```

**Critical Rust Rules:**
1. **Never use `.unwrap()`** in production - always map to `AppError`
2. **Always use `fetch_optional` + `ok_or_else`** for single-record lookups
3. **Type cast SQL columns** when using non-standard PostgreSQL types (DATE → TEXT)
4. **Use `COALESCE($1, column)`** pattern for partial updates
5. **Audit trail on every mutation** - call `log_audit()` after successful DB operations
6. **Extract user ID from headers** via `user_id_from_headers(&headers)?` for auth
7. **Use `chrono::NaiveDate`** for date-only fields (no timezone)

---

### Framework-Specific Rules (Axum + Leptos)

**Axum Backend Patterns:**

**Route Organization:**
```rust
// routes/mod.rs - Export pattern
pub mod resource;
pub use resource::resource_routes;

// lib.rs - Router composition
Router::new()
    .nest("/api/v1/resources", resource_routes())
    .nest("/api/v1/projects", project_routes())
```

**Handler Signature Pattern:**
```rust
// ✅ Standard handler structure
async fn handler_name(
    State(pool): State<PgPool>,           // Database pool
    headers: HeaderMap,                    // For auth extraction
    Path(id): Path<Uuid>,                  // URL params
    Json(req): Json<RequestType>,          // Body
) -> Result<Json<ResponseType>> { ... }
```

**Request/Response DTOs:**
```rust
// Separate structs for API vs Database
#[derive(Debug, Serialize)]      // Response
pub struct ResourceResponse { ... }

#[derive(Debug, Deserialize)]    // Create request
pub struct CreateResourceRequest { ... }

#[derive(Debug, Deserialize)]    // Update request (all Option)
pub struct UpdateResourceRequest { 
    pub name: Option<String>,
    pub status: Option<String>,
}
```

**Leptos Frontend Patterns:**

**Component Structure:**
```rust
#[component]
pub fn ComponentName() -> impl IntoView {
    // Signals for state
    let (data, set_data) = create_signal(Vec::new());
    
    // Resource for async data
    let resource = create_resource(|| (), |_| async move {
        fetch_data().await
    });
    
    view! {
        // JSX-like template
    }
}
```

**Route Registration (lib.rs):**
```rust
<Route path="/resources" view=Resources/>
<Route path="/settings" view=SettingsPage>
    <Route path="/holidays" view=SettingsHolidaysPage/>
</Route>
```

**Critical Framework Rules:**

**Axum:**
1. **Use `#[derive(Debug)]`** on all request/response structs
2. **Always nest routes** under `/api/v1/` prefix
3. **Export routes function** named `{module}_routes()` returning `Router<PgPool>`
4. **Include audit logging** in POST/PUT/DELETE handlers before returning
5. **Denormalized responses** - include related names (e.g., `project_name` in AllocationResponse)

**Leptos:**
1. **Mount to `#root` element**, not body (see lib.rs:76-80)
2. **Use `provide_auth_context()`** for global auth state
3. **Console logging** via `web_sys::console::log_1()` for debugging
4. **Feature-gate SSR/CSR** using `#[cfg(feature = "ssr")]`
5. **Shared types** in `xynergy-shared` crate for frontend/backend compatibility

---

### Testing Rules

**Current Status:** Testing infrastructure not yet established.

**When Implementing Tests:**

**Backend Testing Strategy:**
```rust
// Unit tests in module
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_business_logic() {
        // Test with mocked dependencies
    }
}

// Integration tests in tests/ directory
// tests/api_resources.rs
#[tokio::test]
async fn test_api_endpoint() {
    // Full endpoint test with test database
}
```

**sqlx Testing Requirements:**
- Set `DATABASE_URL` environment variable for test database
- Use `#[sqlx::test]` macro for automatic transactions
- Keep test database separate from development database
- Run migrations before test suite

**Critical Testing Rules (When Implemented):**
1. **All route handlers** must have integration tests
2. **Business logic** (allocation validation) needs comprehensive unit tests
3. **Error handling paths** must be tested (404, 400, 500 cases)
4. **Audit logging** should be verified in mutation tests
5. **Database transactions** should rollback after each test

---

### Code Quality & Style Rules

**Tooling:**
- **rustfmt**: Default configuration (no custom rustfmt.toml)
- **clippy**: Default lints (no custom clippy.toml)
- **thiserror**: For all custom error types

**File Naming Conventions:**

| Location | Pattern | Example |
|----------|---------|---------|
| Backend routes | `snake_case.rs` | `resource.rs`, `allocation.rs` |
| Frontend components | `snake_case.rs` | `resource_form.rs`, `settings_sidebar.rs` |
| Module exports | `mod.rs` | `src/routes/mod.rs` |
| Test files | `{module}_test.rs` or `tests/{module}.rs` | `resource_test.rs` |

**Struct Naming:**
- **Response structs**: `{Entity}Response` (e.g., `ResourceResponse`)
- **Create request structs**: `Create{Entity}Request`
- **Update request structs**: `Update{Entity}Request` (all fields `Option<T>`)
- **Database models**: PascalCase singular (e.g., `Resource`, `Project`)

**Function Naming:**
- **Route handlers**: `get_{entity}s`, `create_{entity}`, `update_{entity}`, `delete_{entity}`
- **Route exports**: `{entity}_routes()` → returns `Router<PgPool>`
- **Service functions**: descriptive verbs (e.g., `get_resource_working_hours()`)

**Documentation Requirements:**
- **All public structs**: Must have `#[derive(Debug)]`
- **Public functions**: Use `///` doc comments
- **Module-level**: Document purpose at top of file
- **Complex logic**: Inline comments explaining "why", not "what"

**Code Organization Rules:**
1. **Separate API models from DB models** - never expose internal structures
2. **Keep handlers thin** - business logic in services
3. **One entity per route file** (resource.rs, project.rs, etc.)
4. **Reusable components** in `components/` directory
5. **Page components** in `pages/` directory

**Critical Quality Rules:**
1. **Always derive Debug** on public structs
2. **Use `thiserror` for error types** - never manual Error impl
3. **Explicit type casts** in SQL queries (DATE → TEXT)
4. **No raw unwrap()** - proper error propagation with `?`
5. **Audit logging** mandatory for all mutations
6. **NaiveDate** for dates, not DateTime (unless time needed)

---

### Development Workflow Rules

**Git Workflow:**

**Branch Naming:**
- **Features**: `feat/{feature-name}` (e.g., `feat/department-management`)
- **Bug fixes**: `fix/{bug-description}`
- **Main branch**: `main` (renamed from master, protected)

**Commit Policy:**
- **User preference**: Only commit when explicitly asked
- **Never auto-commit**: Always ask before `git commit`
- **User preference**: Prefer terse, high-signal communication

**Communication Preferences:**
- **Default style**: Terse, high-signal communication
- **Detailed mode**: Provide when explicitly requested
- **Destructive operations**: Always confirm before proceeding

**Development Process:**
1. **Check existing patterns** before implementing new features
2. **Follow audit logging** for all mutations (non-negotiable)
3. **Maintain separation** between API and database models
4. **Update documentation** after completing features (docs/, deep-dives)

**BMAD Integration:**
1. **Follow workflow definitions** exactly from `_bmad/` folder
2. **Use appropriate agents** for specialized tasks (`/tech-writer`, `/analyst`, etc.)
3. **Load config** from `_bmad/bmm/config.yaml` for project settings
4. **Update project-scan-report.json** when adding deep-dive documentation

**Code Review Guidelines:**
- Prefer **small, focused changes** over large refactors
- Ensure **backward compatibility** when modifying APIs
- Add **audit entries** for any data mutations
- Update **docs/index.md** when adding new documentation

---

### Critical Don't-Miss Rules

**Anti-Patterns to Avoid:**

```rust
// ❌ NEVER: Direct BigDecimal conversion
let percentage: f64 = bigdecimal_value.into();

// ✅ ALWAYS: Use helper functions
let percentage = bigdecimal_to_f64(bigdecimal_value);

// ❌ NEVER: Use unwrap() in production code
let user = sqlx::query!(...).fetch_one(&pool).await.unwrap();

// ✅ ALWAYS: Map to AppError
let user = sqlx::query!(...).fetch_one(&pool).await
    .map_err(|e| AppError::Database(e.to_string()))?;

// ❌ NEVER: Forget audit logging
let result = sqlx::query!(...).fetch_one(&pool).await?;
return Ok(Json(result));

// ✅ ALWAYS: Log audit before returning
let result = sqlx::query!(...).fetch_one(&pool).await?;
log_audit(&pool, user_id, "create", "entity", result.id, changes).await?;
return Ok(Json(result));

// ❌ NEVER: Expose DB models directly
#[derive(Serialize)]
pub struct User { ... }  // This is the DB model!

// ✅ ALWAYS: Use DTOs for API responses
#[derive(Debug, Serialize)]
pub struct UserResponse { ... }

// ❌ NEVER: Use DateTime when you only need dates
pub date: DateTime<Utc>,  // Overkill for holiday dates

// ✅ ALWAYS: Use NaiveDate for date-only fields
pub date: chrono::NaiveDate,
```

**Edge Cases Agents Must Handle:**

1. **BigDecimal Precision**: Always convert via String intermediate to preserve precision
2. **JWT Expiration**: Tokens expire - handle 401s gracefully on frontend
3. **Date Ranges**: Check for overlapping allocations before creating new ones
4. **Weekend/Holiday Logic**: Respect `include_weekend` flag in capacity calculations
5. **User Deletion**: Use LEFT JOIN in audit logs - users may be deleted but logs must persist
6. **Concurrent Allocations**: Consider race conditions when checking resource capacity
7. **WASM Caching**: Browser may cache old WASM - users need hard refresh (Ctrl+F5)

**Security Rules:**
1. **Never log passwords or JWT tokens**
2. **Always validate input** with validator crate before DB operations
3. **Use parameterized queries** (sqlx does this automatically)
4. **Check user permissions** before allowing mutations
5. **Sanitize user input** in frontend before sending to API

**Performance Gotchas:**
1. **Don't fetch all records** - implement pagination for list endpoints
2. **Holiday caching**: Consider caching holiday list in allocation calculations
3. **N+1 queries**: Use JOINs to fetch related data in single query
4. **BigDecimal in loops**: Convert once, use f64 for calculations
5. **Audit log growth**: No retention policy - monitor table size

---

## Usage Guidelines

**For AI Agents:**

- Read this file before implementing any code
- Follow ALL rules exactly as documented
- When in doubt, prefer the more restrictive option
- Update this file if new patterns emerge

**For Humans:**

- Keep this file lean and focused on agent needs
- Update when technology stack changes
- Review quarterly for outdated rules
- Remove rules that become obvious over time

---

*Project Context Version: 1.0*  
*Last Updated: 2026-02-22*  
*Rule Count: 50+ critical rules*  
*Status: Complete*
