---
stepsCompleted: ['step-01-init', 'step-02-discovery', 'step-02b-vision', 'step-02c-executive-summary', 'step-03-success', 'step-04-journeys', 'step-05-domain', 'step-06-innovation', 'step-07-project-type', 'step-08-scoping', 'step-09-functional', 'step-10-nonfunctional', 'step-11-polish', 'step-12-complete']
status: 'complete'
completion_date: '2026-02-22'
inputDocuments:
  - '/Users/ipei/webdev/xynergy/_bmad-output/project-context.md'
  - '/Users/ipei/webdev/xynergy/docs/architecture.md'
  - '/Users/ipei/webdev/xynergy/docs/tech-stack.md'
  - '/Users/ipei/webdev/xynergy/docs/development-guide.md'
documentCounts:
  productBriefs: 0
  researchDocuments: 0
  brainstorming: 0
  projectDocs: 4
workflowType: 'prd'
classification:
  projectType: 'Web App (Full-stack Rust + Leptos + Axum + PostgreSQL)'
  domain: 'HR & Financial Management (Resource Costing & Project P&L)'
  complexity: 'High'
  projectContext: 'Brownfield'
  timeline: '4-week sprint'
  currency: 'Indonesian Rupiah (IDR)'
  compliance: 'Indonesia payroll (BPJS, THR)'
---

# Product Requirements Document - xynergy

**Author:** Putu
**Date:** 2026-02-22

## Executive Summary

Xynergy transforms resource allocation from operational scheduling into financial stewardship, enabling organizations to make cost-aware resource decisions that ensure projects deliver on time **and** on margin. This brownfield enhancement adds integrated CTC (Cost to Company) management and project P&L tracking to the existing Xynergy resource management platform, creating a unified system where allocation decisions automatically reflect financial impact.

**Target Users:**
- **HR Staff:** Manage employee CTC with Indonesia payroll compliance (BPJS Kesehatan/Ketenagakerjaan, THR), track revision history with full audit trail
- **Department Heads:** Assign team members with real-time budget impact visibility, prevent overallocation with cost-aware capacity planning
- **Project Managers:** Track project budgets, monitor resource costs against blended rates, generate monthly P&L with ERP revenue integration

**Problem Solved:**
Organizations currently suffer from "profitability blindness" — resource allocation, cost tracking, and P&L management happen in separate silos (spreadsheets, disconnected systems). This creates three critical gaps: (1) Department Heads assign resources without knowing cost impact until month-end overruns, (2) Project Managers track delivery milestones but lack true profitability visibility during projects, (3) HR maintains CTC data that never connects to operational decisions. The result is reactive financial management, budget surprises, and missed margin optimization opportunities.

**Core Value:**
Every resource allocation decision shows immediate financial impact—transforming gut-feel staffing into data-driven stewardship.

---

### What Makes This Special

**Cost-Aware Allocation (Differentiator):**
Unlike standalone resource management tools, Xynergy shows blended cost rates at the moment of assignment. Department Heads see "This assignment will consume Rp 45M of your monthly budget" before confirming—not after the damage is done. This shifts resource planning from operational scheduling to financial stewardship.

**Integrated P&L Pipeline:**
The platform creates a continuous data flow: CTC drives daily rates → Allocations drive project costs → Costs + ERP revenue drive P&L. No spreadsheet reconciliation, no version confusion, no "which rate is correct?" ambiguity. Monthly P&L generation takes seconds, not hours.

**Security-First CTC Design:**
HR maintains detailed CTC components (salary, allowances, BPJS, THR), while Department Heads and Project Managers see only blended daily rates. Strict RBAC ensures sensitive payroll data stays protected while operational insights flow appropriately.

**Indonesia-Ready Compliance:**
Built for Indonesian payroll requirements from day one—IDR currency (no decimals), BPJS health/employment insurance tracking, THR religious holiday allowance calculations. Not retrofitted, but architected for local compliance.

**The Switch Moment:**
When the first monthly P&L generates in 2 seconds instead of 2 hours of spreadsheet manipulation—and shows a project is losing margin *before* it closes.

---

## Project Classification

| Attribute | Classification |
|:----------|:---------------|
| **Project Type** | Web Application (Full-stack: Rust + Leptos + Axum + PostgreSQL) |
| **Domain** | HR & Financial Management (Resource Costing & Project P&L) |
| **Complexity** | High |
| **Project Context** | Brownfield (extending existing Xynergy platform) |
| **Timeline** | 4-week sprint (aggressive delivery) |
| **Currency** | Indonesian Rupiah (IDR) |
| **Compliance** | Indonesia payroll (BPJS Kesehatan/Ketenagakerjaan, THR) |
| **Integration** | External ERP REST API for revenue data |
| **Data Sensitivity** | High (CTC data requires strict RBAC) |

**Complexity Drivers:**
- Financial data sensitivity requiring multi-level access controls
- Indonesia-specific payroll compliance (BPJS, THR calculations)
- External ERP integration for revenue data
- Three-module integration (Assignments, CTC Management, P&L) with shared data models
- Currency handling (IDR, no decimal places)
- Audit trail requirements for CTC revisions and financial data

**Phased Delivery:**
- **P0 (Week 1-2):** Department Resource Assignment with cost visibility
- **P1 (Week 2-3):** CTC Management with Indonesian compliance
- **P2 (Week 3-4):** Project Budget & P&L with ERP integration

## Success Criteria

### User Success

