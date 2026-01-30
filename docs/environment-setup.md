# Development Environment Setup

## Prerequisites

Before setting up the Xynergy development environment, ensure you have the following installed:

### Required Software

- **Rust** (1.75 or later)
- **PostgreSQL** (15 or later)
- **Node.js** (18 or later) - for Tailwind CSS
- **Podman** (4.0 or later) - for containerization
- **Git** - for version control

---

## Step-by-Step Setup

### 1. Install Rust

```bash
# Install rustup (Rust toolchain installer)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Reload shell environment
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version

# Add required components
rustup component add rustfmt clippy
```

### 2. Install PostgreSQL

**macOS (using Homebrew):**
```bash
brew install postgresql@15
brew services start postgresql@15

# Create database and user
createdb xynergy
createuser -P xynergy_user
```

**Linux (Ubuntu/Debian):**
```bash
sudo apt update
sudo apt install postgresql postgresql-contrib

# Start PostgreSQL
sudo systemctl start postgresql
sudo systemctl enable postgresql

# Create database and user
sudo -u postgres createdb xynergy
sudo -u postgres createuser -P xynergy_user
```

### 3. Install Node.js

**macOS:**
```bash
brew install node
```

**Linux:**
```bash
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt install -y nodejs
```

### 4. Install Podman

**macOS:**
```bash
brew install podman
podman machine init
podman machine start
```

**Linux:**
```bash
sudo apt install podman
```

### 5. Install Cargo Tools

```bash
# Database migrations
cargo install sqlx-cli --no-default-features --features native-tls,postgres

# Development utilities
cargo install cargo-watch cargo-make

# Verify installations
sqlx --version
cargo make --version
```

### 6. Install Tailwind CSS

```bash
npm install -g tailwindcss

# Verify installation
tailwindcss --version
```

---

## Project Setup

### 1. Clone the Repository

```bash
git clone <repository-url>
cd xynergy
```

### 2. Set Up Environment Variables

Create a `.env` file in the project root:

```env
# Database
DATABASE_URL=postgres://xynergy_user:your_password@localhost:5432/xynergy

# Application
APP_ENV=development
APP_PORT=3000
APP_HOST=127.0.0.1

# JWT
JWT_SECRET=your-super-secret-jwt-key-change-in-production
JWT_EXPIRATION=3600

# Frontend
LEPTOS_SITE_ADDR=127.0.0.1:3000
LEPTOS_RELOAD_PORT=3001
```

### 3. Set Up the Database

```bash
# Create database (if not already created)
sqlx database create

# Run migrations
sqlx migrate run

# Or create initial migration
sqlx migrate add initial_schema
```

### 4. Install Dependencies

```bash
# Install Rust dependencies
cargo build

# Install Node.js dependencies (for Tailwind)
npm install
```

---

## Development Workflow

### Running the Application

**Option 1: Using Cargo Watch (Recommended for Development)**

```bash
# Terminal 1: Backend with hot reload
cd src/backend
cargo watch -x run

# Terminal 2: Frontend with hot reload
cd src/frontend
cargo leptos watch
```

**Option 2: Using Cargo Make**

```bash
# Run both frontend and backend
cargo make dev

# Run only backend
cargo make dev-backend

# Run only frontend
cargo make dev-frontend
```

**Option 3: Manual Start**

```bash
# Start backend
cd src/backend
cargo run

# Start frontend (in another terminal)
cd src/frontend
cargo leptos serve
```

### Accessing the Application

- **Frontend**: http://localhost:3000
- **Backend API**: http://localhost:3000/api
- **Health Check**: http://localhost:3000/health

---

## Code Quality Tools

### Formatting

```bash
# Format all Rust code
cargo fmt

# Check formatting without making changes
cargo fmt -- --check
```

### Linting

```bash
# Run Clippy (Rust linter)
cargo clippy --all-targets --all-features

# Run Clippy with all warnings as errors
cargo clippy -- -D warnings
```

### Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests with coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

---

## Database Operations

### Migrations

```bash
# Create a new migration
sqlx migrate add migration_name

# Run pending migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert

# Check migration status
sqlx migrate info
```

### Database Reset

```bash
# Drop and recreate database
sqlx database drop
sqlx database create
sqlx migrate run
```

### Seeding Data

```bash
# Run seed script
cargo run --bin seed

# Or use sqlx
cat seeds/initial_data.sql | psql $DATABASE_URL
```

---

## Container Development

### Building the Container

```bash
# Build the Podman image
podman build -t xynergy:latest .

# Run the container
podman run -p 3000:3000 --env-file .env xynergy:latest
```

### Development with Podman Compose

```bash
# Start all services
podman-compose up -d

# View logs
podman-compose logs -f

# Stop all services
podman-compose down
```

---

## Troubleshooting

### Common Issues

**1. PostgreSQL Connection Refused**
```bash
# Check if PostgreSQL is running
brew services list | grep postgresql  # macOS
sudo systemctl status postgresql      # Linux

# Start PostgreSQL if not running
brew services start postgresql@15     # macOS
sudo systemctl start postgresql       # Linux
```

**2. sqlx Compile-Time Errors**
```bash
# Ensure DATABASE_URL is set
export DATABASE_URL=postgres://xynergy_user:password@localhost:5432/xynergy

# Prepare sqlx queries (for offline mode)
cargo sqlx prepare
```

**3. Port Already in Use**
```bash
# Find process using port 3000
lsof -i :3000

# Kill the process
kill -9 <PID>
```

**4. Tailwind CSS Not Updating**
```bash
# Rebuild Tailwind
npx tailwindcss -i ./input.css -o ./output.css --watch

# Or restart the build process
npm run build:css
```

**5. Rust Analyzer Not Working in VS Code**
- Ensure rust-analyzer extension is installed
- Restart VS Code
- Check Output panel > Rust Analyzer for errors

---

## IDE Setup

### VS Code Configuration

Create `.vscode/settings.json`:

```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.checkOnSave.extraArgs": ["--", "-D", "warnings"],
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "rust-lang.rust-analyzer",
  "tailwindCSS.includeLanguages": {
    "rust": "html"
  }
}
```

Create `.vscode/extensions.json`:

```json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "bradlc.vscode-tailwindcss",
    "tamasfe.even-better-toml",
    "ckolkman.vscode-postgres",
    "usernamehw.errorlens"
  ]
}
```

---

## Environment Variables Reference

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | Required |
| `APP_ENV` | Application environment | `development` |
| `APP_PORT` | Backend server port | `3000` |
| `APP_HOST` | Backend server host | `127.0.0.1` |
| `JWT_SECRET` | Secret key for JWT signing | Required |
| `JWT_EXPIRATION` | JWT token expiration (seconds) | `3600` |
| `LEPTOS_SITE_ADDR` | Leptos server address | `127.0.0.1:3000` |
| `LEPTOS_RELOAD_PORT` | Leptos reload port | `3001` |
| `RUST_LOG` | Logging level | `info` |

---

## Useful Commands

```bash
# Check Rust version
rustc --version

# Update Rust
rustup update

# Check dependencies for updates
cargo outdated

# Audit dependencies for security vulnerabilities
cargo audit

# Generate documentation
cargo doc --open

# Build for release
cargo build --release

# Check project without building
cargo check

# Clean build artifacts
cargo clean
```

---

## Next Steps

1. **Verify Setup**: Run `cargo test` to ensure everything is working
2. **Read the Architecture Doc**: Understand the system design
3. **Explore the Code**: Start with `src/main.rs` and `src/lib.rs`
4. **Create Your First Feature**: Follow the development guide

---

*Generated: 2026-01-29*  
*Setup Guide Version: 1.0*
