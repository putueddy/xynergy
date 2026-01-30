# Xynergy System Architecture

## Executive Summary

Xynergy is a **full-stack Rust web application** designed for resource management and project planning. The architecture leverages modern Rust ecosystem tools to deliver high performance, type safety, and excellent developer experience.

**Key Architectural Decisions:**
- **Full-Stack Rust**: Both frontend (Leptos) and backend (Axum) use Rust
- **SSR + CSR**: Server-side rendering for SEO and initial load, client-side hydration for interactivity
- **PostgreSQL**: Robust relational database for complex data relationships
- **Podman**: Rootless containerization for security and portability

---

## System Overview

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Client Layer                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   Browser    │  │   Browser    │  │   Browser    │          │
│  │   (User 1)   │  │   (User 2)   │  │   (User N)   │          │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
└─────────┼─────────────────┼─────────────────┼──────────────────┘
          │                 │                 │
          └─────────────────┼─────────────────┘
                            │ HTTPS
┌───────────────────────────▼───────────────────────────────────┐
│                      Reverse Proxy Layer                       │
│                     ┌──────────────┐                          │
│                     │    Caddy     │                          │
│                     │  (HTTPS/SSL) │                          │
│                     └──────┬───────┘                          │
└────────────────────────────┼──────────────────────────────────┘
                             │
┌────────────────────────────▼──────────────────────────────────┐
│                    Application Layer                           │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │              Leptos SSR Application                      │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │  │
│  │  │   Pages     │  │ Components  │  │   Hooks     │     │  │
│  │  │   (SSR)     │  │   (Rust)    │  │  (Signals)  │     │  │
│  │  └──────┬──────┘  └─────────────┘  └─────────────┘     │  │
│  │         │                                               │  │
│  │         │ API Calls (HTTP/WebSocket)                    │  │
│  │         ▼                                               │  │
│  │  ┌─────────────────────────────────────────────────┐   │  │
│  │  │           Axum Backend API                       │   │  │
│  │  │  ┌──────────┐ ┌──────────┐ ┌──────────┐        │   │  │
│  │  │  │  Routes  │ │ Services │ │Middleware│        │   │  │
│  │  │  └────┬─────┘ └────┬─────┘ └──────────┘        │   │  │
│  │  │       │            │                          │   │  │
│  │  │       └────────────┼──────────────────────────┘   │  │
│  │  └────────────────────┼──────────────────────────────┘  │
│  └───────────────────────┼─────────────────────────────────┘
└──────────────────────────┼──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                      Data Layer                              │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              PostgreSQL Database                         │ │
│  │  ┌────────────┐ ┌────────────┐ ┌────────────┐          │ │
│  │  │   Users    │ │ Resources  │ │  Projects  │          │ │
│  │  └────────────┘ └────────────┘ └────────────┘          │ │
│  │  ┌────────────┐ ┌────────────┐ ┌────────────┐          │ │
│  │  │Allocations │ │Departments │ │Audit Logs  │          │ │
│  │  └────────────┘ └────────────┘ └────────────┘          │ │
│  └─────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

---

## Component Architecture

### 1. Frontend Layer (Leptos)

**Responsibilities:**
- User interface rendering (SSR + CSR)
- Client-side state management
- API communication
- Form handling and validation
- Real-time updates (WebSocket)

**Component Hierarchy:**
```
App (Root)
├── Router
│   ├── Layout (Common layout wrapper)
│   │   ├── Header (Navigation, user menu)
│   │   ├── Sidebar (Main navigation)
│   │   └── Footer
│   │
│   ├── Pages
│   │   ├── Home (Dashboard)
│   │   ├── Login/Register
│   │   ├── Resources
│   │   │   ├── ResourceList
│   │   │   ├── ResourceDetail
│   │   │   └── ResourceForm
│   │   ├── Projects
│   │   │   ├── ProjectList
│   │   │   ├── ProjectDetail
│   │   │   ├── ProjectGantt (Interactive Gantt chart)
│   │   │   └── ProjectForm
│   │   ├── Allocations
│   │   ├── Reports
│   │   └── Admin (Admin panel)
│   │
│   └── Error Pages (404, 500, etc.)
│
└── Global State (Auth, Theme, Notifications)
```