**HR Staff**
| Metric | Target | Timeline |
|:-------|:-------|:---------|
| CTC Data Completeness | 100% of active employees have current CTC data | Within 30 days of launch |
| Audit Trail Coverage | 100% of CTC revisions tracked with user + timestamp + reason | From day one |
| Support Request Reduction | Zero "what's the rate for [employee]?" inquiries from PMs/Dept Heads | Within 2 weeks of launch |
| Update Efficiency | CTC updates take <2 minutes per employee (vs. spreadsheet + email) | Measured via user feedback |

**Department Heads**
| Metric | Target | Timeline |
|:-------|:-------|:---------|
| Cost-Aware Assignment Rate | Review cost impact before 90% of assignments | Measured by UI interaction logs |
| Budget Overrun Prevention | 80% reduction in department budget surprises vs. previous quarter | Compare quarterly variance reports |
| Team Utilization Optimization | Maintain 80-90% average utilization | Weekly utilization dashboard |
| Assignment Efficiency | Complete resource assignments in <5 minutes with cost visibility | Time-to-complete measurement |

**Project Managers**
| Metric | Target | Timeline |
|:-------|:-------|:---------|
| P&L Generation Speed | Generate monthly P&L in <2 seconds | System performance logging |
| Budget Tracking Adoption | 100% of active projects have budget setup within 1 week of creation | Project audit |
| Margin Visibility | Review P&L dashboard at least weekly for 90% of projects | Login analytics |
| Forecast Accuracy | Project final costs within 10% of forecast (Month 3+ data) | Post-project reconciliation |

---

### Business Success

**Month 1 (Launch)**
- All 3 modules deployed and functional
- HR completes CTC entry for 100% active employees
- Department Heads trained and actively using assignment feature
- First monthly P&L generated for all active projects

**Month 3**
- Finance team stops requesting manual CTC exports (zero requests)
- 90% of resource assignments include cost impact review
- Department budget variance reduced by 50% vs. pre-launch
- Project margin visibility influences at least 3 major project decisions

**Month 6**
- 100% of projects delivered with budget tracking enabled from day one
- CTC data used for annual budget planning (Finance validates accuracy)
- Resource allocation decisions show measurable margin improvement (5%+ average project margin)

**Compliance & Audit**
- Pass internal security audit of CTC data handling with zero critical findings (Month 2)
- BPJS/THR calculations validated by Finance—100% accuracy vs. payroll records (Month 1)
- Complete audit trail for all CTC mutations—no gaps (continuous)

**Financial Impact**
- Cost Savings: Eliminate 8-10 hours/month of spreadsheet reconciliation (Finance + PM time)
- Margin Protection: Early visibility prevents 2-3 margin-erosion events per quarter
- ROI Timeline: Break even within 2 months via prevented budget overruns

---

### Technical Success

| Category | Metric | Target |
|:---------|:-------|:-------|
| **Performance** | P&L Report Generation | <2 seconds for 12-month history, 50 resources |
| | Page Load (Department View) | <1 second for 50 employees with 6-month allocation history |
| | CTC Calculation | <100ms for 1,000 employee daily rate calculations |
| **Security** | Unauthorized CTC Access | Zero incidents (logged + alerted) |
| | Audit Log Completeness | 100% of CTC views and mutations captured |
| | Data Encryption | CTC protected with defense-in-depth (TDE + field-level encryption for sensitive CTC columns) |
| **Integration** | ERP Revenue API Uptime | 99.5% availability |
| | API Response Time | <500ms for revenue data ingestion |
| | Data Sync Frequency | Real-time (on receipt) + daily reconciliation |
| **Reliability** | System Uptime | 99.9% during business hours (8 AM - 6 PM WIB) |
| | Error Rate | <0.1% of requests (excluding validation errors) |
| | Data Consistency | Zero variance between CTC, allocations, and P&L calculations |

---

## Product Scope

### MVP - Minimum Viable Product (4-Week Delivery)

| Module | MVP Requirements |
|:-------|:-----------------|
| **Department Assignment** | Cost-aware assignment, overallocation warnings, department cost dashboard |
| **CTC Management** | Full CRUD, Indonesia compliance (BPJS/THR), revision history, blended rate calculation |
| **Project P&L** | Budget setup, resource cost tracking, non-resource cost entry, revenue API integration, monthly P&L view |

**MVP Critical Validation:**
- CTC data access restricted to HR role only—verified via penetration testing
- BPJS and THR calculations match payroll system outputs (sample validation)
- ERP revenue API handles duplicate submissions gracefully (idempotency)
- Validation rules prevent invalid CTC entries (data quality enforcement)

### Growth Features (Post-MVP, Months 2-6)

- Advanced P&L reporting (custom date ranges, export formats)
- Budget forecasting (projected costs at completion)
- Resource optimization suggestions ("swap senior for junior to save 15%")
- Email alerts for budget overruns, overallocation
- Bulk CTC import/update tools

### Vision (Future, 6+ Months)

- AI-powered cost predictions based on historical patterns
- Automatic budget rebalancing suggestions
- Multi-company/branch support
- Advanced analytics (profitability by client, project type, etc.)

## User Journeys

### Journey 1: HR Staff - CTC Management Success Path

**Meet Sari** - HR Administrator at a growing tech company with 60 employees

**Opening Scene:**
It's Monday morning and Sari is preparing for the monthly payroll cycle. Previously, she maintained CTC data in a complex Excel spreadsheet with multiple tabs—one for base salary, one for allowances, one for BPJS calculations. Every month, Project Managers emailed her asking "What's the daily rate for John?" and she had to manually calculate and respond.

**Rising Action:**
Last week, Xynergy's new CTC module went live. Sari logs in and navigates to the CTC Management section. She sees the clean interface with all employees listed. She clicks on "Ahmad Wijaya" and sees his current CTC breakdown: Base Salary (Rp 15,000,000), HRA Allowance (Rp 3,000,000), BPJS Kesehatan (company portion: Rp 480,000), BPJS Ketenagakerjaan (Rp 420,000), THR provision (Rp 1,250,000/month).

