# Xynergy Project Documentation Index

## Project Overview

**Project Name:** Xynergy  
**Type:** Resource Management Platform (Web Application)  
**Repository Type:** Monolith (Full-Stack)  
**Status:** Greenfield - Planning Phase  

### Quick Reference

- **Tech Stack:** Rust (Leptos + Axum), PostgreSQL, Tailwind CSS
- **Architecture Pattern:** Full-Stack Rust with SSR/CSR capabilities
- **Containerization:** Podman
- **Primary Language:** Rust
- **Database:** PostgreSQL

---

## Planning Artifacts

### Product Requirements
- [Project Requirements Document](../project_requirements_document.md) - Comprehensive PRD with features, user flows, and scope

### Architecture & Technical Planning
- [Architecture](./architecture.md) - System design, data flow, and component architecture
- [Technology Stack Details](./tech-stack.md) - Complete technology stack documentation

---

## Development Documentation

### Getting Started
- [Development Guide](./development-guide.md) - Coding standards, workflow, and best practices
- [Environment Setup](./environment-setup.md) - Step-by-step development environment setup

### API & Data
- [API Contracts](./api-contracts.md) _(To be generated)_
- [Data Models](./data-models.md) _(To be generated)_

### Frontend
- [Component Inventory](./component-inventory.md) _(To be generated)_
- [UI/UX Guidelines](./ui-guidelines.md) _(To be generated)_

### Deployment
- [Deployment Guide](./deployment-guide.md) _(To be generated)_
- [Containerization](./containerization.md) _(To be generated)_

---

## Project Structure

```
xynergy/
├── _bmad/                    # BMAD framework files
├── _bmad-output/             # Generated artifacts
│   ├── planning-artifacts/   # Planning documents
│   └── implementation-artifacts/  # Implementation docs
├── docs/                     # Project documentation (this folder)
├── src/                      # Source code (to be created)
│   ├── frontend/            # Leptos frontend
│   ├── backend/             # Axum backend
│   └── shared/              # Shared types and utilities
├── Cargo.toml               # Rust workspace configuration
├── Dockerfile               # Podman container definition
└── project_requirements_document.md  # PRD
```

---

## Next Steps

### Immediate Actions
1. **Set up Rust workspace** with Leptos and Axum
2. **Initialize PostgreSQL** schema design
3. **Configure Tailwind CSS** for styling
4. **Set up Podman** containerization
5. **Create development environment** documentation

### Documentation to Generate
Once implementation begins, the following documents will be auto-generated:
- Architecture documentation
- API contracts from backend code
- Data models from database schema
- Component inventory from frontend code
- Deployment and containerization guides

---

## Tech Stack Details

### Backend (Axum)
- **Framework:** Axum - Modern Rust web framework
- **Database:** PostgreSQL with sqlx or diesel
- **Authentication:** JWT-based auth (to be implemented)
- **API Style:** RESTful with potential GraphQL extension

### Frontend (Leptos)
- **Framework:** Leptos - Rust-based reactive web framework
- **Styling:** Tailwind CSS with custom components
- **Rendering:** SSR (Server-Side Rendering) + CSR (Client-Side Rendering)
- **State Management:** Leptos signals and stores

### Infrastructure
- **Containerization:** Podman (rootless containers)
- **Database:** PostgreSQL 15+
- **Reverse Proxy:** nginx or Caddy (to be determined)
- **Development:** cargo-watch, hot-reload setup

---

## Resources

- [Leptos Documentation](https://leptos.dev/)
- [Axum Documentation](https://docs.rs/axum/)
- [Tailwind CSS Documentation](https://tailwindcss.com/)
- [Podman Documentation](https://docs.podman.io/)

---

*Generated: 2026-01-29*  
*Last Updated: 2026-01-29*
*Documents Generated: 4*  
*Documentation Version: 1.0*
