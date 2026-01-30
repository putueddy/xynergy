# Xynergy Technology Stack

## Overview

Xynergy is built as a **full-stack Rust web application** leveraging modern Rust ecosystem tools for high performance, type safety, and developer productivity.

---

## Core Technologies

### Backend Stack

| Component | Technology | Version | Purpose |
|-----------|------------|---------|---------|
| **Language** | Rust | 1.75+ | Systems programming with memory safety |
| **Web Framework** | Axum | 0.7+ | Async web framework with Tower ecosystem |
| **Database** | PostgreSQL | 15+ | Relational database for data persistence |
| **Database Access** | sqlx | 0.7+ | Async SQL with compile-time checked queries |
| **Authentication** | jsonwebtoken | 9+ | JWT token generation and validation |
| **Password Hashing** | argon2 | 0.5+ | Secure password hashing |
| **Serialization** | serde | 1.0+ | JSON serialization/deserialization |
| **Validation** | validator | 0.16+ | Input validation |
| **Environment** | dotenvy | 0.15+ | Environment variable management |
| **Logging** | tracing | 0.1+ | Structured logging and observability |
| **Error Handling** | thiserror | 1.0+ | Custom error types |

### Frontend Stack

| Component | Technology | Version | Purpose |
|-----------|------------|---------|---------|
| **Language** | Rust | 1.75+ | WebAssembly compilation target |
| **Framework** | Leptos | 0.6+ | Reactive web framework with SSR/CSR |
| **Styling** | Tailwind CSS | 3.4+ | Utility-first CSS framework |
| **Icons** | Lucide | latest | Icon library |
| **HTTP Client** | reqwest | 0.11+ | HTTP client for API calls |
| **State Management** | Leptos Signals | built-in | Reactive state management |
| **Routing** | leptos_router | built-in | Client-side routing |
| **Forms** | leptos_form | TBD | Form handling and validation |

### Infrastructure & DevOps

| Component | Technology | Purpose |
|-----------|------------|---------|
| **Containerization** | Podman | Rootless container runtime |
| **Container Image** | distroless/cc | Minimal, secure container base |
| **Reverse Proxy** | Caddy | Automatic HTTPS, simple config |
| **Database Migrations** | sqlx-cli | Database schema migrations |
| **Build Tool** | cargo | Rust package manager and build system |
| **Task Runner** | cargo-make | Build automation |
| **Hot Reload** | cargo-watch | Development file watching |
| **Formatting** | rustfmt | Code formatting |
| **Linting** | clippy | Static analysis and linting |

---

## Architecture Patterns

### Backend Architecture (Axum)

```
┌─────────────────────────────────────────────┐
│              Axum Application                │
├─────────────────────────────────────────────┤
│  Router Layer    → Route definitions         │
│  Handler Layer   → Request handlers          │
│  Service Layer   → Business logic            │
│  Repository Layer→ Database access           │
│  Database Layer  → PostgreSQL via sqlx       │
└─────────────────────────────────────────────┘
```

**Key Patterns:**
- **Layered Architecture**: Clear separation of concerns
- **Dependency Injection**: State passed through Axum extensions
- **Repository Pattern**: Abstract database operations
- **DTO Pattern**: Separate API models from database models
- **Error Handling**: Centralized error handling with custom types

### Frontend Architecture (Leptos)

```
┌─────────────────────────────────────────────┐
│            Leptos Application                │
├─────────────────────────────────────────────┤
│  Pages/Routes    → Top-level views           │
│  Components      → Reusable UI elements      │
│  Hooks/Signals   → State management          │
│  API Client      → Backend communication     │
│  Utils/Helpers   → Shared utilities          │
└─────────────────────────────────────────────┘
```

**Key Patterns:**
- **Component-Based**: Reusable, composable UI components
- **Reactive Signals**: Fine-grained reactivity with Leptos signals
- **Server-Side Rendering**: SEO-friendly initial page loads
- **Client-Side Hydration**: Interactive SPA after initial load
- **Resource Pattern**: Async data fetching with loading states

