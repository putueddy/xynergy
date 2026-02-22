---
stepsCompleted: ['step-01-validate-prerequisites', 'step-02-design-epics', 'step-03-create-stories', 'step-04-final-validation']
status: 'complete'
completion_date: '2026-02-22'
inputDocuments: [
  '/Users/ipei/webdev/xynergy/_bmad-output/planning-artifacts/prd.md',
  '/Users/ipei/webdev/xynergy/_bmad-output/planning-artifacts/architecture.md',
  '/Users/ipei/webdev/xynergy/_bmad-output/planning-artifacts/ux-design-specification.md'
]
epicCount: 6
storyCount: 30
frCoverage: 57
nfrCoverage: 43
---

# xynergy - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for xynergy, decomposing the requirements from the PRD, UX Design, and Architecture requirements into implementable stories.

## Requirements Inventory

### Functional Requirements

- **FR1:** HR Staff can create employee CTC records with component breakdown (base salary, allowances, BPJS, THR)
- **FR2:** HR Staff can view CTC details for employees in their department
- **FR3:** HR Staff can update CTC components with revision tracking
- **FR4:** HR Staff can view CTC revision history with audit trail (who, when, what changed)
- **FR5:** System automatically calculates daily rate from monthly CTC (divided by configurable working days)
- **FR6:** System enforces Indonesia payroll compliance (BPJS Kesehatan, BPJS Ketenagakerjaan, THR calculations)
- **FR7:** System validates CTC data integrity (no negative values, valid component combinations)
- **FR8:** System prevents decimal places in IDR amounts (whole numbers only)
- **FR9:** Department Heads can view their team members with blended daily rates
- **FR10:** Department Heads can assign team members to projects with date ranges
- **FR11:** Department Heads can see cost impact preview before confirming assignments
- **FR12:** Department Heads receive overallocation warnings when team member exceeds 100% capacity
- **FR13:** Department Heads can view department budget utilization in real-time
- **FR14:** Project Managers can view assigned resources with blended rates (no CTC component details)
- **FR15:** System prevents assignment of resources without CTC data
- **FR16:** Project Managers can create project budgets with category breakdown (HR, software, hardware, overhead)
- **FR17:** Project Managers can set total budget amount per category
- **FR18:** Project Managers can enter non-resource costs (expenses, vendor payments) with categories
- **FR19:** System automatically calculates resource costs from allocations × daily rates
- **FR20:** Project Managers can view current spend vs. budget by category
- **FR21:** Project Managers can view total project cost (resources + non-resource costs)
- **FR22:** System prevents budget overruns (configurable: warn or block)
- **FR23:** Project Managers can manually enter monthly revenue per project
- **FR24:** System calculates monthly P&L (Revenue - Total Costs = Profit)
- **FR25:** System calculates gross margin percentage (Profit ÷ Revenue)
- **FR26:** Project Managers can view historical P&L by month
- **FR27:** System forecasts project profitability at completion based on current burn rate
- **FR28:** Project Managers can set target margin per project
- **FR29:** System alerts when current margin deviates from target (configurable threshold)
- **FR30:** Finance Team can manually enter cash receipts (actual revenue received)
- **FR31:** Finance Team can manually enter cash outflows (payroll dates, payment schedules)
- **FR32:** System calculates net cash flow (cash in - cash out) per month
- **FR33:** System displays cumulative cash position over time
- **FR34:** Finance Team can view cash flow by month with in/out breakdown
- **FR35:** System differentiates between invoiced revenue (P&L) and received cash (Cash Flow)
- **FR36:** System supports role-based access control (HR, Department Head, Project Manager, Finance, Admin)
- **FR37:** HR role can view and edit CTC details
- **FR38:** Department Head role can view blended rates and assign resources (own department only)
- **FR39:** Project Manager role can view blended rates and manage project budgets (assigned projects only)
- **FR40:** Finance role can view all financial data for compliance and reporting
- **FR41:** System enforces row-level security (users only see data for their department)
- **FR42:** System logs all CTC views and mutations with user, timestamp, and action
- **FR43:** System supports manual bulk export with approval workflow (four-eyes principle)
- **FR44:** System prevents unauthorized CTC data access through authentication and authorization checks
- **FR45:** Users can view personalized dashboard based on role
- **FR46:** HR Staff can view CTC completeness status across all employees
- **FR47:** Department Heads can view team utilization rates and budget status
- **FR48:** Project Managers can view project health dashboard (budget, P&L, margin)
- **FR49:** Finance Team can view CTC validation reports comparing system data to payroll records
- **FR50:** System supports polling-based dashboard updates (30-second refresh)
- **FR51:** Users can manually refresh dashboard data on demand
- **FR52:** System maintains complete audit trail of all CTC changes
- **FR53:** System maintains complete audit trail of all resource assignments
- **FR54:** System maintains complete audit trail of all budget modifications
- **FR55:** Audit logs include: user ID, timestamp, action type, before values, after values, reason
- **FR56:** Finance Team can generate audit reports for compliance verification
- **FR57:** System supports basic audit log viewing and filtering