She notices Ahmad got promoted last month—their old spreadsheet didn't reflect his new salary yet. She clicks "Edit CTC" and updates his base salary to Rp 18,000,000. The system automatically recalculates everything: new daily rate is Rp 977,273 (was Rp 814,545). She adds a note: "Promotion to Senior Developer - effective 1 Feb 2026."

**Climax:**
Before saving, Sari sees a preview: "This change will affect 3 active project allocations. Total monthly cost impact: +Rp 4.36M." She confirms. The system saves the revision with her user ID, timestamp, and reason—automatically creating an audit trail that would have required manual logging in their old process.

**Resolution:**
Later that day, Dewi (a Project Manager) assigns Ahmad to a new project. When she adds him to the allocation, she sees his updated daily rate (Rp 977,273) automatically calculated and applied. No email to Sari needed. The project budget instantly reflects the accurate cost.

Sari checks her dashboard: "Zero CTC inquiries this week." She smiles—her inbox used to flood with rate requests. Now she focuses on strategic HR work instead of spreadsheet maintenance.

---

### Journey 2: Department Head - Cost-Aware Assignment

**Meet Budi** - Engineering Department Head managing 12 developers

**Opening Scene:**
Budi's department budget is Rp 450M per month. Last quarter, he consistently went 15-20% over budget because he assigned senior developers to projects without realizing the cost impact. By month-end, Finance would send angry emails about overruns. He felt blindsided—he had no visibility during the month.

**Rising Action:**
Today, a Project Manager requests two senior developers for Project Alpha. Budi opens Xynergy and navigates to "Department Resource Assignment." He sees his team dashboard: 8 developers currently allocated, 4 available. But here's the new part—next to each person's name, he sees their blended daily rate and current allocation cost.

He selects "Rina" (Rp 1,200,000/day) and "Doni" (Rp 950,000/day) for a 3-week assignment. Before confirming, he sees the cost preview: "This assignment will consume Rp 44.1M of your monthly budget (9.8%). Remaining budget after this assignment: Rp 198.9M."

**Climax:**
Budi pauses. That leaves less than half his budget for the rest of the month, and he knows two other projects are coming. He checks the "What-if" feature: if he swaps Doni (senior) with Andi (mid-level, Rp 650,000/day), the cost drops to Rp 32.55M. He messages the Project Manager: "I can give you Rina full-time and Andi instead of Doni—saves Rp 11.55M. Will that work?"

The PM agrees. Budi confirms the assignment with the adjusted resource mix. He sees his updated department budget: Rp 210.45M remaining—comfortable buffer for upcoming projects.

**Resolution:**
End of month arrives. Budi checks his department P&L: 97% budget utilization, no overruns. Finance sends their usual report—no angry emails this time. At the leadership meeting, the CTO asks how Engineering stayed on budget. Budi shows his Xynergy dashboard: "Real-time cost visibility at the point of decision. We're no longer flying blind."

---

### Journey 3: Project Manager - P&L Visibility

**Meet Dewi** - Project Manager responsible for delivering Project Alpha on time and budget

**Opening Scene:**
Dewi's projects always delivered on time, but profitability was a mystery until months after completion. She tracked tasks in Xynergy (existing resource management), but costs were in spreadsheets she never saw. Last quarter, Project Beta delivered "successfully" but the company lost money on it—senior resources were assigned too early, inflating costs. Dewi only found out during the post-mortem.

**Rising Action:**
Project Alpha kicks off. In Xynergy, Dewi navigates to "Project Budget & P&L." She sets the total budget: Rp 500M, broken down by category—HR (Rp 350M), Software (Rp 50M), Hardware (Rp 80M), Overhead (Rp 20M). She assigns her team: 3 developers, 1 QA, 1 designer. Xynergy automatically calculates resource costs based on their CTC daily rates: Rp 42M/month.

Week 2: The client adds a new feature request. Dewi needs an additional senior developer for 2 weeks. Before adding the resource, she checks the P&L dashboard: Current spend Rp 21M, forecast at completion Rp 378M—within budget. But adding the senior dev (Rp 1.2M/day × 10 days = Rp 12M) pushes the forecast to Rp 390M. Still safe, but tighter.

**Climax:**
Week 6: Monthly P&L review. Dewi clicks "Generate P&L"—2 seconds later, she sees:
- Revenue (from ERP): Rp 600M
- Resource Costs: Rp 273M
- Other Costs: Rp 127M (software licenses, cloud infrastructure)
- **Gross Profit: Rp 200M (33% margin)**

Wait—margin is 33%, but her target was 40%. She drills into resource costs: too many senior developers in weeks 1-3. She adjusts remaining allocations to use more mid-level resources. The forecast updates: projected final margin improves to 38%.

**Resolution:**
Project Alpha completes. Final P&L: 39% margin—close to target. Dewi presents to leadership: "We delivered on time, and I knew our margin status every week. When we drifted to 33%, I adjusted resource mix and recovered. No more profitable surprises after the fact."

Her next project? She starts by setting the margin target (40%) in Xynergy before making any assignments. The system helps her optimize from day one.

---

### Journey 4: Finance Team - Compliance Validation

**Meet Rudi** - Finance Controller responsible for payroll accuracy and audit compliance

**Opening Scene:**
Quarterly audit preparation. Rudi needs to validate that CTC data used for project costing matches actual payroll records. Previously, this meant requesting CTC exports from HR, comparing with payroll system, reconciling discrepancies—2-3 days of work.