**Key Components:**

| Component | Purpose | Technology |
|-----------|---------|------------|
| `GanttChart` | Interactive resource scheduling | Custom Leptos + SVG |
| `ResourceCalendar` | Resource availability view | Leptos + date utilities |
| `AllocationForm` | Drag-and-drop allocation | Leptos + HTML5 Drag API |
| `Dashboard` | Overview widgets | Leptos + charts library |
| `ReportBuilder` | Custom report generation | Leptos + data tables |

### 2. Backend Layer (Axum)

**Responsibilities:**
- HTTP API endpoints
- Business logic execution
- Database access
- Authentication/authorization
- Input validation
- Error handling

**Layered Architecture:**
```
┌─────────────────────────────────────────┐
│           Router Layer                   │
│  ┌─────────────────────────────────┐   │
│  │  Route Definitions               │   │
│  │  - URL paths                     │   │
│  │  - HTTP methods                  │   │
│  │  - Middleware chain              │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│          Handler Layer                   │
│  ┌─────────────────────────────────┐   │
│  │  Request Handlers                │   │
│  │  - Extract request data          │   │
│  │  - Call services                 │   │
│  │  - Format responses              │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│          Service Layer                   │
│  ┌─────────────────────────────────┐   │
│  │  Business Logic                  │   │
│  │  - Domain operations             │   │
│  │  - Validation rules              │   │
│  │  - Transaction coordination      │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│        Repository Layer                  │
│  ┌─────────────────────────────────┐   │
│  │  Data Access                     │   │
│  │  - SQL queries                   │   │
│  │  - Database transactions         │   │
│  │  - Entity mapping                │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│         Database Layer                   │
│  ┌─────────────────────────────────┐   │
│  │  PostgreSQL                      │   │
│  │  - Tables, Indexes               │   │
│  │  - Constraints                   │   │
│  │  - Migrations                    │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

**API Structure:**

```rust
// Main router composition
pub fn create_app() -> Router {
    Router::new()
        .nest("/api/v1/auth", auth_routes())
        .nest("/api/v1/users", user_routes())
        .nest("/api/v1/resources", resource_routes())
        .nest("/api/v1/projects", project_routes())
        .nest("/api/v1/allocations", allocation_routes())
        .nest("/api/v1/reports", report_routes())
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
}
```

### 3. Database Layer (PostgreSQL)

**Schema Design:**

```sql
-- Core entities
users
├── id (UUID, PK)
├── email (VARCHAR, UNIQUE, INDEX)
├── password_hash (VARCHAR)
├── role (ENUM)
├── department_id (UUID, FK)
├── created_at (TIMESTAMP)
└── updated_at (TIMESTAMP)

resources
├── id (UUID, PK)
├── name (VARCHAR)
├── type (ENUM: human, equipment, room)
├── capacity (DECIMAL)
├── department_id (UUID, FK, INDEX)
├── skills (JSONB)
├── availability_schedule (JSONB)
├── created_at (TIMESTAMP)
└── updated_at (TIMESTAMP)

projects
├── id (UUID, PK)
├── name (VARCHAR)
├── description (TEXT)
├── start_date (DATE, INDEX)
├── end_date (DATE, INDEX)
├── status (ENUM, INDEX)
├── project_manager_id (UUID, FK)
├── metadata (JSONB)
├── created_at (TIMESTAMP)
└── updated_at (TIMESTAMP)

allocations
├── id (UUID, PK)
├── project_id (UUID, FK, INDEX)
├── resource_id (UUID, FK, INDEX)
├── start_date (DATE, INDEX)
├── end_date (DATE, INDEX)
├── allocation_percentage (DECIMAL)
├── created_by (UUID, FK)
├── created_at (TIMESTAMP)
└── updated_at (TIMESTAMP)