### Non-Functional Requirements

- **NFR1:** P&L report generation completes within <2 seconds for 12-month history with 50 resources
- **NFR2:** Dashboard page load completes within <1 second after initial data fetch (<500ms data retrieval)
- **NFR3:** CTC calculation for 1,000 employees completes within <100ms
- **NFR4:** Initial page load (SPA bundle) completes within <2 seconds with SSR authentication page
- **NFR5:** Time to interactive (full UI responsiveness) is <3 seconds on first load
- **NFR6:** WASM bundle size is <500KB gzipped with code-splitting by route
- **NFR7:** Dashboard data polling updates display within 30 seconds of server data changes
- **NFR8:** API response time for standard CRUD operations is <200ms (95th percentile)
- **NFR9:** All CTC data encrypted at rest using PostgreSQL TDE
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
- **NFR21:** Web interface conforms to WCAG 2.1 AA standards
- **NFR22:** All functionality accessible via keyboard (Tab navigation, Enter/Space activation)
- **NFR23:** Color contrast ratio of 4.5:1 minimum for all text and UI elements
- **NFR24:** Focus indicators visible and clear for keyboard users
- **NFR25:** All charts and graphs have equivalent data table alternatives
- **NFR26:** ARIA labels provided for all interactive elements, charts, and data tables
- **NFR27:** Touch targets minimum 44×44 pixels for tablet interactions
- **NFR28:** Screen reader compatible (tested with NVDA and VoiceOver)
- **NFR29:** ERP revenue API integration maintains 99.5% uptime availability
- **NFR30:** ERP API response time <500ms for revenue data ingestion (95th percentile)
- **NFR31:** Idempotent revenue submission handling prevents duplicate counting
- **NFR32:** Circuit breaker pattern prevents cascade failures (3 retry attempts, then graceful degradation)
- **NFR33:** Daily reconciliation process validates ERP data sync accuracy
- **NFR34:** API schema versioning supports backward compatibility (v1, v2)
- **NFR35:** Manual revenue entry fallback available when ERP API unavailable
- **NFR36:** System supports initial user base of 100 concurrent users
- **NFR37:** Database design supports 10x data growth (employees, projects, allocations) without schema changes
- **NFR38:** Horizontal scaling possible via load balancer (stateless application servers)
- **NFR39:** CDN support for static assets (WASM bundles, CSS) for global deployment
- **NFR40:** System uptime 99.9% during business hours (8 AM - 6 PM WIB)
- **NFR41:** Error rate <0.1% of requests (excluding validation errors)
- **NFR42:** Zero data loss guarantee for committed transactions
- **NFR43:** Automated daily database backups with 7-day retention

### Additional Requirements

**From Architecture Document:**
- Brownfield extension - integrate with existing Xynergy Leptos/Axum/PostgreSQL stack
- Rust 1.75+ with Leptos 0.6 (frontend), Axum 0.7 (backend)
- CTC table schema with revision history support
- PostgreSQL TDE for encryption at rest (MVP), HashiCorp Vault for production
- Hash chain audit logs with tamper detection (SHA-256)
- Daily ERP polling with manual fallback for revenue sync
- REST API design following existing Axum patterns
- Row-level security at database level via PostgreSQL RLS

**From UX Design Document:**
- Tailwind CSS 3.4 with custom Leptos components
- Professional blue/gray color palette (builds trust for financial data)
- Inter + JetBrains Mono font pairing
- Design direction: "Stripe Financial" (clean, spacious, data-dense when needed)
- Core experience: Cost-aware resource assignment with instant visibility
- Responsive design supporting desktop (primary) and tablet
- WCAG 2.1 AA accessibility compliance

### FR Coverage Map

