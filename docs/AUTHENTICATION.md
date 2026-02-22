# Authentication Documentation

## Overview

Xynergy uses JWT (JSON Web Token) based authentication with Argon2 password hashing.

## Authentication Flow

1. **Login** - User sends credentials to `/api/v1/auth/login`
2. **Token Generation** - Server validates credentials and returns JWT token
3. **Authenticated Requests** - Client includes token in Authorization header
4. **Token Validation** - Server validates token on protected routes

## Endpoints

### POST /api/v1/auth/login

Authenticate user and receive JWT token.

**Request:**
```json
{
  "email": "admin@xynergy.com",
  "password": "admin123"
}
```

**Success Response (200):**
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "user": {
    "id": "uuid",
    "email": "admin@xynergy.com",
    "first_name": "Admin",
    "last_name": "User",
    "role": "admin"
  }
}
```

**Error Response (401):**
```json
{
  "success": false,
  "error": {
    "code": "AUTHENTICATION_ERROR",
    "message": "Invalid credentials"
  }
}
```

## Using the Token

Include the token in the Authorization header for protected routes:

```
Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...
```

## Token Details

- **Algorithm**: HS256 (HMAC with SHA-256)
- **Expiration**: 24 hours
- **Payload**:
  - `sub`: User ID
  - `email`: User email
  - `role`: User role
  - `exp`: Expiration timestamp
  - `iat`: Issued at timestamp

## Password Security

- **Hashing Algorithm**: Argon2id
- **Parameters**:
  - Memory: 19456 KB
  - Iterations: 2
  - Parallelism: 1

## Environment Variables

```bash
# Required for JWT
JWT_SECRET=your-super-secret-jwt-key-change-in-production

# Optional (default: 3600)
JWT_EXPIRATION=86400  # 24 hours in seconds
```

## Testing

```bash
# Login
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@xynergy.com","password":"admin123"}'

# Access protected route (example)
curl http://localhost:3000/api/v1/protected \
  -H "Authorization: Bearer YOUR_TOKEN_HERE"
```

## Default User

**Email**: admin@xynergy.com  
**Password**: admin123  
**Role**: admin

## Next Steps

To protect routes, add the auth middleware:

```rust
use axum::middleware;
use crate::middleware::auth_middleware;

Router::new()
    .route("/protected", get(protected_handler))
    .route_layer(middleware::from_fn_with_state(pool.clone(), auth_middleware))
```