departments
├── id (UUID, PK)
├── name (VARCHAR)
├── head_id (UUID, FK)
├── metadata (JSONB)
└── created_at (TIMESTAMP)

audit_logs
├── id (UUID, PK)
├── user_id (UUID, FK, INDEX)
├── action (VARCHAR, INDEX)
├── entity_type (VARCHAR)
├── entity_id (UUID)
├── changes (JSONB)
├── created_at (TIMESTAMP, INDEX)
└── ip_address (INET)
```

**Key Indexes:**
- `users.email` - Login lookups
- `resources.department_id` - Department filtering
- `projects.status, start_date` - Active project queries
- `allocations.project_id, resource_id` - Resource allocation queries
- `allocations.start_date, end_date` - Date range queries
- `audit_logs.created_at` - Log retrieval

---

## Data Flow

### 1. User Authentication Flow

```
User → Login Form
  │
  ▼
Leptos Frontend (SSR)
  │ POST /api/v1/auth/login
  ▼
Axum Backend
  │
  ├── Validate credentials
  ├── Generate JWT tokens
  └── Store refresh token
  │
  ▼
PostgreSQL (users table)
  │
  ▼
Response: Access token + Refresh token
  │
  ▼
Leptos: Store tokens, redirect to dashboard
```

### 2. Resource Allocation Flow

```
User → Gantt Chart Interface
  │
  ▼
Drag-and-drop allocation
  │
  ▼
Leptos: Optimistic UI update
  │ POST /api/v1/allocations
  ▼
Axum Backend
  │
  ├── Validate request
  ├── Check for conflicts
  ├── Create allocation
  ├── Log audit entry
  └── Notify relevant users
  │
  ▼
PostgreSQL (allocations + audit_logs)
  │
  ▼
Response: Created allocation
  │
  ▼
Leptos: Confirm update / rollback on error
```

### 3. Real-time Collaboration Flow

```
User A → Makes change
  │
  ▼
Axum Backend
  │
  ├── Process change
  ├── Broadcast via WebSocket
  └── Persist to database
  │
  ▼
WebSocket Server
  │
  ├── User B (connected)
  ├── User C (connected)
  └── Broadcast update
  │
  ▼
Leptos Clients (B, C)
  │
  └── Receive update → Refresh UI
```

---

## Security Architecture

### Authentication

```
┌─────────────────────────────────────────┐
│         Authentication Flow              │
├─────────────────────────────────────────┤
│                                         │
│  1. User submits credentials            │
│     ↓                                   │
│  2. Password verified with Argon2id     │
│     ↓                                   │
│  3. JWT Access Token generated          │
│     - Expires: 15 minutes               │
│     - Contains: user_id, role           │
│     ↓                                   │
│  4. Refresh Token generated             │
│     - Expires: 7 days                   │
│     - Stored in httpOnly cookie         │
│     ↓                                   │
│  5. Tokens returned to client           │
│                                         │
└─────────────────────────────────────────┘
```

### Authorization (RBAC)

**Roles and Permissions:**

| Role | Permissions |
|------|-------------|
| **Admin** | Full system access |
| **Project Manager** | Create projects, manage allocations, view reports |
| **Resource Planner** | Manage resources, create allocations |
| **Department Head** | View department resources, approve allocations |
| **Team Member** | View own allocations, update availability |
| **Guest** | View-only access to public projects |

**Middleware Chain:**
```rust
Router::new()
    .route("/api/v1/admin/*", 
        get(handler)
        .layer(require_role(Role::Admin)))
    .route("/api/v1/projects",
        post(create_project)
        .layer(require_role(Role::ProjectManager)))