| FR | Epic | Description |
|:---|:-----|:------------|
| FR1-8 | Epic 2 | CTC Management - Employee cost records with Indonesia compliance |
| FR9-13 | Epic 3 | Department Resource Assignment - Team assignment with cost visibility |
| FR14-15 | Epic 3 | (Cross-cutting) Resource assignment validation |
| FR16-22 | Epic 4 | Project Budget & P&L - Budget setup and tracking |
| FR23-29 | Epic 4 | P&L Reporting - Revenue, margin, forecasting |
| FR30-35 | Epic 5 | Cash Flow Tracking - Cash in/out management |
| FR36-44 | Epic 1 | Security & RBAC Foundation - Authentication and authorization |
| FR45-51 | Epic 6 | Dashboard & Reporting - Role-based dashboards |
| FR52-57 | Epic 1 | (Cross-cutting) Audit Logging - Security requirement |

## Epic List

### Epic 1: Security & RBAC Foundation
**Users can securely authenticate and access only authorized data based on their role and department.**

**FRs covered:** FR36-44, FR52-57 (Security foundation + audit logging)

**User Outcome:**
- Users authenticate securely with MFA-ready architecture
- Role-based access ensures HR sees CTC, Dept Heads see blended rates only
- Row-level security enforces department isolation
- Every data access and mutation is logged for audit

**NFRs addressed:** NFR9-20, NFR40-43 (Security, encryption, audit, reliability)

---

### Epic 2: CTC Management
**HR Staff can manage employee CTC with Indonesia payroll compliance and full audit trail.**

**FRs covered:** FR1-8 (CTC CRUD, BPJS/THR calculations, daily rates)

**User Outcome:**
- HR creates CTC records with component breakdown (salary, allowances, BPJS, THR)
- System auto-calculates BPJS Kesehatan/Ketenagakerjaan per regulations
- Daily rates calculated automatically (monthly CTC ÷ working days)
- Complete revision history with who/what/when/why for every change

**NFRs addressed:** NFR3, NFR14, NFR15, NFR16 (Performance, audit, security)

---

### Epic 3: Department Resource Assignment
**Department Heads can assign team members to projects with real-time cost impact visibility.**

**FRs covered:** FR9-15 (Team view, assignments, cost preview, overallocation warnings)

**User Outcome:**
- View team with blended daily rates (no sensitive CTC components)
- Assign resources to projects with date ranges
- See cost impact BEFORE confirming assignment
- Receive warnings when assignments exceed 100% capacity
- View department budget utilization in real-time

**NFRs addressed:** NFR2, NFR7, NFR8 (Dashboard performance, real-time updates)

---

### Epic 4: Project Budget & P&L
**Project Managers can track project budgets, costs, and profitability with ERP integration.**

**FRs covered:** FR16-29 (Budget setup, cost tracking, P&L, forecasting)

**User Outcome:**
- Create project budgets with category breakdown (HR, software, hardware, overhead)
- Automatic resource cost calculation from allocations
- Manual revenue entry with ERP integration for actuals
- Real-time P&L dashboard with margin tracking
- Forecast profitability at completion based on burn rate

**NFRs addressed:** NFR1, NFR29-35 (P&L performance, ERP integration)

---

### Epic 5: Cash Flow & Compliance
**Finance Team can track cash flow and generate compliance reports with audit validation.**

**FRs covered:** FR30-35, FR49, FR56-57 (Cash flow tracking, validation reports, audit)

**User Outcome:**
- Enter cash receipts and outflows with payment schedules
- View net cash flow and cumulative cash position
- Generate CTC validation reports for payroll reconciliation
- Access complete audit trails for compliance verification
- Export data with four-eyes approval workflow

**NFRs addressed:** NFR19-20, NFR42-43 (Export security, data integrity)

---

### Epic 6: Dashboard & Reporting
**All users can view role-appropriate dashboards with real-time project and financial health.**

**FRs covered:** FR45-51 (Role-based dashboards, polling updates)

**User Outcome:**
- HR sees CTC completeness status across employees
- Department Heads see team utilization and budget status
- Project Managers see project health (budget, P&L, margin)
- Finance sees validation reports and compliance status
- All dashboards update in real-time (30-second polling)

**NFRs addressed:** NFR2, NFR4-7, NFR21-28 (Performance, accessibility)

---

**Total: 6 Epics covering 57 FRs and 43 NFRs**