**Rising Action:**
Rudi logs into Xynergy and navigates to "Finance Reports." He selects "CTC Validation Report" for Q1 2026. The system generates a reconciliation: Xynergy CTC data vs. Payroll system records. Match rate: 100%. Discrepancies: 0.

He drills into BPJS calculations for a sample of 10 employees. Xynergy shows the calculation formula: (Base Salary + Fixed Allowances) × BPJS rate. He cross-references with BPJS regulations—calculations are accurate. THR provisions are properly accrued monthly.

**Climax:**
Auditor asks: "Show me the audit trail for CTC changes." Rudi opens the "CTC Audit Log" and filters by date range. Every change is logged: User, Timestamp, Employee, Field Changed, Old Value, New Value, Reason. The auditor reviews 50 random changes—all properly documented with business reasons.

**Resolution:**
Audit completed with zero findings related to CTC handling. Rudi's quarterly report to leadership: "Xynergy's audit trail and validation features reduced our compliance preparation from 3 days to 3 hours. Data integrity is guaranteed by system design, not manual checks."

---

### Journey 5: System Administrator - Security & Access

**Meet Andi** - IT Administrator managing Xynergy system configuration

**Opening Scene:**
A new HR staff member, Maya, joined yesterday. She needs access to CTC management, but with restricted permissions—she can view and edit CTC data, but cannot export bulk data or access other departments' information.

**Rising Action:**
Andi logs into Xynergy Admin Panel. He navigates to "User Management" → "Role Configuration." He creates a new role: "HR_CTC_Editor" with specific permissions:
- ✓ View CTC data (own department only)
- ✓ Edit CTC data (own department only)
- ✗ Export CTC data
- ✗ View other departments' CTC
- ✓ View audit logs (read-only)

He assigns this role to Maya and sets her department to "HR." He configures row-level security: Maya can only see employees where department = "HR."

**Climax:**
Maya logs in and tries to access CTC data for an Engineering employee. Access denied—she only sees HR department employees. She tries to export data to Excel. Button is disabled—export permission not granted. She edits CTC for an HR employee. Success—change is logged with her user ID, timestamp, and before/after values.

**Resolution:**
Andi reviews the security audit log: "Zero unauthorized CTC access attempts this week." The system's RBAC is working as designed. He schedules quarterly access reviews—automated reports show who has access to what, making compliance audits straightforward.

---

### Journey Requirements Summary

These journeys reveal the following capability areas:

**CTC Management Module:**
- CTC CRUD with component breakdown (salary, allowances, BPJS, THR)
- Automatic daily rate calculation (Monthly CTC ÷ 22)
- Revision history with audit trail (who, when, what, why)
- Department-based access control
- Change impact preview (affected allocations, cost impact)

**Department Resource Assignment:**
- Team availability dashboard with cost visibility
- Cost-aware assignment interface (preview before confirm)
- Budget impact calculation
- What-if scenario tool (resource swap comparison)
- Overallocation warnings (>100% capacity)

**Project Budget & P&L:**
- Budget setup with category breakdown
- Automatic resource cost calculation from allocations
- Non-resource cost entry (expenses, vendor payments)
- ERP revenue API integration
- Real-time P&L dashboard
- Margin tracking and forecasting
- Historical P&L generation

**Security & Compliance:**
- Role-based access control (RBAC)
- Row-level security (department isolation)
- Audit logging (view + mutation tracking)
- CTC encryption at rest
- Permission-based feature access (export controls)

---

## Domain-Specific Requirements

### Security & Compliance (from Red Team Analysis)

**Defense in Depth:**
- Multiple authorization layers (RBAC + row-level + resource-level)
- Strict authorization checks on every request
- Generic error messages to prevent information leakage

**Audit Log Integrity:**
- Cryptographic hash chain for tamper detection
- WORM storage (Write Once Read Many)
- Append-only audit tables
- Separate database user with INSERT-only permissions

**Request Integrity:**
- Idempotency keys prevent replay attacks
- Request signing for critical operations
- CSRF tokens on all state-changing forms

**Export Security:**
- Four-eyes approval for bulk exports
- Watermark exported files with user ID + timestamp
- Rate limiting: Max 1 export per day per user
- Log all exports to SIEM

**Session Security:**
- Short-lived JWT (15 minutes)
- Rotating refresh tokens
- HttpOnly, Secure, SameSite=Strict cookies
- Immediate token revocation on role change

**Input Hardening:**
- Parameterized queries (sqlx automatic)
- Strict input validation and sanitization
- Mass assignment protection (strict DTO validation)

**Side-Channel Protection:**
- Constant-time comparison for sensitive operations
- No timing differences in authorization failures

---

### Calculation Reliability (from Failure Mode Analysis)

**BPJS/THR Calculation:**
- **CAL-001 (P0):** Automated tests for all salary tier thresholds
- **CAL-002 (P0):** PostgreSQL INTEGER for IDR amounts, reject decimal inputs
- **CAL-003 (P1):** Configurable working days (22 default, adjustable per organization)
- **CAL-004 (P1):** Clear mid-month CTC change rule (pro-rata OR effective-first-of-month)

**Formula Documentation:**
- BPJS: (Base Salary + Fixed Allowances) × BPJS rate
- THR: 1 month's salary accrued monthly
- Daily Rate: Monthly CTC ÷ Working Days

---

### Integration Resilience (from Failure Mode Analysis)

**ERP API Integration:**
- **INT-001 (P0):** Circuit breaker pattern (fail gracefully after 3 retries)
- **INT-002 (P0):** Idempotency keys prevent duplicate revenue counting
- **INT-003 (P0):** Timezone handling (UTC storage, WIB display)
- **INT-004 (P1):** Schema versioning (v1, v2) for API evolution

