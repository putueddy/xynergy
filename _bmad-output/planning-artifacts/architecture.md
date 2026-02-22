---
stepsCompleted: ['step-01-init', 'step-02-context', 'step-04-decisions']
note: 'Step 03 (Starter Template) skipped - brownfield project extending existing architecture'
inputDocuments:
  - '/Users/ipei/webdev/xynergy/_bmad-output/planning-artifacts/prd.md'
  - '/Users/ipei/webdev/xynergy/_bmad-output/planning-artifacts/ux-design-specification.md'
  - '/Users/ipei/webdev/xynergy/_bmad-output/project-context.md'
workflowType: 'architecture'
project_name: 'xynergy'
user_name: 'Putu'
date: '2026-02-22'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Base Architecture Reference

**Foundation Document:** `docs/architecture.md` (System Architecture v1.0, 2026-01-29)

The existing architecture provides a solid foundation including:
- Full-stack Rust architecture (Leptos + Axum + PostgreSQL)
- User authentication and RBAC system
- Resources, Projects, Allocations, Departments core entities
- Security model with JWT and audit logging
- Deployment strategy with Podman

**This document extends the base architecture** with architectural decisions for the new HR & Financial Management modules:
- CTC (Cost to Company) Management
- Project Budget & P&L Tracking
- Cash Flow Management
- ERP Integration
- Indonesia Payroll Compliance (BPJS, THR)
- Enhanced Security for Financial Data

---

---

## Project Context Analysis

### Requirements Overview

**Functional Requirements:**

**57 testable capabilities** across 8 capability areas, extending the base Xynergy system:

| Capability Area | FR Count | Key Features |
|:----------------|:---------|:-------------|
| **CTC Management** | 8 | Component breakdown, revision history, Indonesia compliance (BPJS/THR), daily rate calculation |
| **Resource Assignment** | 7 | Cost-aware assignment with instant preview, overallocation warnings, budget impact visibility |
| **Project Budget Management** | 7 | Budget categories, resource cost tracking, non-resource cost entry |
| **P&L Generation** | 7 | Manual revenue entry, margin calculation, forecasting, target tracking |
| **Cash Flow Tracking** | 6 | Cash in/out tracking, cumulative position, variance analysis (Post-MVP) |
| **Access Control & Security** | 9 | RBAC, row-level security, CTC encryption, audit logging |
| **Dashboard & Reporting** | 7 | Role-based dashboards, 30-second polling updates, manual refresh |
| **Audit & Compliance** | 6 | Complete audit trails, CTC validation reports, compliance verification |

**Core Differentiator:** Cost-aware resource assignment with instant financial impact visibility—transforming "blind" allocation into informed decision-making.

**Non-Functional Requirements:**

**45 quality attributes** driving architectural decisions:

| Category | Key Requirements |
|:---------|:-----------------|
| **Performance** | <2s P&L generation, <200ms cost preview, <500ms API response (95th percentile) |
| **Security** | CTC encrypted at rest (PostgreSQL TDE), zero unauthorized access incidents, audit hash chains |
| **Accessibility** | WCAG 2.1 AA compliance, keyboard navigation, 44×44px touch targets |
| **Integration** | ERP API 99.5% uptime, circuit breaker pattern, manual fallback |
| **Reliability** | 99.9% uptime during business hours, automated daily backups, point-in-time recovery |

**Scale & Complexity:**

- **Primary domain:** Full-stack web application (HR & Financial Management)
- **Complexity level:** **High** (financial data sensitivity + Indonesia compliance + external integrations)
- **Estimated new architectural components:** 15-20 (services, routes, models, extensions)
- **Data volume:** 60 employees, moderate transaction rate, financial precision required
- **Timeline:** 4-week MVP (aggressive delivery schedule)

### Technical Constraints & Dependencies

**Existing Constraints (from base architecture):**