```

### Data Protection

- **Encryption at Rest**: PostgreSQL encryption
- **Encryption in Transit**: TLS 1.3
- **Password Hashing**: Argon2id
- **Input Sanitization**: Server-side validation
- **SQL Injection Prevention**: Parameterized queries (sqlx)
- **XSS Protection**: Leptos automatic escaping
- **CSRF Protection**: Token-based

---

## Scalability Considerations

### Horizontal Scaling

```
                    ┌──────────────┐
                    │  Load Balancer│
                    │   (Caddy)     │
                    └──────┬───────┘
                           │
           ┌───────────────┼───────────────┐
           │               │               │
    ┌──────▼──────┐ ┌──────▼──────┐ ┌──────▼──────┐
    │  Leptos     │ │  Leptos     │ │  Leptos     │
    │  Instance 1 │ │  Instance 2 │ │  Instance N │
    └──────┬──────┘ └──────┬──────┘ └──────┬──────┘
           │               │               │
           └───────────────┼───────────────┘
                           │
                    ┌──────▼──────┐
                    │  PostgreSQL │
                    │   Primary   │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │  PostgreSQL │
                    │   Replica   │
                    └─────────────┘
```

### Performance Optimizations

1. **Database**
   - Connection pooling (sqlx)
   - Query result caching (Redis)
   - Read replicas for reporting
   - Proper indexing strategy

2. **Frontend**
   - Code splitting by route
   - Asset optimization
   - SSR for initial load
   - Lazy loading for heavy components

3. **Backend**
   - Async/await throughout
   - Request batching
   - Response compression
   - Efficient serialization

---

## Deployment Architecture

### Container Strategy (Podman)

```dockerfile
# Multi-stage build
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM gcr.io/distroless/cc
COPY --from=builder /app/target/release/xynergy /xynergy
EXPOSE 3000
CMD ["/xynergy"]
```

### Pod Configuration

```yaml
# podman-compose.yml
version: '3'
services:
  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=postgres://xynergy:password@db:5432/xynergy
      - RUST_LOG=info
    depends_on:
      - db
      
  db:
    image: postgres:15-alpine
    environment:
      - POSTGRES_USER=xynergy
      - POSTGRES_PASSWORD=password
      - POSTGRES_DB=xynergy
    volumes:
      - postgres_data:/var/lib/postgresql/data
      
  caddy:
    image: caddy:2-alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile
      - caddy_data:/data
      - caddy_config:/config
```

---

## Monitoring and Observability

### Logging Strategy

```rust
// Structured logging with tracing
tracing::info!(
    user_id = %user.id,
    action = "resource_allocated",
    resource_id = %resource_id,
    project_id = %project_id,
    "Resource allocated successfully"
);
```

### Health Checks

```
GET /health/live    # Liveness probe (Kubernetes)
GET /health/ready   # Readiness probe
GET /health/metrics # Prometheus metrics
```

### Key Metrics

- Request latency (p50, p95, p99)
- Error rates by endpoint
- Database query performance
- Active WebSocket connections
- Resource utilization

---

## Future Architecture Evolution

### Phase 2 Enhancements

1. **Microservices Split**
   - Extract reporting service
   - Extract notification service
   - Extract audit service

2. **Event-Driven Architecture**
   - Message queue (RabbitMQ/NATS)
   - Event sourcing for allocations
   - CQRS for read-heavy operations

3. **Caching Layer**
   - Redis for session storage
   - Query result caching
   - CDN for static assets

### Phase 3 Enhancements

1. **Multi-tenancy**
   - Tenant isolation
   - Shared database, separate schemas
   - Tenant-specific configurations

2. **Advanced Analytics**
   - Data warehouse (ClickHouse)
   - Real-time dashboards
   - ML-based forecasting

---

## Technology Stack Summary

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Frontend** | Leptos | Reactive web framework |
| **Backend** | Axum | Async web framework |
| **Database** | PostgreSQL | Relational data store |
| **ORM** | sqlx | Type-safe SQL |
| **Auth** | JWT + Argon2 | Authentication |
| **Styling** | Tailwind CSS | Utility-first CSS |
| **Container** | Podman | Container runtime |
| **Proxy** | Caddy | Reverse proxy |
| **Logging** | tracing | Structured logging |

---

*Generated: 2026-01-29*  
*Architecture Version: 1.0*  
*Last Updated: 2026-01-29*