**Failure Handling:**
- Cached revenue data for API downtime
- Manual revenue entry fallback
- Async job queue for retry failed syncs
- Daily reconciliation: API data vs. stored data

**API Contract:**
- Authentication: [To be specified with ERP team]
- Data format: JSON
- Real-time push + daily reconciliation
- Idempotency required on all revenue submissions

---

### Cash Flow Tracking (Supervisor Addition)

**Cash Flow Components:**
- **CASH-001:** Cash In - Actual receipts (ERP) + estimated receipts (manual)
- **CASH-002:** Cash Out - Resource costs (payroll dates) + other costs (payment schedules)
- **CASH-003:** Net Cash Flow - Monthly in minus out
- **CASH-004:** Cumulative - Running cash position
- **CASH-005:** Variance - Actual vs. estimated comparison

**Cash Flow Failure Mode Preventions:**
- **CASH-006:** Track actual payroll dates separately from CTC effective dates
- **CASH-007:** Payment terms tracking (Net 30, etc.) with due dates
- **CASH-008:** Separate "invoiced" (P&L) from "received" (Cash Flow)

**Cash Flow vs. P&L Distinction:**
| Aspect | P&L | Cash Flow |
|:-------|:----|:----------|
| Timing | Revenue recognized when invoiced | Revenue when cash received |
| Costs | Accrued monthly | When actually paid |
| Purpose | Profitability analysis | Liquidity management |

---

### Access Control Hardening (from Failure Mode Analysis)

**Role Validation:**
- **ACC-001 (P0):** DB lookup on every CTC request (not just JWT decode)
- Re-validation on sensitive operations
- Short JWT expiry prevents stale permission issues

**Department Transfer Handling:**
- **ACC-002 (P1):** Effective dating on department assignments
- Historical department tracking
- Access check: "Did user have access at time of query?"

**API Endpoint Security:**
- Internal endpoints bind to localhost only
- API gateway with strict routing rules
- Automated security scanning in CI/CD
- Separate internal API service (not publicly routable)

---

### Audit & Monitoring

**Audit Log Requirements:**
- Async logging via message queue
- High-volume event handling
- Audit log database separate from main DB
- Alert if audit log lag > 5 minutes

**Clock Synchronization:**
- NTP synchronization on all servers
- Monotonic clock + wall clock in logs
- Alert on clock drift > 1 second

**Monitoring & Alerting:**
- Zero unauthorized CTC access incidents
- API health checks and error rates
- Integration monitoring (ERP sync status)
- Performance metrics (P&L generation time)

---

## Web Application Specific Requirements

### Project-Type Overview

Xynergy HR & Financial Management is a **Single Page Application (SPA)** built with Leptos (Rust/WASM) for high-interactivity financial dashboards and real-time data visualization. The application prioritizes performance and responsiveness for complex financial operations over SEO or public discoverability.

**Architecture Pattern:**
- **Main Application:** SPA (Client-Side Rendering with Hydration)
- **Authentication Pages:** SSR (Server-Side Rendering) for faster initial load
- **Reports/Exports:** SSR for print-friendly output generation

---

### Browser Support Matrix

| Browser | Minimum Version | Notes |
|:--------|:----------------|:------|
| Chrome | 90+ | Primary development target |
| Firefox | 88+ | Full feature support |
| Safari | 14+ | Including iPad for executive review |
| Edge | 90+ | Chromium-based |
| Mobile Safari | 14+ | iPad/tablet support for approvals |
| Chrome Mobile | 90+ | Tablet-optimized touch interface |

**Out of Scope:**
- Internet Explorer 11 (not supported)
- Older mobile browsers
- Screen sizes < 768px (tablet minimum, not mobile-optimized)

---

### Technical Architecture Considerations

**Frontend Stack:**
- **Framework:** Leptos 0.6 (Rust-based reactive framework)
- **Rendering:** CSR (Client-Side Rendering) + Hydration for SPA
- **Build Target:** WebAssembly (wasm32-unknown-unknown)
- **Styling:** Responsive design with touch-friendly controls for tablet use

**State Management:**
- Leptos signals for reactive state
- Resource pattern for async data fetching
- Global auth context via `provide_auth_context()`

**Real-time Updates:**
- **Technology:** Server-Sent Events (SSE)
- **Direction:** Server → Client (one-way push)
- **Use Cases:**
  - CTC updates immediately reflect in project calculations
  - Resource assignments update department dashboards instantly
  - Revenue from ERP triggers P&L refresh
  - Budget threshold alerts pushed to Project Managers

**Why SSE over WebSockets:**
- Simpler implementation for one-way data flow
- Auto-reconnection built-in
- Works over HTTP/2 without upgrade complexity
- Lower overhead for financial data streaming (no bidirectional chat needed)

---

### Performance Targets

| Metric | Target | Notes |
|:-------|:-------|:------|
| Initial Page Load | <2 seconds | SSR for auth, lazy load SPA bundle |
| Dashboard Render | <1 second | After initial load, data fetch <500ms |
| P&L Generation | <2 seconds | 12-month history, 50 resources |
| Real-time Latency | <500ms | SSE update propagation |
| Time to Interactive | <3 seconds | Full interactivity on first load |
| WASM Bundle Size | <500KB | Gzipped, code-split by route |

---

### SEO Strategy

**Status:** Not Required

**Rationale:**
- Internal business tool behind authentication
- No public marketing content
- Users access via direct login, not search engines

**Implementation:**
- All routes require authentication
- No SEO meta tags or structured data needed
- Robots.txt: `Disallow: /` (block all crawlers)

---

### Accessibility Requirements

**Standard:** WCAG 2.1 AA

**Specific Requirements:**