## Epic 1: Security & RBAC Foundation

{{epic_goal_1}}

### Story 1.1: User Authentication System

As a **System User**,
I want **to authenticate securely with username/password**,
So that **I can access the Xynergy system with confidence my credentials are protected**.

**Acceptance Criteria:**

**Given** I am on the login page
**When** I enter valid username and password
**Then** I am authenticated and redirected to my role-based dashboard
**And** a JWT access token (15-min expiry) and rotating refresh token are issued

**Given** I enter invalid credentials
**When** I submit the login form
**Then** I receive a generic error message (no information leakage)
**And** failed attempts are counted toward account lockout

**Given** I have failed login 5 times
**When** I attempt another login
**Then** my account is locked for 30 minutes
**And** an alert is logged for security monitoring

---

### Story 1.2: Role-Based Access Control

As a **System Administrator**,
I want **to assign roles to users (HR, Department Head, Project Manager, Finance, Admin)**,
So that **users can only access features appropriate to their responsibilities**.

**Acceptance Criteria:**

**Given** I am logged in as Admin
**When** I navigate to User Management
**Then** I can view all users and their current roles

**Given** I select a user to edit
**When** I change their role to "HR"
**Then** the user gains access to CTC management features
**And** the change is logged with timestamp and admin ID

**Given** a user has the "Department Head" role
**When** they attempt to access CTC component details
**Then** access is denied with "Insufficient permissions" message
**And** the access attempt is logged for audit

**Given** a user has the "Project Manager" role  
**When** they access resource assignment
**Then** they see only blended rates, never CTC components
**And** they can only view projects assigned to them

---

### Story 1.3: Row-Level Security

As a **Department Head**,
I want **to only see employees and data from my own department**,
So that **I cannot access sensitive information from other departments**.

**Acceptance Criteria:**

**Given** I am logged in as Department Head for "Engineering"
**When** I view the team list
**Then** I only see employees where department = "Engineering"
**And** employees from "Sales" or "HR" are not visible

**Given** I attempt to access a CTC record via direct URL manipulation
**When** the employee belongs to a different department
**Then** access is denied at the database level (PostgreSQL RLS)
**And** the unauthorized access attempt is logged

**Given** I am an HR staff member
**When** I view CTC data
**Then** I can see employees from all departments
**And** this access is logged for audit purposes

---

### Story 1.4: Audit Logging System

As a **Finance Controller**,
I want **every CTC view and mutation to be logged with complete details**,
So that **I can demonstrate compliance during audits and detect unauthorized access**.

**Acceptance Criteria:**

**Given** a user views any CTC record
**When** the view action completes
**Then** an audit log entry is created with: user ID, timestamp, employee ID, action="VIEW"

**Given** a user modifies a CTC record
**When** the change is saved
**Then** an audit log entry includes: before values, after values, change reason
**And** the log entry hash is computed for tamper detection

**Given** I navigate to Audit Reports
**When** I filter by date range and action type
**Then** I see all matching audit entries
**And** I can export the report (subject to four-eyes approval)

**Given** an audit log entry exists
**When** I verify the hash chain
**Then** any tampering with the log is immediately detectable

---

## Epic 2: CTC Management

{{epic_goal_2}}

### Story 2.1: Employee CTC Record Creation

As an **HR Staff member**,
I want **to create employee CTC records with full component breakdown**,
So that **the system has accurate cost data for project calculations**.

**Acceptance Criteria:**

**Given** I am logged in as HR
**When** I navigate to CTC Management → Add Employee
**Then** I see a form with fields: Employee ID, Name, Department, Base Salary, Allowances

**Given** I enter CTC components
**When** I input values in IDR
**Then** the system rejects any decimal places (whole numbers only)
**And** displays validation errors for negative values

**Given** I enter base salary and allowances
**When** I click "Calculate BPJS"
**Then** the system calculates BPJS Kesehatan (4% employer, 1% employee) and BPJS Ketenagakerjaan (0.24-1.74% based on tier)
**And** displays the calculated amounts for confirmation

**Given** I complete the CTC form
**When** I click "Save"
**Then** the record is created with status="Active"
**And** the daily rate is automatically calculated (monthly CTC ÷ 22 working days)
**And** an audit log entry is created with all values

**Given** I am logged in as a non-HR role
**When** I view top navigation or attempt to open `/ctc`
**Then** I do not see the CTC menu item
**And** direct URL access to `/ctc` is blocked by RBAC (forbidden UX state and redirect to dashboard)