| Constraint | Impact |
|:-----------|:-------|
| **Rust + Leptos + Axum + PostgreSQL** | Locked tech stack, must use idiomatic patterns |
| **WebAssembly frontend** | Bundle size constraints, no DOM manipulation libraries |
| **Podman containerization** | Rootless containers, security-first deployment |
| **JWT authentication** | 15-minute token expiry, refresh token rotation |
| **sqlx compile-time checking** | SQL queries validated at build time |

**New Dependencies:**

| Dependency | Purpose | Risk Level |
|:-----------|:--------|:-----------|
| **External ERP system** | Revenue data for P&L | High (external dependency) |
| **Indonesia payroll rules** | BPJS rates, THR calculations | Medium (regulatory complexity) |
| **BigDecimal** | Precise IDR currency calculations | Low (standard library) |
| **Hash chain library** | Audit log integrity | Low (implement in-house or use crate) |

### Cross-Cutting Concerns Identified

**1. Audit & Compliance**
- Every CTC mutation logged with user, timestamp, before/after values
- Cryptographic hash chain prevents tampering
- Compliance reports for Finance team validation

**2. Multi-Level Authorization**
- **Role-based:** HR (full CTC), Dept Head (blended rates), PM (view only)
- **Row-level:** Users see only their department's data
- **Resource-level:** Individual CTC records protected

**3. Financial Calculation Accuracy**
- No floating-point errors (use BigDecimal)
- IDR whole numbers only (no decimals)
- Daily rate = Monthly CTC ÷ 22 working days (configurable)
- BPJS/THR calculations match payroll system exactly

**4. Integration Resilience**
- Circuit breaker: 3 retries then graceful degradation
- Manual revenue entry fallback when ERP unavailable
- Daily reconciliation to catch discrepancies
- Idempotency keys prevent duplicate revenue

**5. Data Privacy & Security**
- CTC data encrypted at rest (PostgreSQL TDE)
- Audit logs in separate table with restricted access
- Export controls (four-eyes approval for bulk export)
- Watermarking with user ID and timestamp

---

## Core Architectural Decisions

### Decision Summary

| Category | Decision | MVP Scope | Post-MVP |
|:---------|:---------|:----------|:---------|
| **Data Architecture** | Separate CTC table with revision history | ✅ Include | Enhanced indexing |
| **Security** | PostgreSQL TDE (app-level encryption Post-MVP) | ✅ Include | Field-level encryption |
| **Integration** | Daily ERP polling + manual fallback | ✅ Include | Webhooks/real-time |
| **Calculation** | Pre-calculated daily rates, configurable working days | ✅ Include | Caching layer |
| **API Design** | REST endpoints following existing patterns | ✅ Include | GraphQL (optional) |
| **Audit** | Hash chain for CTC mutations (simplified) | ✅ Include | Full immutability |

### Data Architecture Decisions

#### Decision: CTC Data Model

**Choice:** Separate CTC table with foreign key to resources

**Rationale:**
- CTC has independent lifecycle (revisions, effective dates)
- Multiple CTC versions per resource (promotions, raises)
- Clear separation of concerns (resource identity vs. financial data)
- Enables precise access control (HR-only table)
- Supports audit trail requirements

**Schema Design:**

```sql
CREATE TABLE ctc_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource_id UUID NOT NULL REFERENCES resources(id),
    
    -- CTC Components (all amounts in IDR, whole numbers)
    base_salary DECIMAL(12,0) NOT NULL,
    hra_allowance DECIMAL(12,0) DEFAULT 0,
    medical_allowance DECIMAL(12,0) DEFAULT 0,
    transport_allowance DECIMAL(12,0) DEFAULT 0,
    meal_allowance DECIMAL(12,0) DEFAULT 0,
    bpjs_kesehatan DECIMAL(12,0) DEFAULT 0,      -- Company portion
    bpjs_ketenagakerjaan DECIMAL(12,0) DEFAULT 0, -- Company portion
    thr_monthly_accrual DECIMAL(12,0) DEFAULT 0,  -- THR ÷ 12
    
    -- Calculated Fields
    total_monthly_ctc DECIMAL(12,0) NOT NULL,
    daily_rate DECIMAL(12,2) NOT NULL,            -- total ÷ working_days
    
    -- Metadata
    effective_date DATE NOT NULL,
    working_days_per_month INTEGER DEFAULT 22,
    revision_number INTEGER DEFAULT 1,
    
    -- Audit
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- Constraints
    CONSTRAINT positive_base_salary CHECK (base_salary > 0),
    CONSTRAINT positive_total_ctc CHECK (total_monthly_ctc > 0)
);

-- Indexes
CREATE INDEX idx_ctc_resource ON ctc_records(resource_id);
CREATE INDEX idx_ctc_effective ON ctc_records(effective_date);
CREATE INDEX idx_ctc_current ON ctc_records(resource_id, effective_date DESC);
```

