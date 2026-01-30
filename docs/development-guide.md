# Development Guide

## Getting Started

Welcome to the Xynergy development team! This guide will help you understand the development workflow, coding standards, and best practices for contributing to the project.

---

## Development Philosophy

### Core Principles

1. **Type Safety First**: Leverage Rust's type system to catch errors at compile time
2. **Performance Matters**: Write efficient code that scales
3. **Security by Design**: Security is not an afterthought
4. **Clean Code**: Readable, maintainable, and well-documented code
5. **Test Coverage**: Comprehensive testing for reliability

### Code Quality Standards

- **No unsafe code** unless absolutely necessary and well-documented
- **All public APIs must be documented**
- **Error handling is mandatory** - no unwrap() in production code
- **Clippy warnings must be resolved** before committing
- **Tests required** for all business logic

---

## Project Structure

```
xynergy/
├── src/
│   ├── backend/           # Axum backend application
│   │   ├── src/
│   │   │   ├── main.rs           # Application entry point
│   │   │   ├── lib.rs            # Library exports
│   │   │   ├── config.rs         # Configuration management
│   │   │   ├── routes/           # API route handlers
│   │   │   ├── models/           # Database models
│   │   │   ├── services/         # Business logic
│   │   │   ├── repositories/     # Database access layer
│   │   │   ├── middleware/       # Axum middleware
│   │   │   └── errors.rs         # Error types
│   │   ├── Cargo.toml
│   │   └── migrations/           # Database migrations
│   │
│   ├── frontend/          # Leptos frontend application
│   │   ├── src/
│   │   │   ├── main.rs           # Entry point
│   │   │   ├── lib.rs            # Library exports
│   │   │   ├── app.rs            # Main app component
│   │   │   ├── pages/            # Route pages
│   │   │   ├── components/       # Reusable components
│   │   │   ├── hooks/            # Custom hooks
│   │   │   ├── api/              # API client
│   │   │   └── utils/            # Utilities
│   │   ├── public/               # Static assets
│   │   ├── style/                # Tailwind CSS
│   │   └── Cargo.toml
│   │
│   └── shared/            # Shared code between frontend and backend
│       ├── src/
│       │   ├── models/           # Shared data models
│       │   ├── validation/       # Shared validation logic
│       │   └── constants.rs      # Shared constants
│       └── Cargo.toml
│
├── docs/                  # Documentation
├── migrations/            # Database migrations
├── tests/                 # Integration tests
├── scripts/               # Build and deployment scripts
├── Cargo.toml            # Workspace configuration
└── Dockerfile            # Container definition
```

---

## Development Workflow

### 1. Branching Strategy

We follow **Git Flow** with the following branches:

- `main` - Production-ready code
- `develop` - Integration branch for features
- `feature/*` - Individual feature branches
- `hotfix/*` - Critical production fixes
- `release/*` - Release preparation

```bash
# Create a feature branch
git checkout develop
git pull origin develop
git checkout -b feature/your-feature-name

# Make your changes
git add .
git commit -m "feat: add new feature"

# Push and create PR
git push origin feature/your-feature-name
```

### 2. Commit Message Convention

We follow **Conventional Commits**:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Test changes
- `chore`: Build process or auxiliary tool changes

**Examples:**
```
feat(auth): add JWT token refresh endpoint

fix(resources): resolve allocation overlap calculation bug

docs(api): update API endpoint documentation

refactor(models): extract common user fields into trait
```

### 3. Code Review Process

1. **Create PR** from feature branch to `develop`
2. **Fill out PR template** with description and testing notes
3. **Ensure CI passes** (tests, clippy, formatting)
4. **Request review** from at least one team member
5. **Address feedback** and update PR
6. **Merge** once approved and CI passes

---

## Coding Standards

### Rust Style Guide

**Formatting:**
```rust
// Use rustfmt (enforced in CI)
// Max line length: 100 characters
// Indentation: 4 spaces

// Good
pub async fn get_user(
    db: &PgPool,
    user_id: Uuid,
) -> Result<User, AppError> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
}

// Bad
pub async fn get_user(db:&PgPool,user_id:Uuid)->Result<User,AppError>{
    sqlx::query_as::<_,User>("SELECT * FROM users WHERE id = $1").bind(user_id).fetch_one(db).await.map_err(|e|AppError::Database(e.to_string()))
}
```

**Error Handling:**
```rust
// Good - Use Result and custom error types
pub fn process_data(input: &str) -> Result<ProcessedData, AppError> {
    let validated = validate_input(input)?;
    let processed = transform_data(validated)?;
    Ok(processed)
}

// Good - Use match for error handling
match result {
    Ok(data) => data,
    Err(e) => {
        tracing::error!("Failed to process: {}", e);
        return Err(AppError::Processing(e.to_string()));
    }
}

// Bad - unwrap() in production code
let data = result.unwrap(); // NEVER DO THIS

// Bad - expect() without good reason
let data = result.expect("should work"); // AVOID THIS
```