---

### Story 2.2: CTC Revision Management

As an **HR Staff member**,
I want **to update CTC components with full revision tracking**,
So that **salary changes are documented with who made the change and why**.

**Acceptance Criteria:**

**Given** an employee has an existing CTC record
**When** I click "Edit CTC"
**Then** I see the current values and a "Change Reason" field (required)

**Given** I modify the base salary
**When** I enter a new value and provide a reason
**Then** the system creates a new revision record
**And** preserves the previous version in history

**Given** I view CTC details
**When** I click "View History"
**Then** I see a chronological list of all changes with: date, user, field changed, old value, new value, reason

**Given** a mid-month CTC change
**When** I apply the update
**Then** the system applies pro-rata calculation for the current month (configurable: pro-rata OR effective-first-of-month)

---

### Story 2.3: THR Management

As an **HR Staff member**,
I want **the system to track THR (Tunjangan Hari Raya) religious holiday allowance**,
So that **compliance with Indonesian labor law is maintained**.

**Acceptance Criteria:**

**Given** I create or edit a CTC record
**When** I navigate to the THR section
**Then** I can set THR eligibility and calculation basis

**Given** THR is configured
**When** the monthly accrual runs
**Then** the system accrues 1/12 of annual THR entitlement
**And** displays accrued amount in the CTC summary

**Given** it is THR payment month (typically before Eid)
**When** I generate the THR report
**Then** the system shows total THR due per employee
**And** includes the calculation basis (1 month salary or prorated)

---

### Story 2.4: CTC Validation & Compliance

As an **HR Manager**,
I want **the system to validate CTC data integrity automatically**,
So that **payroll errors are caught before they impact project costing**.

**Acceptance Criteria:**

**Given** I enter CTC data with invalid combinations
**When** I attempt to save
**Then** the system displays validation errors (e.g., allowances > base salary, negative values)

**Given** I view the CTC completeness dashboard
**When** I filter by department
**Then** I see which employees have complete CTC data and which are missing
**And** the system prevents resource assignment for employees without CTC

**Given** I run the compliance report
**When** I select a date range
**Then** the system validates BPJS calculations against regulations
**And** flags any discrepancies for review

---

## Epic 3: Department Resource Assignment

{{epic_goal_3}}

### Story 3.1: Team View with Blended Rates

As a **Department Head**,
I want **to see my team members with their blended daily rates**,
So that **I can make informed assignment decisions without seeing sensitive CTC components**.

**Acceptance Criteria:**

**Given** I am logged in as Department Head
**When** I navigate to "My Team"
**Then** I see a list of employees in my department with: Name, Role, Current Allocations, Blended Daily Rate

**Given** I view the team list
**When** I look at an employee's daily rate
**Then** I see the blended rate (calculated from CTC)
**And** I do NOT see base salary, allowances, or BPJS components

**Given** an employee has no CTC data
**When** I view the team list
**Then** they are marked with "CTC Missing" status
**And** I cannot assign them to projects until CTC is complete

---

### Story 3.2: Resource Assignment Interface

As a **Department Head**,
I want **to assign team members to projects with date ranges and allocation percentages**,
So that **project staffing is tracked in the system**.

**Acceptance Criteria:**

**Given** I select an employee from my team
**When** I click "Assign to Project"
**Then** I see a form with: Project dropdown, Start Date, End Date, Allocation % (0-100)

**Given** I enter assignment details
**When** I select a project I have access to
**Then** the project dropdown shows only projects where I have assignment rights

**Given** I set an allocation percentage
**When** I submit the assignment
**Then** the system validates the employee has capacity (total allocations + new ≤ 100%)

**Given** an assignment overlaps with existing allocations
**When** I review the timeline view
**Then** I see a Gantt-style visualization of all assignments for that employee

---

### Story 3.3: Cost Impact Preview

As a **Department Head**,
I want **to see the cost impact of an assignment BEFORE confirming**,
So that **I can make cost-aware staffing decisions**.

**Acceptance Criteria:**

**Given** I am creating a new assignment
**When** I enter the date range and allocation %
**Then** the system calculates: (daily rate × working days × allocation%) = total cost impact
**And** displays this amount in real-time

**Given** the cost impact is calculated
**When** I view the preview panel
**Then** I see: Total cost, Monthly breakdown, Impact on department budget