**Budget Storage (projects table extension):**

```sql
ALTER TABLE projects ADD COLUMN budget_settings JSONB DEFAULT '{
  "total_budget": 0,
  "categories": {
    "hr": 0,
    "software": 0,
    "hardware": 0,
    "materials": 0,
    "overhead": 0,
    "subcontractors": 0
  }
}';
```

### Security Architecture Decisions

#### Decision: CTC Data Protection (MVP)

**Choice:** PostgreSQL Transparent Data Encryption (TDE) only for MVP

**Rationale:**
- 4-week MVP timeline constraints
- TDE provides adequate protection for initial launch
- Application-level encryption adds significant complexity
- Can add defense-in-depth later without schema changes

**MVP Implementation:**
- Enable PostgreSQL TDE for entire database
- Row-level security policies on `ctc_records` table
- Application-level access control (HR role only)

**Post-MVP Enhancement:**
- Application-level field encryption for sensitive CTC components
- Separate encryption keys per field type
- Key rotation strategy

#### Decision: Audit Log Hash Chain (Simplified MVP)

**Choice:** Cryptographic hash chain linking audit entries

**MVP Implementation:**

```rust
struct AuditLog {
    id: Uuid,
    previous_hash: String,        // SHA-256 of previous entry
    entity_type: String,          // "ctc", "allocation", "budget"
    entity_id: Uuid,
    action: String,               // "create", "update", "delete"
    changes: serde_json::Value,   // { before: {}, after: {} }
    user_id: Uuid,
    timestamp: DateTime<Utc>,
    hash: String,                 // SHA-256 of (prev_hash + data + timestamp)
}

impl AuditLog {
    fn calculate_hash(&self) -> String {
        let data = format!(
            "{}:{}:{}:{}:{}:{}",
            self.previous_hash,
            self.entity_type,
            self.entity_id,
            self.action,
            self.changes.to_string(),
            self.timestamp.to_rfc3339()
        );
        sha256(data)
    }
}
```

**Validation:**
- Daily integrity check (verify chain links)
- Alert on any hash mismatch
- WORM storage for audit table (append-only)

### Integration Architecture Decisions

#### Decision: ERP Revenue Integration

**Choice:** Daily polling with manual fallback

**Rationale:**
- Simple to implement within 4-week MVP
- Reliable and predictable
- Manual fallback covers urgent needs
- Easy to enhance later (webhooks, real-time)

**Implementation:**

```rust
// Daily scheduled job (6 AM)
async fn sync_erp_revenue(pool: &PgPool) -> Result<()> {
    // 1. Fetch revenue data from ERP API
    let revenue_data = erp_client::fetch_revenue(
        yesterday_date(),
        yesterday_date()
    ).await?;
    
    // 2. Process with idempotency
    for entry in revenue_data {
        let idempotency_key = format!("{}:{}", entry.project_id, entry.date);
        
        if !idempotency_key_exists(pool, &idempotency_key).await? {
            insert_revenue(pool, entry, &idempotency_key).await?;
        }
    }
    
    // 3. Log sync completion
    log_audit(pool, "erp_sync", "completed", revenue_data.len()).await?;
    Ok(())
}
```

**Error Handling:**
- Circuit breaker: 3 retries with exponential backoff
- After 3 failures: Disable auto-sync, alert admin
- Manual entry UI always available as fallback
- Daily reconciliation report to catch discrepancies