---

## Database Schema Overview

### Core Entities

```sql
-- Users and Authentication
users
├── id (UUID, PK)
├── email (VARCHAR, UNIQUE)
├── password_hash (VARCHAR)
├── role (ENUM: admin, manager, planner, member, guest)
├── created_at (TIMESTAMP)
└── updated_at (TIMESTAMP)

-- Resources (People, Equipment, Rooms)
resources
├── id (UUID, PK)
├── name (VARCHAR)
├── type (ENUM: human, equipment, room)
├── capacity (DECIMAL)
├── department_id (UUID, FK)
├── skills (JSONB)
├── availability_schedule (JSONB)
└── metadata (JSONB)

-- Projects
projects
├── id (UUID, PK)
├── name (VARCHAR)
├── description (TEXT)
├── start_date (DATE)
├── end_date (DATE)
├── status (ENUM: planning, active, completed, cancelled)
├── project_manager_id (UUID, FK)
└── metadata (JSONB)

-- Resource Allocations
allocations
├── id (UUID, PK)
├── project_id (UUID, FK)
├── resource_id (UUID, FK)
├── start_date (DATE)
├── end_date (DATE)
├── allocation_percentage (DECIMAL)
├── created_by (UUID, FK)
└── created_at (TIMESTAMP)

-- Departments
departments
├── id (UUID, PK)
├── name (VARCHAR)
├── head_id (UUID, FK)
└── metadata (JSONB)
```

---

## API Design

### RESTful Endpoints

**Authentication**
```
POST   /api/v1/auth/register
POST   /api/v1/auth/login
POST   /api/v1/auth/logout
POST   /api/v1/auth/refresh
GET    /api/v1/auth/me
```

**Resources**
```
GET    /api/v1/resources
POST   /api/v1/resources
GET    /api/v1/resources/:id
PUT    /api/v1/resources/:id
DELETE /api/v1/resources/:id
GET    /api/v1/resources/:id/availability
GET    /api/v1/resources/:id/allocations
```

**Projects**
```
GET    /api/v1/projects
POST   /api/v1/projects
GET    /api/v1/projects/:id
PUT    /api/v1/projects/:id
DELETE /api/v1/projects/:id
GET    /api/v1/projects/:id/resources
POST   /api/v1/projects/:id/allocations
```

**Allocations**
```
GET    /api/v1/allocations
POST   /api/v1/allocations
GET    /api/v1/allocations/:id
PUT    /api/v1/allocations/:id
DELETE /api/v1/allocations/:id
```

---

## Development Toolchain

### Required Tools

```bash
# Rust toolchain
rustup update
rustup component add rustfmt clippy

# Database tools
cargo install sqlx-cli --no-default-features --features native-tls,postgres

# Development utilities
cargo install cargo-watch cargo-make

# Node.js (for Tailwind)
npm install -g tailwindcss
```

### VS Code Extensions

- **rust-analyzer**: Rust language support
- **Tailwind CSS IntelliSense**: CSS class autocomplete
- **Even Better TOML**: Cargo.toml editing
- **PostgreSQL**: Database management
- **Error Lens**: Inline error display

---

## Performance Considerations

### Backend Optimizations

- **Connection Pooling**: sqlx connection pool for database
- **Async/Await**: Full async stack with Tokio runtime
- **Caching**: Redis for session and query caching (future)
- **Query Optimization**: Compile-time checked SQL with sqlx
- **Request Batching**: Batch API requests where appropriate

### Frontend Optimizations

- **WebAssembly**: Rust compiled to WASM for near-native performance
- **SSR**: Server-side rendering for fast initial loads
- **Code Splitting**: Lazy load routes and components
- **Signal-Based Reactivity**: Minimal DOM updates
- **Asset Optimization**: Optimized images and fonts

### Database Optimizations