**Given** the assignment would exceed department budget
**When** the cost preview is displayed
**Then** I see a warning: "This assignment consumes Rp XXM of your Rp YYM budget"
**And** the system may require additional approval based on configuration

**Given** I review the cost impact
**When** I click "Confirm Assignment"
**Then** the assignment is saved
**And** the budget utilization is updated in real-time

---

### Story 3.4: Overallocation Warnings

As a **Department Head**,
I want **to receive warnings when assignments would exceed 100% capacity**,
So that **I avoid overcommitting my team members**.

**Acceptance Criteria:**

**Given** an employee has existing allocations totaling 80%
**When** I attempt to add a 30% allocation
**Then** the system displays a warning: "Total allocation would be 110% - confirm over-allocation?"

**Given** an employee has allocations exceeding 100%
**When** I view the team dashboard
**Then** the employee is highlighted with "Overallocated" status
**And** the total allocation percentage is shown in red

**Given** I view the department capacity report
**When** I select a date range
**Then** I see utilization % per employee over time
**And** overallocation periods are visually highlighted

---

### Story 3.5: Department Budget Utilization

As a **Department Head**,
I want **to view my department budget utilization in real-time**,
So that **I can track spending against allocated budget**.

**Acceptance Criteria:**

**Given** I navigate to Department Budget
**When** the page loads
**Then** I see: Total Budget, Allocated (committed), Spent (actual), Remaining

**Given** I view budget details
**When** I expand the breakdown
**Then** I see costs by: Employee, Project, Time period

**Given** I set a budget threshold alert
**When** utilization exceeds that threshold (e.g., 80%)
**Then** I receive a notification
**And** the budget gauge changes color (green → yellow → red)

---

## Epic 4: Project Budget & P&L

{{epic_goal_4}}

### Story 4.1: Project Budget Setup

As a **Project Manager**,
I want **to create project budgets with category breakdown**,
So that **I can track spending against planned allocations**.

**Acceptance Criteria:**

**Given** I am logged in as Project Manager
**When** I navigate to "Create Project"
**Then** I see a form with: Project Name, Client, Start/End Date, Budget Categories

**Given** I set up budget categories
**When** I enter amounts for HR, Software, Hardware, Overhead
**Then** the system validates the total equals the project budget
**And** displays category percentages

**Given** I complete project setup
**When** I click "Save Project"
**Then** the project is created with status="Active"
**And** I am set as the Project Manager

**Given** I view my projects
**When** I select a project
**Then** I see the budget summary with: Total, Spent, Remaining per category

---

### Story 4.2: Non-Resource Cost Entry

As a **Project Manager**,
I want **to enter non-resource costs (expenses, vendor payments)**,
So that **the total project cost includes all expenditures**.

**Acceptance Criteria:**

**Given** I am viewing a project
**When** I click "Add Expense"
**Then** I see a form with: Category, Description, Amount, Date, Vendor (optional)

**Given** I enter an expense
**When** I select the category
**Then** the dropdown shows: HR, Software, Hardware, Overhead

**Given** I save an expense
**When** the entry is created
**Then** it appears in the project cost history
**And** the budget utilization updates immediately

**Given** I need to edit an expense
**When** I click "Edit"
**Then** I can modify details with an "Edit Reason" field
**And** the change is logged for audit

---

### Story 4.3: Automatic Resource Cost Calculation

As a **Project Manager**,
I want **resource costs to be calculated automatically from allocations**,
So that **I don't need to manually compute costs**.

**Acceptance Criteria:**

**Given** resources are assigned to my project
**When** I view the project dashboard
**Then** I see a "Resource Costs" section with: Employee, Daily Rate, Days Allocated, Total Cost

**Given** an assignment spans multiple months
**When** the system calculates costs
**Then** it prorates by working days in each month

**Given** an assignment allocation is less than 100%
**When** costs are calculated
**Then** the amount is: (daily rate × days × allocation%)

**Given** a resource's CTC changes mid-project
**When** costs are recalculated
**Then** the system applies pro-rata for the change period
**And** displays a note about the rate change

---

### Story 4.4: Revenue Entry

As a **Project Manager**,
I want **to enter monthly revenue for my projects**,
So that **P&L calculations have the revenue component**.

**Acceptance Criteria:**

**Given** I navigate to Project → Revenue
**When** the page loads
**Then** I see a month-by-month grid for revenue entry

