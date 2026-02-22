# Deep-Dive: Auth & User Management API

**Generated:** 2026-02-01  
**Scope:** Backend authentication and user management routes  
**Files Analyzed:** 2 (auth.rs, user.rs) - 435 LOC  

---

## 1. Overview

The Xynergy backend implements authentication and user management through two primary route modules:
- **`auth.rs`** - Authentication layer (143 lines): JWT-based login system with Argon2 password hashing
- **`user.rs`** - User CRUD operations (292 lines): Full user lifecycle management with audit logging

**Architecture Pattern:** Axum-based REST API with PostgreSQL (sqlx) backend, featuring JWT stateless authentication and comprehensive audit trails.

---

## 2. File-by-File Breakdown

### 2.1 auth.rs - Authentication Module

#### Imports & Dependencies
- **Cryptography:** argon2 (password hashing), rand (salt generation)
- **Web Framework:** axum (routing, extractors)
- **JWT:** jsonwebtoken (encoding)
- **Time:** chrono (expiration timestamps)
- **Database:** sqlx::PgPool, uuid::Uuid
- **Internal:** AppError, User model

#### Exported Items

| Item | Type | Description |
|------|------|-------------|
| `Claims` | Struct | JWT payload structure |
| `LoginRequest` | Struct | POST /auth/login request body |
| `LoginResponse` | Struct | Login success response |
| `UserInfo` | Struct | Sanitized user data for response |
| `hash_password` | Function | Argon2 password hashing (pub) |
| `auth_routes` | Function | Route builder (pub) |

#### Core Functions

**`hash_password(password: &str) -> Result<String>`**
- Uses Argon2 with randomly generated salt
- Returns `AppError::Internal` on failure
- **Used by:** login (verify), create_user (hash)

**`verify_password(password: &str, hash: &str) -> Result<bool>`**
- Private helper for login verification
- Parses PHC string format hash

**`generate_token(user: &User) -> Result<String>`**
- Creates 24-hour JWT with HS256
- Requires `JWT_SECRET` environment variable
- Claims: sub (UUID), email, role, exp, iat

**`login(State, Json) -> Result<Json<LoginResponse>>`**
- Queries user by email
- Verifies password with Argon2
- Returns JWT + user info on success
- **Security:** Generic error "Invalid credentials" (prevents user enumeration)

---

### 2.2 user.rs - User Management Module

#### Imports & Dependencies
- **Web Framework:** axum (routing, extractors, headers)
- **Serialization:** serde, serde_json
- **Database:** sqlx::PgPool, uuid::Uuid
- **Internal:** AppError, hash_password (from auth), audit services

#### Exported Items

| Item | Type | Description |
|------|------|-------------|
| `CreateUserRequest` | Struct | POST /users request body |
| `UpdateUserRequest` | Struct | PUT /users/:id request body |
| `user_routes` | Function | Route builder (pub) |

#### Core Functions

**`get_users(State) -> Result<Json<Value>>`**
- Returns all users (password_hash excluded)
- Ordered by last_name, first_name
- Uses `sqlx::query!` macro for compile-time checking

**`get_user(State, Path) -> Result<Json<Value>>`**
- Single user lookup by UUID
- Returns `AppError::NotFound` if missing

**`create_user(State, HeaderMap, Json) -> Result<Json<Value>>`**
- **Audit Integration:** Logs creation with before/after diff
- **Auth Context:** Extracts actor ID from headers via `user_id_from_headers`
- Hashes password using shared `hash_password` from auth module

**`update_user(State, HeaderMap, Path, Json) -> Result<Json<Value>>`**
- **Partial Updates:** Uses `COALESCE` to preserve unspecified fields
- **Audit Trail:** Captures complete before/after state
- **Empty Check:** Returns current user if no fields provided
- All fields optional: email, first_name, last_name, role, department_id

**`delete_user(State, HeaderMap, Path) -> Result<Json<Value>>`**
- **Hard Delete:** No archival (permanent removal)
- **Audit:** Logs deleted user data as "before" state

---

## 3. API Endpoints Reference

### Authentication Endpoints

| Method | Path | Handler | Auth Required | Description |
|--------|------|---------|---------------|-------------|
| POST | `/api/v1/auth/login` | `login` | No | Authenticate, receive JWT |

### User Management Endpoints

| Method | Path | Handler | Auth Required | Description |
|--------|------|---------|---------------|-------------|
| GET | `/api/v1/users` | `get_users` | Yes | List all users |
| POST | `/api/v1/users` | `create_user` | Yes | Create new user |
| GET | `/api/v1/users/:id` | `get_user` | Yes | Get user by UUID |
| PUT | `/api/v1/users/:id` | `update_user` | Yes | Update user (partial) |
| DELETE | `/api/v1/users/:id` | `delete_user` | Yes | Delete user permanently |

---

## 4. Data Structures

### Request/Response Types

#### Authentication