| Requirement | Implementation |
|:------------|:---------------|
| Keyboard Navigation | All functions accessible via keyboard (Tab, Enter, Escape) |
| Screen Reader Support | ARIA labels on all charts, tables, and financial data |
| Color Contrast | 4.5:1 minimum (critical for red/green budget indicators) |
| Focus Indicators | Visible focus rings for keyboard users |
| Chart Alternatives | All P&L graphs have equivalent data tables |
| Touch Targets | Minimum 44×44px for tablet interactions |
| Error Identification | Clear error messages with suggested fixes |

**Testing Approach:**
- Automated a11y testing in CI/CD (axe-core)
- Manual keyboard navigation testing
- Screen reader validation (NVDA/VoiceOver)

---

### Implementation Considerations

**Development Priorities:**

1. **Performance First:** WASM optimization, code splitting, lazy loading
2. **Real-time Second:** SSE infrastructure, connection management
3. **Accessibility Third:** Keyboard navigation, ARIA implementation
4. **Tablet Optimization:** Touch-friendly controls for iPad users

**Technical Constraints:**
- Rust/Axum backend with shared types (xynergy-shared crate)
- PostgreSQL for data persistence
- WebAssembly compilation target
- Modern browser APIs (no polyfills for old browsers)

**Deployment Considerations:**
- Static asset caching (WASM bundles, CSS)
- CDN for global distribution (if multi-region)
- HTTP/2 for SSE multiplexing efficiency

---

## Project Scoping & Phased Development

### MVP Strategy & Philosophy

**MVP Approach:** **Functional Core with Manual Fallbacks**

**Philosophy:** Deliver working software that solves the core problem (profitability visibility) using manual processes where automation adds convenience but not core value. A working P&L with manual entry is better than a broken automated system.

**Resource Requirements:**
- 1 Backend Developer (Rust/Axum/sqlx)
- 1 Frontend Developer (Leptos/Rust/WASM)
- 1 Full-stack or floating support
- Product Owner for weekly demos and feedback

---

### MVP Feature Set (Phase 1) - 4 Weeks

**Week 1: Foundation (CTC Core)**

| Feature | Description | Success Criteria |
|:--------|:------------|:-----------------|
| Database Schema | CTC, allocations, projects, budgets tables | All entities modeled with relationships |
| CTC CRUD | Create, read, update CTC with components | Full component breakdown (salary, allowances, BPJS, THR) |
| Daily Rate Calc | Monthly CTC ÷ 22 working days | Accurate to nearest IDR |
| RBAC Setup | HR, Dept Head, PM roles configured | Role-based access working |
| Basic Audit Log | Track CTC changes (user, timestamp, values) | All mutations captured |

**Week 2: Cost Visibility (Assignments)**

| Feature | Description | Success Criteria |
|:--------|:------------|:-----------------|
| Department Team View | See all team members with rates | Blended rates visible (not CTC components) |
| Resource Assignment | Assign to projects with cost preview | Cost impact shown before confirm |
| Overallocation Warnings | Alert when >100% capacity | Warning displays, assignment still possible |
| Dept Budget Dashboard | Monthly budget tracking | Real-time spend vs. budget |
| Blended Rate Calc | Daily rate visible to PMs | No CTC component exposure |

**Week 3: P&L Core (Project Financials)**

| Feature | Description | Success Criteria |
|:--------|:------------|:-----------------|
| Project Budget Setup | Budget by category (HR, software, etc.) | Categories configurable |
| Resource Cost Tracking | Allocation × daily rate | Automatic calculation |
| Non-Resource Costs | Manual entry (expenses, vendors) | Free-form entry with category |
| Manual Revenue Entry | PM enters monthly revenue | Simple form, validated |
| Monthly P&L Generation | Revenue - Costs = Profit | <5 second generation |

**Week 4: Cash Flow MVP + Polish**

| Feature | Description | Success Criteria |
|:--------|:------------|:-----------------|
| Cash In Entry | Manual actual receipts | Date, amount, source |
| Cash Out Entry | Manual payment schedules | Date, amount, vendor |
| Net Cash Flow Calc | Monthly in - out | Automatic calculation |
| Cumulative Cash Position | Running cash chart | Visual trend display |
| Polling Updates | 30-second dashboard refresh | Data freshness maintained |
| Performance Optimization | Load testing and tuning | <3 second page loads |
| Documentation + UAT | User guides, acceptance testing | Supervisor sign-off |

---

### Post-MVP Features

**Phase 2: Automation & Real-time (Month 2)**

| Feature | Value | Complexity |
|:--------|:------|:-----------|
| ERP API Integration | Automatic revenue sync | High |
| SSE Real-time Updates | Instant dashboard refresh | Medium |
| Cash Flow Variance Analysis | Actual vs. estimated | Medium |
| Advanced P&L Reporting | Exports, custom ranges | Low |
| Email Alerts | Budget/cash flow warnings | Low |

**Phase 3: Intelligence & Scale (Month 3)**

| Feature | Value | Complexity |
|:--------|:------|:-----------|
| Budget Forecasting | Project at completion | Medium |
| Audit Log Hash Chain | Tamper-proof logs | Medium |
| WORM Storage | Compliance-grade audit | Medium |
| Bulk CTC Import | HR efficiency | Low |
| Resource Optimization | "Swap X for Y to save Z" | High |

---

### Risk Mitigation Strategy

**Technical Risks:**

| Risk | Likelihood | Impact | Mitigation |
|:-----|:-----------|:-------|:-----------|
| BPJS calculation errors | Medium | High | Use lookup tables, not formulas; validate against payroll |
| Performance issues | Medium | Medium | Load test Week 3, optimize Week 4; defer SSE to Post-MVP |
| Cash Flow scope creep | High | High | Strict "manual entry only" rule; no automation in MVP |
| Week 1 delay (CTC) | Low | Critical | CTC is foundational—if delayed, compress Week 2 features |