**API Contract (with ERP):**
- **Endpoint:** `GET /api/v1/revenue` (ERP system)
- **Auth:** API key in header
- **Request:** `?start_date=YYYY-MM-DD&end_date=YYYY-MM-DD`
- **Response:** JSON array of revenue entries
- **Idempotency:** Required header `X-Idempotency-Key`

### Calculation Engine Decisions

#### Decision: Daily Rate Calculation Strategy

**Choice:** Pre-calculated and cached, configurable working days

**Rationale:**
- Performance: <200ms cost preview requirement
- Flexibility: Different organizations may use different working day counts
- Accuracy: Calculated once, used many times
- Cache invalidation: Automatic on CTC update

**Implementation:**

```rust
pub struct DailyRateCalculator {
    working_days_per_month: u8, // Configurable, default 22
}

impl DailyRateCalculator {
    pub fn new(working_days: Option<u8>) -> Self {
        Self {
            working_days_per_month: working_days.unwrap_or(22),
        }
    }
    
    pub fn calculate(&self, monthly_ctc: &BigDecimal) -> BigDecimal {
        monthly_ctc / BigDecimal::from(self.working_days_per_month)
    }
}

// Caching strategy (MVP: Application memory, Post-MVP: Redis)
pub struct RateCache {
    cache: DashMap<Uuid, CachedRate>, // resource_id -> rate
}

struct CachedRate {
    daily_rate: BigDecimal,
    ctc_version: i32,  // For cache invalidation
    calculated_at: DateTime<Utc>,
}
```

**Cache Invalidation:**
- Triggered on CTC record creation/update
- Immediate update (no stale data)
- TTL: 24 hours as safety net

### API Design Decisions

#### Decision: API Endpoint Structure

**Choice:** REST endpoints following existing Axum patterns

**New Routes:**

```rust
// CTC Management (HR only)
Router::new()
    .route("/ctc", get(list_ctc).post(create_ctc))
    .route("/ctc/:id", get(get_ctc).put(update_ctc))
    .route("/ctc/:id/history", get(ctc_history))
    .route("/ctc/:id/daily-rate", get(get_daily_rate))
    .layer(require_role(Role::HR))

// Project Budget (Project Managers)
Router::new()
    .route("/projects/:id/budget", get(get_budget).post(set_budget))
    .route("/projects/:id/budget/costs", get(get_project_costs))
    .layer(require_role(Role::ProjectManager))

// P&L Generation (Project Managers)
Router::new()
    .route("/projects/:id/pl", get(generate_pl))
    .route("/projects/:id/pl/monthly", get(get_monthly_pl))
    .layer(require_role(Role::ProjectManager))

// Cash Flow (Finance team)
Router::new()
    .route("/cash-flow", get(get_cash_flow_summary))
    .route("/cash-flow/entries", post(add_cash_flow_entry))
    .layer(require_role(Role::Finance))

// Enhanced Resource Assignment (Dept Heads + PMs)
Router::new()
    .route("/allocations", post(create_allocation))
    .route("/allocations/:id/cost-preview", get(get_cost_preview))
    .layer(require_role(Role::DepartmentHead, Role::ProjectManager))
```

**Error Response Format (consistent with existing):**
```json
{
  "error": "ValidationError",
  "message": "Invalid CTC data",
  "details": {
    "base_salary": ["Must be greater than 0"]
  }
}
```

### Implementation Sequence

**Week 1: Foundation**
1. Database schema (CTC tables, budget extensions)
2. Daily rate calculation service
3. Basic CTC CRUD API (HR only)

**Week 2: Cost Visibility**
1. Enhanced allocation API with cost preview
2. Budget tracking for projects
3. Department budget dashboard

**Week 3: P&L Core**
1. Project cost aggregation
2. Manual revenue entry
3. P&L generation service

**Week 4: Integration & Polish**
1. ERP integration (polling)
2. Cash flow tracking (MVP scope)
3. Performance optimization