**Async/Await:**
```rust
// Good - Proper async function
pub async fn fetch_data(client: &Client) -> Result<Data, Error> {
    let response = client.get("/api/data").send().await?;
    let data = response.json::<Data>().await?;
    Ok(data)
}

// Good - Concurrent operations
let (users, projects) = tokio::join!(
    fetch_users(),
    fetch_projects(),
);

// Good - Spawn tasks for CPU-intensive work
let handle = tokio::spawn(async move {
    heavy_computation(data).await
});
let result = handle.await?;
```

### Leptos Component Patterns

**Component Structure:**
```rust
use leptos::*;

#[component]
pub fn UserProfile(
    #[prop(into)] user_id: Signal<String>,
) -> impl IntoView {
    // State management
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(None::<String>);
    
    // Resource for async data fetching
    let user_resource = create_resource(
        move || user_id.get(),
        |id| async move {
            fetch_user(&id).await
        },
    );
    
    view! {
        <div class="user-profile">
            <Suspense fallback=move || view! { <Loading /> }>
                {move || user_resource.get().map(|result| match result {
                    Ok(user) => view! { <UserCard user=user /> },
                    Err(e) => view! { <ErrorMessage message=e.to_string() /> },
                })}
            </Suspense>
        </div>
    }
}
```

**Signal Management:**
```rust
// Good - Use signals for reactive state
let (count, set_count) = create_signal(0);

// Good - Derived signals
let doubled = move || count.get() * 2;

// Good - Effect for side effects
create_effect(move |_| {
    let current_count = count.get();
    tracing::info!("Count changed to: {}", current_count);
});

// Good - Memo for expensive computations
let expensive_value = create_memo(move |_| {
    expensive_calculation(count.get())
});
```

---

## Testing Guidelines

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_validation() {
        let valid_user = User::new("test@example.com", "password123");
        assert!(valid_user.validate().is_ok());
        
        let invalid_user = User::new("invalid-email", "pwd");
        assert!(invalid_user.validate().is_err());
    }
    
    #[tokio::test]
    async fn test_database_operations() {
        let pool = setup_test_db().await;
        let user = create_test_user(&pool).await;
        
        let fetched = User::find_by_id(&pool, user.id).await.unwrap();
        assert_eq!(fetched.email, user.email);
    }
}
```

### Integration Tests

```rust
// tests/api_tests.rs
use xynergy_backend::app;

#[tokio::test]
async fn test_create_user_endpoint() {
    let app = app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email":"test@example.com","password":"password123"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::CREATED);
}
```

### Frontend Tests

```rust
// Use leptos testing utilities
#[cfg(test)]
mod component_tests {
    use leptos::*;
    
    #[test]
    fn test_button_component() {
        let (clicked, set_clicked) = create_signal(false);
        
        mount_to_body(move || view! {
            <button on:click=move |_| set_clicked.set(true)>
                "Click me"
            </button>
        });
        
        // Simulate click and verify
        // (Use testing library utilities)
    }
}
```

---

## Database Guidelines

### Migration Best Practices

1. **Always use reversible migrations**
```sql
-- Up
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Down
DROP TABLE IF EXISTS users;
```

2. **Never modify existing migrations** - create new ones instead
3. **Test migrations** on a copy of production data before deploying
4. **Use transactions** for complex migrations

### Query Best Practices

```rust
// Good - Use query_as for type safety
let users = sqlx::query_as::<_, User>(
    "SELECT id, email, created_at FROM users WHERE active = $1"
)
.bind(true)
.fetch_all(&pool)
.await?;

// Good - Use transactions for multiple operations
let mut tx = pool.begin().await?;

sqlx::query("INSERT INTO users ...")
    .bind(&email)
    .execute(&mut *tx)
    .await?;

sqlx::query("INSERT INTO profiles ...")
    .bind(&user_id)
    .execute(&mut *tx)
    .await?;

tx.commit().await?;

// Bad - String concatenation in queries (SQL injection risk)
let query = format!("SELECT * FROM users WHERE email = '{}'", email); // NEVER DO THIS
```

---

## API Design Guidelines

### RESTful Endpoints

```rust
// Good - Consistent endpoint structure
GET    /api/v1/resources          // List all
POST   /api/v1/resources          // Create new
GET    /api/v1/resources/:id      // Get one
PUT    /api/v1/resources/:id      // Update (full)
PATCH  /api/v1/resources/:id      // Update (partial)
DELETE /api/v1/resources/:id      // Delete

// Good - Nested resources
GET /api/v1/projects/:id/resources    // Get project resources
GET /api/v1/users/:id/allocations     // Get user allocations
```

### Response Format

```rust
// Success response
{
    "success": true,
    "data": {
        "id": "uuid",
        "name": "Resource Name",
        ...
    }
}