**Market/User Risks:**

| Risk | Likelihood | Impact | Mitigation |
|:-----|:-----------|:-------|:-----------|
| User adoption | Medium | High | Weekly demos to supervisor; iterate on feedback |
| Manual entry resistance | Medium | Medium | Emphasize "working now" vs "automated later" |
| Data quality issues | Medium | High | Validation rules, audit logs, manual reconciliation |

**Resource Risks:**

| Risk | Likelihood | Impact | Mitigation |
|:-----|:-----------|:-------|:-----------|
| Developer availability | Low | Critical | 2 developers minimum; no single points of failure |
| Scope additions | High | Critical | Change control process; supervisor approval required |

---

### Revised Success Criteria

| Metric | MVP Target (4 Weeks) | Post-MVP Target (Month 3) |
|:-------|:---------------------|:--------------------------|
| CTC Data Completeness | 100% within 30 days | Same |
| Cost-Aware Assignments | 80% with manual process | 90% with automation |
| P&L Generation Speed | <5 seconds (manual entry) | <2 seconds (automated) |
| Cash Flow Visibility | Basic in/out tracking | Full variance + forecasting |
| System Uptime | 99% | 99.9% |
| Manual Revenue Entry | Required | Optional (ERP auto) |
| Real-time Updates | 30-second polling | SSE push |

---