**Given** I enter revenue for a month
**When** I input the amount
**Then** the system records: Amount, Entry Date, Entered By

**Given** ERP integration is configured
**When** revenue is pulled from the ERP API
**Then** it appears as "ERP Synced" with the source noted
**And** I can override with manual entry if needed

**Given** revenue data exists
**When** I view the P&L
**Then** revenue is displayed by month with year-to-date total

---

### Story 4.5: P&L Dashboard

As a **Project Manager**,
I want **to view a real-time P&L dashboard**,
So that **I can monitor project profitability at a glance**.

**Acceptance Criteria:**

**Given** I navigate to Project → P&L
**When** the page loads (<2 seconds)
**Then** I see: Revenue, Total Costs, Gross Profit, Margin %

**Given** I view the P&L
**When** I select a time period
**Then** I see month-by-month breakdown with charts

**Given** I set a target margin (e.g., 40%)
**When** the current margin differs from target by >5%
**Then** I see an alert: "Margin below target: 33% vs 40% target"

**Given** I view the P&L chart
**When** I hover over a data point
**Then** I see the breakdown: Revenue, Resource Costs, Non-Resource Costs, Margin

---

### Story 4.6: Profitability Forecasting

As a **Project Manager**,
I want **to forecast project profitability at completion**,
So that **I can take corrective action before it's too late**.

**Acceptance Criteria:**

**Given** I view the P&L dashboard
**When** I click "Forecast"
**Then** the system calculates: Projected final cost based on current burn rate

**Given** the forecast is generated
**When** I review the projection
**Then** I see: Current Spend, Projected Total, Forecast Margin, Variance from Target

**Given** the forecast shows declining margins
**When** I analyze the breakdown
**Then** I can see which cost categories are over-running
**And** identify opportunities for resource mix adjustments

**Given** I make resource adjustments
**When** the allocations change
**Then** the forecast updates automatically with new projections

---

## Epic 5: Cash Flow & Compliance

{{epic_goal_5}}

### Story 5.1: Cash Flow Entry

As a **Finance Team member**,
I want **to enter cash receipts and outflows**,
So that **we can track actual cash position vs invoiced revenue**.

**Acceptance Criteria:**

**Given** I navigate to Finance → Cash Flow
**When** I click "Add Cash Entry"
**Then** I see a form with: Type (In/Out), Category, Amount, Date, Description, Project (optional)

**Given** I enter a cash receipt
**When** I select "Cash In"
**Then** categories include: Client Payment, Interest, Other Income

**Given** I enter a cash outflow
**When** I select "Cash Out"
**Then** categories include: Payroll, Vendor Payment, Expense, Tax

**Given** I link a cash entry to a project
**When** I select from the project dropdown
**Then** the entry appears in that project's cash flow view

---

### Story 5.2: Cash Flow Dashboard

As a **Finance Team member**,
I want **to view cash flow by month with in/out breakdown**,
So that **I can monitor liquidity and plan payments**.

**Acceptance Criteria:**

**Given** I navigate to Finance → Cash Flow Dashboard
**When** the page loads
**Then** I see: Monthly Cash In, Cash Out, Net Cash Flow, Cumulative Position

**Given** I view the cash flow chart
**When** I select a date range
**Then** I see a line chart showing cumulative cash position over time

**Given** I filter by project
**When** I select a specific project
**Then** the dashboard shows only cash flow for that project
**And** displays project-level net cash position

**Given** I view cash flow details
**When** I expand a month
**Then** I see all individual entries with drill-down capability

---

### Story 5.3: CTC Validation Reports

As a **Finance Controller**,
I want **to generate CTC validation reports comparing system data to payroll records**,
So that **I can ensure data accuracy for compliance**.

**Acceptance Criteria:**

**Given** I navigate to Finance → CTC Validation
**When** I select a date range and run the report
**Then** the system compares: Xynergy CTC data vs Payroll system records

**Given** the comparison completes
**When** I view the results
**Then** I see: Match Rate %, Discrepancy Count, List of mismatches

**Given** discrepancies are found
**When** I click on a mismatch
**Then** I see: Employee, Field, Xynergy Value, Payroll Value, Variance

**Given** I run the BPJS validation
**When** I select employees to sample
**Then** the system verifies calculations match regulations
**And** flags any calculation errors

---

### Story 5.4: Compliance Audit Reports