```rust
// JWT Claims (internal)
pub struct Claims {
    pub sub: String,      // User ID (UUID as string)
    pub email: String,    // User email
    pub role: String,     // Role identifier
    pub exp: usize,       // Unix timestamp expiration
    pub iat: usize,       // Unix timestamp issued
}

// POST /auth/login
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

// 200 OK Response
pub struct LoginResponse {
    pub token: String,    // JWT bearer token
    pub user: UserInfo,
}
```

#### User Management

```rust
// POST /users
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    pub department_id: Option<Uuid>,
}

// PUT /users/:id (all fields optional)
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: Option<String>,
    pub department_id: Option<Uuid>,
}
```

---

## 5. Authentication Flow

```
Client → POST /auth/login → auth.rs::login()
  ↓
Query User by Email
  ↓
Argon2 Verify Password
  ↓
Generate JWT (24hr expiry)
  ↓
Return {token, user}

Subsequent Requests:
Client → Authorization: Bearer <JWT> → user.rs
  ↓
Extract & Validate JWT
  ↓
Process Request
  ↓
Return JSON Response
```

**Token Claims:**
- `sub`: User ID (UUID as string)
- `email`: User email address
- `role`: User role (admin, project_manager, team_member)
- `exp`: Expiration timestamp (24 hours from issue)
- `iat`: Issued-at timestamp

---

## 6. Error Handling Patterns

### Error Types

| Scenario | Error Type | Message | HTTP Status |
|----------|-----------|---------|-------------|
| Invalid login credentials | `Authentication` | "Invalid credentials" | 401 |
| User not found | `NotFound` | "User {id} not found" | 404 |
| Database query failure | `Database` | Context-specific | 500 |
| Password hashing failure | `Internal` | "Password hashing failed: {}" | 500 |
| Missing JWT_SECRET | `Internal` | "JWT_SECRET not set" | 500 |

**Security Note:** Login failures return generic "Invalid credentials" to prevent user enumeration attacks.

---

## 7. Cross-Module Dependencies

### Shared Components

```
user.rs ──────────────────────────► auth.rs
    │                                │
    │ use hash_password;             │ hash_password()
    │                                │
    ▼                                ▼
┌─────────────┐            ┌─────────────┐
│  User CRUD  │            │   Auth      │
│  Operations │            │   Module    │
└─────────────┘            └─────────────┘
```

### Shared Structures

| Structure | Defined In | Used By | Purpose |
|-----------|-----------|---------|---------|
| `hash_password` | `auth.rs` | `user.rs` | Consistent password hashing |
| `User` model | `crate::models` | Both | Database entity |
| `AppError` | `crate::error` | Both | Unified error handling |
| `PgPool` | sqlx | Both | Database connection |

---

## 8. Implementation Guidance

### When Modifying auth.rs

**Risks:**
- JWT secret exposure in logs
- Timing attacks on password verification
- Token expiration handling

**Verification Steps:**
1. Test login with valid credentials
2. Test login with invalid password (verify generic error)
3. Verify token expiration after 24 hours
4. Check JWT_SECRET environment variable requirement

**Testing:**
```bash
# Login endpoint
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"password"}'
```

### When Modifying user.rs

**Risks:**
- Audit log inconsistency
- Partial update logic errors (COALESCE)
- UUID parsing failures
- Authorization bypass

**Verification Steps:**
1. Test CRUD operations with various user roles
2. Verify audit logs are created for create/update/delete
3. Test partial updates (only changing one field)
4. Verify password is never returned in responses

**Testing:**
```bash
# Create user
curl -X POST http://localhost:3000/api/v1/users \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"email":"new@example.com","password":"pass","first_name":"New","last_name":"User","role":"team_member"}'

# Update user (partial)
curl -X PUT http://localhost:3000/api/v1/users/<id> \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"first_name":"Updated"}'
```

---

## 9. Architecture Observations

1. **Separation of Concerns:** Clean split between authentication (stateless) and user management (stateful)
2. **Audit Integration:** User routes tightly coupled to audit system; auth routes have no audit (login is self-evident)
3. **Password Handling:** Single source of truth for hashing (auth.rs exports, user.rs imports)
4. **Authentication Context:** User routes depend on external service for JWT validation
5. **Security:** Argon2 for password hashing, JWT for session management, generic error messages

---

## 10. Related Code References

### Similar Patterns in Codebase

- **Department Management** (`department.rs`): Similar CRUD pattern with COALESCE updates
- **Resource Management** (`resource.rs`): Similar audit integration pattern
- **Settings Pages** (frontend): Similar form handling for user management

### Reusable Components

- `hash_password()` - Can be used for password reset functionality
- `AppError` handling pattern - Consistent across all route modules
- Audit logging integration - Pattern reusable for other resources

---

## Summary Statistics

| Metric | auth.rs | user.rs | Total |
|--------|---------|---------|-------|
| Lines of Code | 143 | 292 | 435 |
| Public Exports | 6 | 3 | 9 |
| API Endpoints | 1 | 5 | 6 |
| Database Queries | 1 | 6 | 7 |

**Key Insight:** The architecture demonstrates clear separation between authentication concerns (JWT, password hashing) and business logic (user CRUD), with audit logging integrated at the user management layer for compliance tracking.