**Key Scope Boundaries (What We're NOT Building in MVP):**

❌ ERP API integration (manual revenue entry instead)  
❌ SSE real-time updates (polling instead)  
❌ Cash Flow variance analysis (basic tracking only)  
❌ Budget forecasting (historical only)  
❌ Email alerts (dashboard indicators only)  
❌ Audit log hash chain (basic logging only)  
❌ Bulk import/export (individual CRUD only)  

**What We ARE Building:**

✅ Working CTC management with Indonesia compliance  
✅ Cost-aware resource assignment  
✅ Manual P&L generation  
✅ Basic Cash Flow tracking  
✅ Secure RBAC and audit logging  
✅ Responsive SPA with polling updates

---

## Functional Requirements

### 1. CTC Management

- **FR1:** HR Staff can create employee CTC records with component breakdown (base salary, allowances, BPJS, THR)
- **FR2:** HR Staff can view CTC details for employees in their department
- **FR3:** HR Staff can update CTC components with revision tracking
- **FR4:** HR Staff can view CTC revision history with audit trail (who, when, what changed)
- **FR5:** System automatically calculates daily rate from monthly CTC (divided by configurable working days)
- **FR6:** System enforces Indonesia payroll compliance (BPJS Kesehatan, BPJS Ketenagakerjaan, THR calculations)
- **FR7:** System validates CTC data integrity (no negative values, valid component combinations)
- **FR8:** System prevents decimal places in IDR amounts (whole numbers only)

### 2. Resource Assignment

- **FR9:** Department Heads can view their team members with blended daily rates
- **FR10:** Department Heads can assign team members to projects with date ranges
- **FR11:** Department Heads can see cost impact preview before confirming assignments
- **FR12:** Department Heads receive overallocation warnings when team member exceeds 100% capacity
- **FR13:** Department Heads can view department budget utilization in real-time
- **FR14:** Project Managers can view assigned resources with blended rates (no CTC component details)
- **FR15:** System prevents assignment of resources without CTC data

### 3. Project Budget Management

- **FR16:** Project Managers can create project budgets with category breakdown (HR, software, hardware, overhead)
- **FR17:** Project Managers can set total budget amount per category
- **FR18:** Project Managers can enter non-resource costs (expenses, vendor payments) with categories
- **FR19:** System automatically calculates resource costs from allocations × daily rates
- **FR20:** Project Managers can view current spend vs. budget by category
- **FR21:** Project Managers can view total project cost (resources + non-resource costs)
- **FR22:** System prevents budget overruns (configurable: warn or block)

### 4. P&L Generation

- **FR23:** Project Managers can manually enter monthly revenue per project
- **FR24:** System calculates monthly P&L (Revenue - Total Costs = Profit)
- **FR25:** System calculates gross margin percentage (Profit ÷ Revenue)
- **FR26:** Project Managers can view historical P&L by month
- **FR27:** System forecasts project profitability at completion based on current burn rate
- **FR28:** Project Managers can set target margin per project
- **FR29:** System alerts when current margin deviates from target (configurable threshold)

### 5. Cash Flow Tracking

- **FR30:** Finance Team can manually enter cash receipts (actual revenue received)
- **FR31:** Finance Team can manually enter cash outflows (payroll dates, payment schedules)
- **FR32:** System calculates net cash flow (cash in - cash out) per month
- **FR33:** System displays cumulative cash position over time
- **FR34:** Finance Team can view cash flow by month with in/out breakdown
- **FR35:** System differentiates between invoiced revenue (P&L) and received cash (Cash Flow)

### 6. Access Control & Security

- **FR36:** System supports role-based access control (HR, Department Head, Project Manager, Finance, Admin)
- **FR37:** HR role can view and edit CTC details
- **FR38:** Department Head role can view blended rates and assign resources (own department only) — enforced via `departments.head_id` database relationship, not just role string
- **FR39:** Project Manager role can view blended rates and manage project budgets (assigned projects only) — enforced via `projects.project_manager_id` database relationship
- **FR40:** Finance role can view all financial data for compliance and reporting
- **FR41:** System enforces row-level security (users only see data for their department)
- **FR42:** System logs all CTC views and mutations with user, timestamp, and action
- **FR43:** System supports manual bulk export with approval workflow (four-eyes principle)
- **FR44:** System prevents unauthorized CTC data access through authentication and authorization checks

### 7. Dashboard & Reporting

- **FR45:** Users can view personalized dashboard based on role
- **FR46:** HR Staff can view CTC completeness status across all employees
- **FR47:** Department Heads can view team utilization rates and budget status
- **FR48:** Project Managers can view project health dashboard (budget, P&L, margin)
- **FR49:** Finance Team can view CTC validation reports comparing system data to payroll records
- **FR50:** System supports polling-based dashboard updates (30-second refresh)
- **FR51:** Users can manually refresh dashboard data on demand

### 8. Audit & Compliance

- **FR52:** System maintains complete audit trail of all CTC changes
- **FR53:** System maintains complete audit trail of all resource assignments
- **FR54:** System maintains complete audit trail of all budget modifications
- **FR55:** Audit logs include: user ID, timestamp, action type, before values, after values, reason
- **FR56:** Finance Team can generate audit reports for compliance verification
- **FR57:** System supports basic audit log viewing and filtering

### 9. Capacity & Budget Reporting Enhancements (Post-Implementation)

- **FR58:** Capacity report uses weighted working-day formula: `Σ(allocated_weekdays × allocation_pct / 100) / working_days_in_month × 100%` — not raw percentage sum
- **FR59:** All team page sections (team members, capacity report, budget utilization) are consistently scoped to the user's department via `resolve_department_id()` helper
- **FR60:** RBAC authorization uses relationship-based access control: `departments.head_id` identifies department heads, `projects.project_manager_id` identifies project managers — database lookups validate access, not just JWT role string
- **FR61:** JWT Claims include `department_id` for efficient department scoping without additional DB lookups on every request
---

## Non-Functional Requirements

### Performance

- **NFR1:** P&L report generation completes within <2 seconds for 12-month history with 50 resources
- **NFR2:** Dashboard page load completes within <1 second after initial data fetch (<500ms data retrieval)
- **NFR3:** CTC calculation for 1,000 employees completes within <100ms
- **NFR4:** Initial page load (SPA bundle) completes within <2 seconds with SSR authentication page
- **NFR5:** Time to interactive (full UI responsiveness) is <3 seconds on first load
- **NFR6:** WASM bundle size is <500KB gzipped with code-splitting by route
- **NFR7:** Dashboard data polling updates display within 30 seconds of server data changes
- **NFR8:** API response time for standard CRUD operations is <200ms (95th percentile)

### Security

- **NFR9:** CTC sensitive data must be encrypted with defense-in-depth: storage encryption at rest (TDE/disk) and application-controlled field-level encryption for salary/allowance/BPJS/THR components so direct DB table reads do not reveal plaintext.
- **NFR10:** All data in transit encrypted using TLS 1.3
- **NFR11:** JWT tokens expire after 15 minutes; refresh tokens rotate on each use
- **NFR12:** Session cookies configured as HttpOnly, Secure, SameSite=Strict
- **NFR13:** Zero unauthorized CTC data access incidents (monitored and alerted)
- **NFR14:** Audit logs capture 100% of CTC views and mutations with no gaps
- **NFR15:** Role re-validation performed on database for every CTC request (not cached)
- **NFR16:** Row-level security enforced at database level (users cannot bypass via SQL)
- **NFR17:** Password complexity enforced (min 12 chars, mixed case, numbers, symbols)
- **NFR18:** Account lockout after 5 failed login attempts (30-minute lockout)
- **NFR19:** Bulk export requires four-eyes approval workflow (second authorization)
- **NFR20:** All exports watermarked with user ID and timestamp

### Accessibility

- **NFR21:** Web interface conforms to WCAG 2.1 AA standards
- **NFR22:** All functionality accessible via keyboard (Tab navigation, Enter/Space activation)
- **NFR23:** Color contrast ratio of 4.5:1 minimum for all text and UI elements
- **NFR24:** Focus indicators visible and clear for keyboard users
- **NFR25:** All charts and graphs have equivalent data table alternatives
- **NFR26:** ARIA labels provided for all interactive elements, charts, and data tables
- **NFR27:** Touch targets minimum 44×44 pixels for tablet interactions
- **NFR28:** Screen reader compatible (tested with NVDA and VoiceOver)

### Integration

- **NFR29:** ERP revenue API integration maintains 99.5% uptime availability
- **NFR30:** ERP API response time <500ms for revenue data ingestion (95th percentile)
- **NFR31:** Idempotent revenue submission handling prevents duplicate counting
- **NFR32:** Circuit breaker pattern prevents cascade failures (3 retry attempts, then graceful degradation)
- **NFR33:** Daily reconciliation process validates ERP data sync accuracy
- **NFR34:** API schema versioning supports backward compatibility (v1, v2)
- **NFR35:** Manual revenue entry fallback available when ERP API unavailable

### Scalability

- **NFR36:** System supports initial user base of 100 concurrent users
- **NFR37:** Database design supports 10x data growth (employees, projects, allocations) without schema changes
- **NFR38:** Horizontal scaling possible via load balancer (stateless application servers)
- **NFR39:** CDN support for static assets (WASM bundles, CSS) for global deployment

### Reliability

- **NFR40:** System uptime 99.9% during business hours (8 AM - 6 PM WIB)
- **NFR41:** Error rate <0.1% of requests (excluding validation errors)
- **NFR42:** Zero data loss guarantee for committed transactions
- **NFR43:** Automated daily database backups with 7-day retention
- **NFR44:** Point-in-time recovery capability (24-hour recovery window)
- **NFR45:** Graceful degradation when external services (ERP) unavailable