As a **Finance Controller**,
I want **to generate comprehensive audit reports**,
So that **external audits can be completed efficiently**.

**Acceptance Criteria:**

**Given** I navigate to Finance → Audit Reports
**When** I select report type and date range
**Then** I can generate: CTC Change Log, Assignment History, Budget Modifications, Access Logs

**Given** I generate a CTC Change Log
**When** the report completes
**Then** I see: Employee, Changed By, Change Date, Field, Old Value, New Value, Reason

**Given** I generate Access Logs
**When** I filter by user or action type
**Then** I see: Timestamp, User, Action, Resource Accessed, Success/Failure

**Given** I need to export for auditors
**When** I click "Export"
**Then** the system initiates four-eyes approval workflow
**And** the export is watermarked with user ID and timestamp

---

## Epic 6: Dashboard & Reporting

{{epic_goal_6}}

### Story 6.1: Role-Based Dashboard

As a **System User**,
I want **to see a personalized dashboard based on my role**,
So that **I immediately see information relevant to my responsibilities**.

**Acceptance Criteria:**

**Given** I am an HR Staff member
**When** I log in
**Then** I see: CTC Completeness Status, Recent CTC Changes, Pending Updates, Compliance Alerts

**Given** I am a Department Head
**When** I log in
**Then** I see: Team Utilization Rates, Budget Status, Overallocations, Upcoming Assignments

**Given** I am a Project Manager
**When** I log in
**Then** I see: Project Health Cards (Budget, P&L, Margin), Active Projects, Margin Alerts

**Given** I am a Finance Team member
**When** I log in
**Then** I see: Cash Position, CTC Validation Status, Audit Alerts, Export Requests Pending

---

### Story 6.2: Real-Time Dashboard Updates

As a **Dashboard User**,
I want **dashboard data to update automatically**,
So that **I always see current information without manual refresh**.

**Acceptance Criteria:**

**Given** I am viewing a dashboard
**When** data changes on the server
**Then** the dashboard updates within 30 seconds (polling interval)

**Given** I need immediate data
**When** I click the "Refresh" button
**Then** the dashboard fetches latest data immediately
**And** displays a timestamp of last update

**Given** the dashboard is updating
**When** new data arrives
**Then** changed values briefly highlight to indicate update

---

### Story 6.3: Project Health Dashboard

As a **Project Manager**,
I want **a project health dashboard showing budget, P&L, and margin status**,
So that **I can quickly identify projects needing attention**.

**Acceptance Criteria:**

**Given** I navigate to Projects Dashboard
**When** the page loads
**Then** I see project cards with: Name, Budget Status (green/yellow/red), Current Margin, Forecast Margin

**Given** I view the project list
**When** a project exceeds budget
**Then** its status indicator shows red
**And** a warning icon appears

**Given** I view the project list
**When** a project's margin is below target
**Then** the margin is displayed in orange/red
**And** I can click for detailed P&L view

**Given** I have multiple projects
**When** I view the dashboard
**Then** I can sort by: Margin, Budget Utilization, End Date

---

### Story 6.4: Team Utilization Dashboard

As a **Department Head**,
I want **to view team utilization rates and budget status**,
So that **I can optimize resource allocation**.

**Acceptance Criteria:**

**Given** I navigate to Team Dashboard
**When** the page loads
**Then** I see: Team members, Current Utilization %, Current Projects, Available Capacity

**Given** I view the utilization chart
**When** I select a time range
**Then** I see utilization trends over time for each team member

**Given** I identify underutilized resources
**When** I see capacity < 50%
**Then** I can click to assign them to new projects

**Given** I view budget status
**When** I look at department summary
**Then** I see: Total Budget, Committed, Spent, Available with visual gauge

---

### Story 6.5: CTC Completeness Dashboard

As an **HR Staff member**,
I want **to view CTC completeness status across all employees**,
So that **I can ensure all employees have complete cost data**.

**Acceptance Criteria:**

**Given** I navigate to HR → CTC Dashboard
**When** the page loads
**Then** I see: Total Employees, With CTC, Missing CTC, Completeness %

**Given** I view completeness by department
**When** I expand the breakdown
**Then** I see: Department name, Employee count, CTC complete count, % complete

**Given** I identify employees missing CTC
**When** I click on the count
**Then** I see a list of employees with "Add CTC" action

**Given** I track completeness over time
**When** I view the trend chart
**Then** I see completeness % by month showing progress toward 100%