- **Indexing**: Strategic indexes on query columns
- **Query Planning**: EXPLAIN ANALYZE for query optimization
- **Connection Pooling**: Efficient connection reuse
- **Read Replicas**: Future scaling with read replicas

---

## Security Considerations

### Authentication & Authorization

- **JWT Tokens**: Stateless authentication with short-lived access tokens
- **Refresh Tokens**: Long-lived refresh tokens for session continuity
- **Role-Based Access**: RBAC with 6 distinct roles
- **Password Security**: Argon2id for password hashing
- **HTTPS Only**: All traffic encrypted in production

### Data Protection

- **SQL Injection Prevention**: Parameterized queries via sqlx
- **XSS Protection**: Leptos automatic escaping
- **CSRF Protection**: Token-based CSRF protection
- **Input Validation**: Server-side validation on all inputs
- **Audit Logging**: Track all data modifications

### Infrastructure Security

- **Rootless Containers**: Podman containers run without root
- **Minimal Base Images**: Distroless container images
- **Secret Management**: Environment variables for secrets
- **Network Isolation**: Container network segmentation
- **Security Headers**: HSTS, CSP, X-Frame-Options

---

## Monitoring & Observability

### Logging

- **Structured Logging**: JSON format with tracing
- **Log Levels**: DEBUG, INFO, WARN, ERROR
- **Correlation IDs**: Request tracing across services
- **Sensitive Data**: Automatic redaction of PII

### Metrics (Future)

- **Prometheus**: Metrics collection
- **Grafana**: Visualization and dashboards
- **Key Metrics**: Request latency, error rates, DB query times

### Health Checks

```
GET /health/live    # Liveness probe
GET /health/ready   # Readiness probe
GET /health/metrics # Prometheus metrics
```

---

## Deployment Architecture

### Container Strategy

```
┌─────────────────────────────────────────────┐
│              Podman Pod                      │
├─────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐        │
│  │   Caddy      │  │   Leptos     │        │
│  │  (Reverse    │  │   (Frontend  │        │
│  │   Proxy)     │  │    + SSR)    │        │
│  └──────┬───────┘  └──────┬───────┘        │
│         │                 │                │
│  ┌──────┴───────┐  ┌──────┴───────┐        │
│  │     Axum     │  │  PostgreSQL  │        │
│  │   (Backend)  │  │  (Database)  │        │
│  └──────────────┘  └──────────────┘        │
└─────────────────────────────────────────────┘
```

### Production Setup

- **Load Balancer**: Caddy with automatic HTTPS
- **Application Server**: Leptos SSR + Axum API
- **Database**: PostgreSQL with persistent volumes
- **Backups**: Automated daily backups
- **SSL/TLS**: Automatic certificate management

---

## Future Enhancements

### Phase 2

- **Real-time Updates**: WebSocket integration for live collaboration
- **Advanced Analytics**: Data warehouse for reporting
- **Mobile App**: Native mobile applications
- **Integrations**: Jira, Trello, Microsoft Project connectors

### Phase 3

- **AI/ML Features**: Predictive resource allocation
- **Multi-tenancy**: SaaS offering with tenant isolation
- **Advanced Permissions**: Fine-grained ACL system
- **Workflow Engine**: Custom approval workflows

---

## Resources & References

### Official Documentation

- [Rust Book](https://doc.rust-lang.org/book/)
- [Axum Documentation](https://docs.rs/axum/)
- [Leptos Book](https://book.leptos.dev/)
- [sqlx Documentation](https://docs.rs/sqlx/)
- [Tailwind CSS Docs](https://tailwindcss.com/docs)
- [Podman Documentation](https://docs.podman.io/)

### Learning Resources

- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Leptos Examples](https://github.com/leptos-rs/leptos/tree/main/examples)
- [Axum Examples](https://github.com/tokio-rs/axum/tree/main/examples)
- [Zero to Production in Rust](https://www.zero2prod.com/)

---

*Generated: 2026-01-29*  
*Tech Stack Version: 1.0*  
*Last Updated: 2026-01-29*