// List response
{
    "success": true,
    "data": [
        { ... },
        { ... }
    ],
    "meta": {
        "total": 100,
        "page": 1,
        "per_page": 20
    }
}

// Error response
{
    "success": false,
    "error": {
        "code": "VALIDATION_ERROR",
        "message": "Invalid input data",
        "details": [
            { "field": "email", "message": "Invalid email format" }
        ]
    }
}
```

---

## Security Checklist

### Authentication & Authorization

- [ ] All endpoints (except public) require authentication
- [ ] JWT tokens have short expiration (15-30 minutes)
- [ ] Refresh tokens are rotated on use
- [ ] Role-based access control enforced
- [ ] Passwords hashed with Argon2id

### Input Validation

- [ ] All user inputs validated on server
- [ ] SQL injection prevented (parameterized queries)
- [ ] XSS protection enabled (Leptos handles this)
- [ ] CSRF tokens for state-changing operations
- [ ] File uploads validated (type, size)

### Data Protection

- [ ] Sensitive data encrypted at rest
- [ ] HTTPS only in production
- [ ] Security headers configured (HSTS, CSP, etc.)
- [ ] Audit logging for sensitive operations
- [ ] PII redacted from logs

---

## Performance Guidelines

### Database Optimization

```rust
// Good - Use indexes for frequent queries
// CREATE INDEX idx_users_email ON users(email);

// Good - Select only needed columns
sqlx::query_as::<_, UserSummary>(
    "SELECT id, name, email FROM users WHERE active = $1"
)

// Good - Use pagination
sqlx::query_as::<_, User>(
    "SELECT * FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2"
)
.bind(limit)
.bind(offset)

// Good - Batch operations
sqlx::query(
    "INSERT INTO allocations (project_id, resource_id) 
     SELECT * FROM UNNEST($1::uuid[], $2::uuid[])"
)
.bind(&project_ids)
.bind(&resource_ids)
```

### Frontend Optimization

```rust
// Good - Use create_memo for expensive calculations
let expensive_computation = create_memo(move |_| {
    compute_something_heavy(data.get())
});

// Good - Lazy load routes
let Routes = || view! {
    <Routes>
        <Route path="/" view=Home/>
        <Route path="/dashboard" view=|| view! { <Dashboard/> }/>
        <Route path="/reports" view=lazy!(|| view! { <Reports/> })/>
    </Routes>
};

// Good - Debounce user input
let (search, set_search) = create_signal("".to_string());
let debounced_search = debounce(search, 300); // 300ms delay
```

---

## Debugging & Troubleshooting

### Logging

```rust
// Use tracing for structured logging
tracing::info!("User logged in: {}", user_email);
tracing::debug!("Processing request: {:?}", request);
tracing::warn!("Rate limit approaching for user: {}", user_id);
tracing::error!(error = ?e, "Failed to process payment");

// In development, use pretty formatting
RUST_LOG=debug cargo run
```

### Common Issues

**1. Database connection pool exhausted**
```rust
// Increase pool size in configuration
let pool = PgPoolOptions::new()
    .max_connections(20) // Increase from default 10
    .connect(&database_url)
    .await?;
```

**2. Leptos hydration mismatch**
```rust
// Ensure server and client render the same content
// Use Suspense for async data
// Avoid random values during SSR
```

**3. Slow compile times**
```bash
# Use cargo check instead of build during development
cargo check

# Enable incremental compilation
export CARGO_INCREMENTAL=1

# Use sccache for caching
export RUSTC_WRAPPER=sccache
```

---

## Deployment

### Pre-deployment Checklist

- [ ] All tests passing
- [ ] Clippy warnings resolved
- [ ] Documentation updated
- [ ] Environment variables configured
- [ ] Database migrations tested
- [ ] Security audit completed
- [ ] Performance benchmarks run

### Build for Production

```bash
# Build release binary
cargo build --release

# Build frontend with optimizations
cd src/frontend
cargo leptos build --release

# Build container
podman build -t xynergy:prod .

# Run production container
podman run -d \
  --name xynergy-prod \
  -p 3000:3000 \
  --env-file .env.production \
  xynergy:prod
```

---

## Resources

### Internal Documentation

- [Architecture](./architecture.md)
- [Tech Stack](./tech-stack.md)
- [Environment Setup](./environment-setup.md)
- [API Documentation](./api-contracts.md)

### External Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Leptos Documentation](https://book.leptos.dev/)
- [Axum Examples](https://github.com/tokio-rs/axum/tree/main/examples)
- [sqlx Documentation](https://docs.rs/sqlx/)

---

## Getting Help

- **Technical Questions**: Ask in #dev-rust channel
- **Architecture Questions**: Tag @architect-team
- **Bug Reports**: Create issue in GitHub with reproduction steps
- **Feature Requests**: Use GitHub Discussions

---

*Generated: 2026-01-29*  
*Development Guide Version: 1.0*
