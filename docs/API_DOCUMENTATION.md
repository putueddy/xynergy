# API Documentation

## Base URL

All API endpoints are prefixed with `/api/v1`

## Endpoints

### Departments

#### GET /api/v1/departments
List all departments

**Response:**
```json
[
  {
    "id": "uuid",
    "name": "Engineering"
  }
]
```

#### GET /api/v1/departments/:id
Get a specific department

**Response:**
```json
{
  "id": "uuid",
  "name": "Engineering"
}
```

### Resources

#### GET /api/v1/resources
List all resources

**Response:**
```json
[
  {
    "id": "uuid",
    "name": "John Developer",
    "resource_type": "human",
    "capacity": "1",
    "department_id": "uuid",
    "skills": ["Rust", "PostgreSQL", "React"]
  }
]
```

#### GET /api/v1/resources/:id
Get a specific resource

**Response:**
```json
{
  "id": "uuid",
  "name": "John Developer",
  "resource_type": "human",
  "capacity": "1",
  "department_id": "uuid",
  "skills": ["Rust", "PostgreSQL", "React"]
}
```

### Projects

#### GET /api/v1/projects
List all projects

**Response:**
```json
[
  {
    "id": "uuid",
    "name": "Xynergy Platform Launch",
    "description": "Initial launch...",
    "start_date": "2026-02-01",
    "end_date": "2026-06-30",
    "status": "planning",
    "project_manager_id": "uuid"
  }
]
```

#### GET /api/v1/projects/:id
Get a specific project

**Response:**
```json
{
  "id": "uuid",
  "name": "Xynergy Platform Launch",
  "description": "Initial launch...",
  "start_date": "2026-02-01",
  "end_date": "2026-06-30",
  "status": "planning",
  "project_manager_id": "uuid"
}
```

### Users

#### GET /api/v1/users
List all users (excludes password_hash)

**Response:**
```json
[
  {
    "id": "uuid",
    "email": "admin@xynergy.com",
    "first_name": "Admin",
    "last_name": "User",
    "role": "admin",
    "department_id": "uuid"
  }
]
```

#### GET /api/v1/users/:id
Get a specific user

**Response:**
```json
{
  "id": "uuid",
  "email": "admin@xynergy.com",
  "first_name": "Admin",
  "last_name": "User",
  "role": "admin",
  "department_id": "uuid"
}
```

## Error Responses

All errors follow this format:

```json
{
  "success": false,
  "error": {
    "code": "NOT_FOUND",
    "message": "Resource not found"
  }
}
}
```

## Status Codes

- **200 OK** - Success
- **404 Not Found** - Resource doesn't exist
- **500 Internal Server Error** - Database or server error

## Testing

```bash
# Get all departments
curl http://localhost:3000/api/v1/departments

# Get all resources
curl http://localhost:3000/api/v1/resources

# Get all projects
curl http://localhost:3000/api/v1/projects

# Get all users
curl http://localhost:3000/api/v1/users

# Get specific resource
curl http://localhost:3000/api/v1/resources/<uuid>
```
