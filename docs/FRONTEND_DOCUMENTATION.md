# Frontend Documentation

## Overview

The Xynergy frontend is built with **Leptos** - a modern Rust web framework that compiles to WebAssembly.

## Technology Stack

- **Leptos 0.6** - Reactive web framework
- **Tailwind CSS v4** - Utility-first CSS
- **WebAssembly (WASM)** - Compiled Rust code
- **Reqwest** - HTTP client for API calls

## Project Structure

```
src/frontend/
├── src/
│   ├── lib.rs           # Main application entry
│   ├── auth.rs          # Authentication context
│   ├── components/      # Reusable UI components
│   │   └── mod.rs       # Header, Footer, Buttons
│   └── pages/           # Route pages
│       ├── home.rs      # Landing page
│       ├── login.rs     # Login form
│       ├── dashboard.rs # User dashboard
│       └── not_found.rs # 404 page
├── public/              # Static assets
│   ├── index.html       # HTML template
│   └── output.css       # Compiled Tailwind CSS
├── style/
│   └── tailwind.css     # Tailwind input
└── Cargo.toml           # Dependencies
```

## Pages

### 1. Home ("/")
Landing page with project overview and feature highlights.

### 2. Login ("/login")
- Email/password form
- JWT token storage in localStorage
- Automatic redirect to dashboard on success
- Error handling for invalid credentials

**Default Credentials:**
- Email: admin@xynergy.com
- Password: admin123

### 3. Dashboard ("/dashboard")
- User welcome message
- Role display
- Quick stats cards
- Logout functionality
- Protected route (redirects to login if not authenticated)

### 4. Not Found ("/*any")
404 error page.

## Authentication Flow

1. **Login** - User submits credentials
2. **API Call** - Frontend sends POST to `/api/v1/auth/login`
3. **Token Storage** - JWT token saved to localStorage
4. **State Update** - AuthContext updated with user info
5. **Redirect** - User redirected to dashboard
6. **Protected Routes** - Dashboard checks authentication
7. **Logout** - Token removed, state cleared

## Components

### Header
Navigation bar with logo and links.

### Footer
Copyright information.

### Buttons
- `btn-primary` - Blue action button
- `btn-secondary` - Gray secondary button

## Building

### Development Build
```bash
# Build CSS
cd src/frontend
npm run watch

# Build Rust (in another terminal)
cargo build --package xynergy-frontend
```

### Production Build
```bash
# Build optimized CSS
npm run build

# Build WASM
cargo build --package xynergy-frontend --target wasm32-unknown-unknown --release
```

## Styling

Uses Tailwind CSS v4 with custom utilities:
- `.btn` - Base button styles
- `.btn-primary` - Blue primary button
- `.btn-secondary` - Gray secondary button
- `.card` - Card container
- `.input` - Form input

## State Management

Uses Leptos signals for reactive state:
- `RwSignal` - Read-write signals
- `Memo` - Computed values
- `provide_context` / `use_context` - Dependency injection

## API Integration

All API calls use `reqwest` with JSON:
```rust
// Login example
let response = reqwest::Client::new()
    .post("http://localhost:3000/api/v1/auth/login")
    .json(&request)
    .send()
    .await?;
```

## Browser Storage

Uses localStorage for:
- JWT token persistence
- Auto-login on page refresh

## Routing

Uses `leptos_router`:
- `/` - Home
- `/login` - Login
- `/dashboard` - Dashboard (protected)
- `/*any` - 404

## Next Steps

To complete the frontend:
1. Build resource management pages
2. Create project listing interface
3. Add Gantt chart visualization
4. Implement resource allocation UI
5. Add real-time updates with WebSockets

## Testing

```bash
# Start backend
cargo run --bin xynergy-server

# Build frontend
cargo build --package xynergy-frontend

# Open browser
http://localhost:3000
```
