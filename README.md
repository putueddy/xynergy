# Xynergy

Resource Management and Project Planning Platform built with Rust (Leptos + Axum), PostgreSQL, and Tailwind CSS.

## Tech Stack

- **Frontend**: Leptos (Rust) with Tailwind CSS
- **Backend**: Axum (Rust) with Tower
- **Database**: PostgreSQL with sqlx
- **Containerization**: Podman

## Project Structure

```
xynergy/
├── src/
│   ├── backend/       # Axum backend API
│   ├── frontend/      # Leptos frontend
│   └── shared/        # Shared types and utilities
├── docs/              # Documentation
├── Cargo.toml         # Workspace configuration
└── .env.example       # Environment variables template
```

## Quick Start

1. **Install dependencies**:
   ```bash
   # Rust toolchain
   rustup update
   
   # Database tools
   cargo install sqlx-cli --no-default-features --features native-tls,postgres
   
   # Node.js (for Tailwind)
   npm install -g tailwindcss
   ```

2. **Set up environment**:
   ```bash
   cp .env.example .env
   # Edit .env with your database credentials
   ```

3. **Run the backend**:
   ```bash
   cargo run --bin xynergy-server
   ```

4. **Access the application**:
   - Backend: http://localhost:3000
   - Health check: http://localhost:3000/health

## Documentation

See the `docs/` directory for comprehensive documentation:
- [Architecture](docs/architecture.md)
- [Tech Stack](docs/tech-stack.md)
- [Development Guide](docs/development-guide.md)
- [Environment Setup](docs/environment-setup.md)

## License

MIT
