---
stepsCompleted: ['step-01-init', 'step-02-discovery', 'step-03-core-experience', 'step-04-emotional-response', 'step-05-inspiration', 'step-06-design-system', 'step-07-defining-experience', 'step-08-visual-foundation', 'step-09-design-directions']
inputDocuments:
  - '/Users/ipei/webdev/xynergy/_bmad-output/planning-artifacts/prd.md'
  - '/Users/ipei/webdev/xynergy/_bmad-output/project-context.md'
workflowType: 'ux-design'
---

# UX Design Specification - xynergy

**Author:** Putu
**Date:** 2026-02-22

---

<!-- UX design content will be appended sequentially through collaborative workflow steps -->

## Executive Summary

### Project Vision

Xynergy transforms resource allocation from operational scheduling into financial stewardship. The UX must deliver **instant cost visibility** at the moment of decision, transforming financial blindness into data-driven confidence.

### Target Users

| User Type | Count | Comfort | Primary Tasks | Key UX Need |
|:----------|:------|:--------|:--------------|:------------|
| **HR Staff** | 2-3 | High | CTC entry, revisions | Efficiency, bulk operations, validation feedback |
| **Department Heads** | 5-8 | Medium | Resource assignments | Quick decisions, visual cost indicators, budget guardrails |
| **Project Managers** | 10-15 | High | Budget management, P&L | Rich analysis, export capabilities, forecasting |
| **Finance Team** | 2-3 | Very High | Compliance, reconciliation | Validation tools, audit trails, accuracy verification |
| **Executives** | 3-5 | Low-Medium | High-level oversight | Minimal clicks, clear KPIs, drill-down capability |

### Key Design Challenges

1. **Financial Data Trust** - Users must feel 100% confident in accuracy; errors are career-impacting
2. **Efficiency at Scale** - Monthly rhythm requires streamlined workflows for 60+ employees
3. **Cross-User Impact Visibility** - CTC changes affect multiple users; need clear impact communication
4. **Error Recovery** - Catastrophic error potential requires robust validation and undo capabilities
5. **Confidential-by-Design Payroll UX** - CTC component values are rendered only in HR-authorized flows; non-HR experiences must never expose component plaintext.

### Design Opportunities

1. **Cost-Aware Assignment** - Visual cost preview before confirmation (core differentiator)
2. **Transparent Calculations** - Show formulas, highlight changes, build trust through visibility
3. **Role-Optimized Workflows** - Tailored interfaces for each user type's specific needs
4. **Confidence-Building Feedback** - Clear validation, confirmation summaries, undo capabilities

---

## Core User Experience

### Defining Experience

**Core User Action:** Resource assignment with instant cost visibility

The defining interaction is the **cost-aware assignment moment**: User selects a resource, immediately sees the financial impact, and makes an informed decision. This transforms the mindset from "who is available?" to "what can I afford?"

**Critical Flow:**
1. Department Head views team dashboard
2. Selects resource for assignment
3. Sees instant cost preview (daily rate × duration = total cost)
4. Views remaining budget impact
5. Confirms with full context summary

**Frequency:** This action happens dozens of times per month across all Department Heads and Project Managers.

### Platform Strategy

**Primary Platform:** Desktop Web Application (SPA)
- Complex dashboards with charts and data tables
- Multi-field forms for CTC entry
- Drag-and-drop resource assignment
- Rich P&L analysis tools

**Secondary Platform:** Tablet (iPad)
- Quick approval workflows
- Dashboard overview for executives
- Status checks on-the-go

**Technical Constraints:**
- Modern browsers only (Chrome 90+, Firefox 88+, Safari 14+)
- WebAssembly compilation target (performance-critical calculations)
- 30-second polling updates (MVP), SSE for real-time (Post-MVP)
- Responsive design minimum 768px width

**Interaction Modes:**
- **Mouse/Keyboard:** Primary for detailed work
- **Touch:** Secondary for tablet approvals (minimum 44×44px touch targets)

### Effortless Interactions

**1. Cost Impact Visibility**
- Select resource → Cost appears instantly (<200ms)
- No page loads, no hunting for information
- One-glance comprehension: "This costs Rp 45M"

**2. Calculation Transparency**
- Hover over any number → See formula tooltip
- Daily rate = Monthly CTC ÷ Working days
- BPJS = (Base + Allowances) × Rate
- No black boxes, complete transparency

**3. Validation Feedback**
- Green checkmark + "Looks good!" for valid entries
- Red indicator + specific fix for errors
- Real-time validation prevents propagation

**4. Decision Confidence**
- Confirmation modal with full context
- "Assigning [Resource] to [Project] for [Duration] at [Rate] = [Total Cost]"
- Remaining budget clearly displayed
- One-click confirm or cancel

**Eliminated Friction:**
- ❌ Email chains asking "what's the rate?"
- ❌ Spreadsheet version confusion
- ❌ Manual calculation errors
- ❌ Month-end budget surprises

### Critical Success Moments

**Moment 1: First Cost-Aware Assignment**
- Dept Head assigns first resource with cost preview
- Realization: "I can see impact BEFORE committing"
- Emotional response: Empowerment, control
- **Success indicator:** User immediately assigns 3+ more resources

**Moment 2: First P&L Generation**
- PM generates monthly P&L in 2 seconds
- Sees profit margin instantly vs. 2-hour spreadsheet process
- Emotional response: Delight, time savings
- **Success indicator:** PM shares screen with colleague

**Moment 3: CTC Update Impact Visibility**
- HR updates salary, sees affected allocations
- PM sees updated costs automatically
- No emails, no confusion
- Emotional response: Trust, system reliability
- **Success indicator:** User recommends system to peer

**Failure Scenarios to Avoid:**
- Cost preview is slow (>1 second) → User ignores it
- Calculation error discovered → Complete trust loss
- No undo on assignment → User hesitates, adoption drops
- Unclear confirmation → User anxiety, second-guessing

### Experience Principles

**1. Transparency Builds Trust**
- Every number is explainable (hover for formulas)
- Every change is visible (audit trails, impact previews)
- Every decision is informed (cost before commitment)

**2. Efficiency Enables Scale**
- Monthly tasks complete in minutes, not hours
- Bulk operations for repetitive actions (HR entry)
- Smart defaults anticipate user needs

**3. Clarity Prevents Errors**
- Validation catches mistakes before propagation
- Visual indicators show status at a glance
- Confirmation summaries provide full context

**4. Confidence Drives Adoption**
- Users feel certain before confirming actions
- Undo capability provides safety net
- Progressive disclosure reduces cognitive load

**5. Impact Visibility is King**
- Financial impact shown at point of decision
- No surprises, no hidden costs
- Real-time budget awareness

---

## Desired Emotional Response

### Primary Emotional Goals

**Core Emotional Experience:** Confidence, Empowerment, and Trust

Users should feel **confident** in their financial decisions, **empowered** by real-time visibility, and **trusting** of the system's accuracy. These emotions transform resource allocation from anxiety-inducing guesswork into data-driven stewardship.

**Differentiating Emotion:** Clarity vs. Uncertainty

Unlike competitors that leave users wondering about budget impact, Xynergy provides immediate clarity. Users transition from "I hope this fits the budget" to "I know exactly what this costs."

**Viral Emotion:** Delight at Efficiency

The "2-second P&L vs. 2-hour spreadsheet" moment creates genuine delight. Users want to share this efficiency gain with colleagues.

### Emotional Journey Mapping

**First Discovery (Onboarding):**
| Stage | Emotion | UX Support |
|:------|:--------|:-----------|
| Initial Exposure | Curiosity | Intriguing "cost-aware" concept |
| First Login | Skepticism | Clean, professional interface builds credibility |
| First Assignment | Surprise | Instant cost preview exceeds expectations |
| First P&L Generation | Delight | Sub-2-second speed creates "wow" moment |

**During Core Experience (Monthly Rhythm):**
| Stage | Emotion | UX Support |
|:------|:--------|:-----------|
| CTC Updates (HR) | Efficiency | Bulk operations, smart defaults |
| Resource Assignment (Dept Heads) | Confidence | Clear cost preview, budget impact visible |
| P&L Review (PMs) | Control | Real-time margin tracking, forecasting |
| Compliance Check (Finance) | Trust | Audit trails, validation reports |

**After Completing Task:**
- **Satisfaction** - Task complete, budget intact
- **Accomplishment** - Monthly work done in minutes not hours
- **Empowerment** - Data-driven decisions made confidently

**When Returning (Habit Formation):**
- **Familiarity** - "Back to my dashboard"
- **Efficiency** - "Faster than my old spreadsheets"
- **Reliance** - "I need this for every decision now"

**When Something Goes Wrong:**
- **Concern → Reassurance** (undo available, clear audit trail)
- **Confusion → Clarity** (specific error messages with fixes)
- **Frustration → Support** (validation prevents errors before propagation)

### Micro-Emotions

**Critical Emotional States to Cultivate:**

✅ **Confidence** (Avoid: Confusion)
- Clear cost previews at point of decision
- Transparent calculation formulas
- Visual validation indicators

✅ **Trust** (Avoid: Skepticism)
- Complete audit trail visibility
- Calculation formulas on hover
- Historical accuracy validation

✅ **Accomplishment** (Avoid: Frustration)
- Monthly tasks complete in minutes
- No spreadsheet reconciliation needed
- Clear progress indicators

✅ **Delight** (Avoid: Mere Satisfaction)
- 2-second P&L generation speed
- Instant cost visibility magic
- "It just works" reliability

**Emotional States to Prevent:**

❌ **Anxiety** - Prevent with clear impact visibility
❌ **Doubt** - Prevent with transparent calculations
❌ **Overwhelm** - Prevent with progressive disclosure
❌ **Regret** - Prevent with undo capabilities

### Design Implications

**Emotion → UX Design Approach:**

**Confidence → Transparent UI:**
- Hover tooltips showing formulas: "Daily Rate = Rp 22,000,000 ÷ 22 days = Rp 1,000,000"
- Clear cost breakdowns in confirmation modals
- Green validation indicators for correct entries

**Trust → Reliable Feedback:**
- Audit trail accessible from every screen
- Confirmation summaries before destructive actions
- Error prevention through real-time validation

**Accomplishment → Efficient Workflows:**
- Progress bars for multi-step processes
- Bulk operations for HR data entry
- Smart defaults reducing clicks

**Delight → Speed & Polish:**
- Skeleton screens during loading
- Smooth page transitions
- Celebratory micro-interactions (subtle checkmark animations)

### Emotional Design Principles

**1. Confidence Through Transparency**
Every number is explainable. Every action has clear impact. Every decision is informed. Users never wonder "where did this number come from?"

**2. Trust Through Reliability**
System behaves predictably. Errors are caught early. Recovery is always possible. Users trust the system with career-impacting financial data.

**3. Empowerment Through Visibility**
Financial impact is immediate. Data drives decisions. No more flying blind. Users feel in control of their budgets and projects.

**4. Delight Through Efficiency**
Tasks complete faster than expected. "It just works" experience. Time savings compound. Users genuinely enjoy using the system.

**5. Safety Through Recoverability**
Undo is always available. Audit trails show exactly what happened. Errors are caught before propagation. Users feel safe to act decisively.

---

## UX Pattern Analysis & Inspiration

### Inspiring Products Analysis

**Primary Inspiration Sources:**

**1. Stripe Dashboard**
- **Core Problem Solved:** Complex financial data made trustworthy and actionable
- **UX Strengths:** Clean data tables, transparent calculations, excellent visualization
- **Delight Factor:** Users trust the numbers because they can see exactly how they're calculated
- **Why It Works:** Financial professionals need clarity, not flashy design

**2. Linear**
- **Core Problem Solved:** Speed-first issue tracking without complexity
- **UX Strengths:** Command palette, instant feedback, keyboard-first design
- **Delight Factor:** Everything happens immediately—no waiting, no spinners
- **Why It Works:** Power users appreciate efficiency and shortcuts

**3. Notion / Airtable**
- **Core Problem Solved:** Flexible data views for different use cases
- **UX Strengths:** View switching, clean typography, real-time indicators
- **Delight Factor:** Same data, multiple perspectives—user controls their view
- **Why It Works:** Different roles need different views of the same data

**Secondary Inspiration Sources:**

**4. Excel / Google Sheets**
- **Why Users Love It:** Familiar patterns, grid interface, formula visibility
- **Current Pain Points:** Version confusion, no audit trail, manual reconciliation
- **What to Keep:** Inline editing, formula transparency, keyboard navigation

**5. Monday.com / Asana**
- **Why Users Love It:** Visual project management, color-coded status
- **Current Pain Points:** Can become cluttered, limited financial visibility
- **What to Keep:** Timeline views, drag-and-drop, status indicators

---

### Transferable UX Patterns

**Navigation & Layout:**

| Pattern | Source | Application in Xynergy |
|:--------|:-------|:-----------------------|
| Command Palette (Cmd+K) | Linear | Quick navigation between CTC, assignments, P&L |
| View Switching | Notion | Same data: Table view (HR), Timeline view (PMs), Dashboard (Execs) |
| Sidebar Navigation | Linear | Collapsible nav with keyboard shortcuts |
| Breadcrumb Trails | Stripe | Clear location: Dept > Project > Assignment |

**Interaction Patterns:**

| Pattern | Source | Application in Xynergy |
|:--------|:-------|:-----------------------|
| Inline Cell Editing | Excel | Click CTC cell → edit directly, no modal |
| Hover Tooltips | Stripe | Hover over cost → see formula: "Daily Rate = Monthly ÷ 22" |
| Optimistic UI | Linear | Click assign → immediate visual feedback, sync in background |
| Command Palette | Linear | Cmd+K → "assign to project", "generate P&L", "view audit" |

**Data Visualization:**

| Pattern | Source | Application in Xynergy |
|:--------|:-------|:-----------------------|
| Clean Tables | Stripe | Right-aligned numbers, proper formatting (Rp 1,000,000) |
| Status Badges | Stripe | Green (healthy), Yellow (warning), Red (overrun) |
| Drill-Down | Stripe | Click budget number → see detailed breakdown |
| Timeline/Gantt | Monday.com | Visual resource allocation over time |

**Feedback & Trust:**

| Pattern | Source | Application in Xynergy |
|:--------|:-------|:-----------------------|
| Calculation Transparency | Stripe | Show formulas, not just results |
| Audit Trail | GitHub | Timeline of all changes with who/when/what |
| Auto-Save | Notion | No save button needed—just happens |
| Undo/Redo | Excel | Ctrl+Z works everywhere |

---

### Anti-Patterns to Avoid

| Anti-Pattern | Why Avoid | Better Approach |
|:-------------|:----------|:----------------|
| **Jira's Complexity** | Too many options overwhelm users | Linear's simplicity—focused, minimal chrome |
| **Traditional ERP Clutter** | Dense interfaces hide important actions | Stripe's clarity—content-first, breathing room |
| **Spreadsheet Version Chaos** | No single source of truth | System-enforced audit trails and versioning |
| **Modal-Heavy Workflows** | Disorienting, lose context | Side panels or inline editing |
| **Hidden Calculations** | Users distrust black boxes | Transparent formulas visible on hover |
| **Slow Feedback** | Users wonder if action worked | Optimistic UI with instant visual feedback |

---

### Visual Metaphors

| Concept | Metaphor | Implementation |
|:--------|:---------|:---------------|
| **Budget Health** | Traffic Light | Green (on track), Yellow (80% used), Red (overrun) |
| **Cost Impact** | Price Tag | Cost displayed next to every resource assignment |
| **Resource Allocation** | Calendar Timeline | Visual blocks showing who is assigned when |
| **P&L Status** | Dashboard Gauges | At-a-glance metrics with trend indicators |
| **Audit Trail** | Git Commit History | Timeline showing every change with diff view |

---

### Design Inspiration Strategy

**What to Adopt:**

1. **Stripe's Financial Clarity**
   - Clean data tables with excellent typography
   - Transparent calculations build trust
   - Drill-down from summary to details

2. **Linear's Speed-First Design**
   - Command palette for power users
   - Keyboard shortcuts for common actions
   - Optimistic UI with instant feedback

3. **Notion's Flexible Views**
   - Same data, different perspectives
   - Side panel editing (not modals)
   - Real-time collaboration indicators

**What to Adapt:**

1. **Excel's Familiar Patterns**
   - Grid editing for CTC data (but with validation)
   - Formula visibility (but calculated automatically)
   - Keyboard navigation (but enhanced with shortcuts)

2. **Monday.com's Visual Approach**
   - Timeline views for resource allocation
   - Color-coded status indicators
   - But simplify—avoid clutter

**What to Avoid:**

1. **Jira's Complexity**
   - Too many configuration options
   - Overwhelming interface for new users

2. **Traditional ERP Density**
   - Information overload
   - Hidden actions behind menus

---

**Design System Direction:**

**Visual Style:**
- Clean, minimalist (Linear/Stripe influence)
- High information density (Excel influence)
- Clear visual hierarchy (Notion influence)

**Interaction Model:**
- Keyboard-first with full mouse support
- Command palette for power users (Cmd+K)
- Inline editing wherever possible
- Optimistic UI with real-time feedback

**Color Strategy:**
- Professional blues and grays (trustworthy)
- Status colors: Green (healthy), Yellow (warning), Red (critical)
- High contrast for accessibility (WCAG 2.1 AA)

**Typography:**
- Clean, readable sans-serif
- Monospace for numbers (alignment)
- Clear hierarchy: headers, labels, data
---

## Design System Foundation

### Design System Choice

**Selected Approach:** Tailwind CSS 3.4 + Custom Leptos Component Library

**Rationale:**

| Factor | Why Tailwind Fits |
|:-------|:------------------|
| **Tech Stack Alignment** | Already in Xynergy tech stack (confirmed in project-context.md) |
| **Leptos Compatibility** | Works perfectly with Rust/WebAssembly SPA |
| **4-Week Timeline** | Rapid styling without custom CSS |
| **Inspiration Alignment** | Easy to implement Stripe/Linear clean aesthetic |
| **Accessibility** | Built-in accessibility utilities (WCAG 2.1 AA) |
| **Performance** | Purges unused CSS, small bundle size |

### Implementation Approach

**Foundation Stack:**
- **Tailwind CSS 3.4** - Utility-first CSS framework
- **Lucide Icons** - Consistent iconography
- **Custom Leptos Components** - Built on Tailwind utilities
- **CSS Custom Properties** - Design tokens for theming

### Design Tokens

**CSS Variables (globals.css):**

```css
:root {
  /* Colors - Professional Blue/Gray Palette */
  --color-primary: #2563eb;      /* Blue 600 - trust, actions */
  --color-success: #16a34a;      /* Green 600 - on track, healthy */
  --color-warning: #ca8a04;      /* Yellow 600 - caution, approaching limit */
  --color-danger: #dc2626;       /* Red 600 - overrun, critical */
  --color-neutral-50: #f9fafb;   /* Gray 50 - page background */
  --color-neutral-100: #f3f4f6;  /* Gray 100 - card backgrounds */
  --color-neutral-200: #e5e7eb;  /* Gray 200 - borders, dividers */
  --color-neutral-700: #374151;  /* Gray 700 - secondary text */
  --color-neutral-900: #111827;  /* Gray 900 - primary text */
  
  /* Spacing - 4px base unit */
  --space-1: 0.25rem;   /* 4px - tight spacing, icons */
  --space-2: 0.5rem;    /* 8px - compact elements */
  --space-3: 0.75rem;   /* 12px - default padding */
  --space-4: 1rem;      /* 16px - standard gaps */
  --space-6: 1.5rem;    /* 24px - section spacing */
  --space-8: 2rem;      /* 32px - large sections */
  --space-12: 3rem;     /* 48px - page-level spacing */
  
  /* Typography */
  --font-sans: 'Inter', system-ui, -apple-system, sans-serif;
  --font-mono: 'JetBrains Mono', 'Fira Code', monospace;
  
  /* Shadows */
  --shadow-sm: 0 1px 2px 0 rgb(0 0 0 / 0.05);
  --shadow-md: 0 4px 6px -1px rgb(0 0 0 / 0.1);
  --shadow-lg: 0 10px 15px -3px rgb(0 0 0 / 0.1);
  --shadow-xl: 0 20px 25px -5px rgb(0 0 0 / 0.1);
}
```

### Component Library Structure

**Core Components (to implement in Leptos):**

| Component | Purpose | Key Tailwind Classes |
|:----------|:--------|:---------------------|
| `Button` | Actions, confirmations | `px-4 py-2 rounded-md font-medium transition-colors` |
| `DataTable` | CTC lists, P&L tables | `w-full text-sm border-collapse divide-y` |
| `Input` | Form fields | `w-full px-3 py-2 border rounded-md focus:ring-2` |
| `Select` | Dropdowns | `w-full px-3 py-2 border rounded-md appearance-none` |
| `Modal` | Confirmations | `fixed inset-0 bg-black/50 flex items-center justify-center` |
| `SidePanel` | Edit forms | `fixed right-0 top-0 h-full w-96 shadow-xl transform` |
| `Badge` | Status indicators | `px-2 py-1 rounded-full text-xs font-medium` |
| `Tooltip` | Calculation info | `absolute z-50 px-2 py-1 bg-gray-900 text-white text-xs` |
| `CommandPalette` | Quick navigation | `fixed inset-0 bg-black/50 flex items-start justify-center` |

### Color Palette

| Role | Color | Hex | Usage |
|:-----|:------|:----|:------|
| **Primary** | Blue 600 | `#2563eb` | Buttons, links, active states, trust signals |
| **Success** | Green 600 | `#16a34a` | On budget, healthy metrics, positive actions |
| **Warning** | Yellow 600 | `#ca8a04` | Approaching limit, caution, attention needed |
| **Danger** | Red 600 | `#dc2626` | Over budget, critical alerts, destructive actions |
| **Neutral-50** | Gray 50 | `#f9fafb` | Page background |
| **Neutral-100** | Gray 100 | `#f3f4f6` | Card backgrounds, hover states |
| **Neutral-200** | Gray 200 | `#e5e7eb` | Borders, dividers, subtle backgrounds |
| **Neutral-700** | Gray 700 | `#374151` | Secondary text, muted content |
| **Neutral-900** | Gray 900 | `#111827` | Primary text, headings |

### Typography System

| Element | Font | Size | Weight | Line Height |
|:--------|:-----|:-----|:-------|:------------|
| **H1** | Inter | 1.875rem (30px) | 700 (Bold) | 1.2 |
| **H2** | Inter | 1.5rem (24px) | 600 (Semibold) | 1.3 |
| **H3** | Inter | 1.25rem (20px) | 600 (Semibold) | 1.4 |
| **Body** | Inter | 1rem (16px) | 400 (Regular) | 1.5 |
| **Small** | Inter | 0.875rem (14px) | 400 (Regular) | 1.5 |
| **Label** | Inter | 0.75rem (12px) | 500 (Medium) | 1.4 |
| **Numbers** | JetBrains Mono | 1rem (16px) | 400 (Regular) | 1.5 |
| **Currency** | JetBrains Mono | 1.125rem (18px) | 500 (Medium) | 1.5 |

### Spacing System

| Token | Value | Usage |
|:------|:------|:------|
| `space-1` | 0.25rem (4px) | Icon spacing, tight gaps |
| `space-2` | 0.5rem (8px) | Compact elements, inline spacing |
| `space-3` | 0.75rem (12px) | Default padding, form fields |
| `space-4` | 1rem (16px) | Standard gaps, card padding |
| `space-6` | 1.5rem (24px) | Section spacing, grid gaps |
| `space-8` | 2rem (32px) | Large sections, page headers |
| `space-12` | 3rem (48px) | Page-level spacing, major sections |

### Layout Grid

**Container:**
- Max-width: 1280px (`max-w-7xl`)
- Centered: `mx-auto`
- Padding: `px-4 sm:px-6 lg:px-8`

**Grid System:**
- 12-column grid with 24px gutters
- Responsive breakpoints:
  - `sm`: 640px (tablet portrait)
  - `md`: 768px (tablet landscape)
  - `lg`: 1024px (desktop)
  - `xl`: 1280px (large desktop)

### Accessibility Requirements (WCAG 2.1 AA)

| Requirement | Implementation |
|:------------|:---------------|
| **Color Contrast** | 4.5:1 minimum for text (Tailwind's default palette meets this) |
| **Touch Targets** | Minimum 44×44px (`min-w-[44px] min-h-[44px]`) |
| **Focus Indicators** | Visible focus rings (`focus:ring-2 focus:ring-primary`) |
| **Keyboard Navigation** | All interactive elements focusable and operable |
| **Screen Reader Support** | ARIA labels on icons, buttons, data tables |
| **Reduced Motion** | Respect `prefers-reduced-motion` media query |

### Customization Strategy

**Component Variants:**

Components support multiple variants (Primary, Secondary, Danger) through Tailwind class composition.

**Dark Mode Support:**
- Use Tailwind's `dark:` modifier for dark mode variants
- Design tokens support CSS variables for easy theming

**Responsive Patterns:**
- Mobile-first approach (default styles for mobile, `md:` for desktop)
- Touch-friendly on tablets (44×44px targets)
- Collapsible navigation on smaller screens

---

## Defining Core Experience

### Defining Experience

**Core Interaction:** Resource assignment with instant cost visibility

The defining experience is the moment when a Department Head or Project Manager selects a resource for assignment and immediately sees the financial impact - transforming "blind" resource allocation into informed cost-aware decisions.

**The "Aha Moment":** 
Users realize they can see exact costs **before** confirming assignments, eliminating month-end budget surprises and enabling data-driven staffing decisions.

**User Description:**
> "I can see exactly what every assignment costs before I confirm it. No more month-end budget surprises. I make data-driven staffing decisions in seconds."

### User Mental Model

**Current Approach (Painful):**
1. Check who's available (spreadsheet/resource tool)
2. Assign person to project (without cost visibility)
3. Hope budget works out
4. Discover overruns at month-end

**Desired Mental Model (Xynergy):**
1. Check who's available (with blended rate visible)
2. Select resource → See instant cost preview
3. Evaluate budget impact
4. Confirm with confidence

**User Expectations:**
- Cost visibility should be as natural as seeing a price tag
- Budget impact should be clear and immediate
- Mistakes should be recoverable (undo)

**Where Users Get Confused:**
- Delayed cost calculation (feels broken)
- Unclear budget impact ("what does this mean?")
- No way to compare options
- Accidental assignments without confirmation

### Success Criteria

| Criteria | Target | Measurement |
|:---------|:-------|:------------|
| Cost Preview Speed | < 200ms | Feels instant |
| Assignment Confirmation | < 500ms | Optimistic UI |
| User Confidence | 90%+ | Post-assignment survey |
| Error Prevention | > 95% | Validation catch rate |

**Success Indicators:**
- ✅ User immediately assigns 3+ resources in first session
- ✅ Cost preview used for 90%+ of assignments
- ✅ Zero "what's the rate?" support requests
- ✅ Users proactively check budget before assigning

### Novel UX Patterns

**Pattern Type:** Novel combination of established patterns

**Established Patterns Used:**
- Resource selection (dropdown/search)
- Form submission with validation
- Confirmation dialogs
- Real-time calculation feedback

**Novel Combination:**
- Cost visibility **at** point of decision (not after)
- Budget impact integrated into assignment flow
- Real-time calculation with visual feedback

**No User Education Required:**
The pattern is intuitive - users already want to know costs, Xynergy simply shows them at the right moment.

**Metaphor:** Shopping cart with instant price updates
- Select item → See price immediately
- Adjust quantity → Price updates
- Checkout with confidence

### Experience Mechanics

**Flow: Resource Assignment with Cost Visibility**

**Step 1: Initiation**
- Navigate to "Resource Assignment" page
- Click "Assign Resource" or select team member
- Trigger: Need to staff a project (PM request or proactive planning)

**Step 2: Interaction**
- **Select Resource:** Dropdown with avatar + blended rate displayed
- **Select Project:** Dropdown with project code and name
- **Set Duration:** Date range picker (start/end dates)
- **Set Allocation:** Percentage slider (default: 100%)

System automatically calculates: **Daily Rate × Duration × Allocation% = Total Cost**

**Step 3: Feedback**
- Real-time cost preview updates as inputs change
- Budget impact visualized (progress bar fills up)
- Color-coded indicators:
  - 🟢 Green: Healthy budget impact (<50%)
  - 🟡 Yellow: Approaching limit (50-80%)
  - 🔴 Red: Budget overrun risk (>80%) or overallocation
- Hover on cost → See formula breakdown

**Step 4: Completion**
- Review confirmation summary modal:
  - Resource name + rate
  - Project name
  - Duration + allocation%
  - Total cost
  - Remaining budget after assignment
- Click "Confirm Assignment"
- Success toast notification
- Dashboard updates automatically

**Error Handling:**
- Invalid date range → Red inline error with fix
- Overallocation → Yellow warning with capacity details
- Budget overrun → Red warning with alternative suggestions

---

## Visual Design Foundation

### Color System

**Primary Palette (Professional Blue):**

| Color | Hex | Tailwind | Usage |
|:------|:----|:---------|:------|
| **Primary** | `#2563eb` | blue-600 | Trust, actions, links, buttons |
| **Primary Hover** | `#1d4ed8` | blue-700 | Interactive hover states |
| **Primary Light** | `#dbeafe` | blue-100 | Backgrounds, highlights, badges |

**Semantic Colors:**

| Color | Hex | Tailwind | Usage |
|:------|:----|:---------|:------|
| **Success** | `#16a34a` | green-600 | On budget, healthy metrics, positive actions |
| **Warning** | `#ca8a04` | yellow-600 | Caution, approaching limits, attention |
| **Danger** | `#dc2626` | red-600 | Over budget, critical alerts, destructive actions |

**Neutral Grays (Clean, Modern):**

| Color | Hex | Tailwind | Usage |
|:------|:----|:---------|:------|
| **Background** | `#f9fafb` | gray-50 | Page background |
| **Surface** | `#ffffff` | white | Cards, modals, elevated surfaces |
| **Border** | `#e5e7eb` | gray-200 | Dividers, borders, separators |
| **Text Secondary** | `#6b7280` | gray-500 | Labels, hints, metadata |
| **Text Primary** | `#111827` | gray-900 | Headings, body text, primary content |

**Color Psychology:**

| Color | Emotion | Application |
|:------|:--------|:------------|
| **Blue** | Trust, professionalism, stability | Primary actions, financial data |
| **Green** | Growth, success, on-track | Budget healthy, positive indicators |
| **Yellow** | Caution, attention | Approaching limits, warnings |
| **Red** | Urgency, critical | Over budget, errors, destructive |

### Typography System

**Font Families:**

| Font | Role | Usage |
|:-----|:-----|:------|
| **Inter** | Primary | All text, headings, body, UI elements |
| **JetBrains Mono** | Monospace | Numbers, currency, code, aligned data |

**Type Scale:**

| Level | Size | Weight | Line Height | Usage |
|:------|:-----|:-------|:------------|:------|
| **H1** | 30px / 1.875rem | 700 (Bold) | 1.2 | Page titles, main headings |
| **H2** | 24px / 1.5rem | 600 (Semibold) | 1.3 | Section headers |
| **H3** | 20px / 1.25rem | 600 (Semibold) | 1.4 | Card titles, subsections |
| **Body** | 16px / 1rem | 400 (Regular) | 1.5 | Paragraphs, descriptions |
| **Small** | 14px / 0.875rem | 400 (Regular) | 1.5 | Captions, metadata, secondary |
| **Label** | 12px / 0.75rem | 500 (Medium) | 1.4 | Form labels, tags |
| **Currency** | 18px / 1.125rem | 500 (Medium) | 1.5 | Financial amounts (monospace) |
| **Numbers** | 16px / 1rem | 400 (Regular) | 1.5 | Data values (monospace) |

**Typography Patterns:**

- **Numbers/Amounts:** Monospace font, right-aligned for easy scanning
- **Currency:** Monospace font, medium weight for prominence
- **Line Height:** 1.5 for body (readability), 1.2-1.4 for headings (tight)
- **Max Width:** 65ch for body text (optimal reading length)

### Spacing & Layout Foundation

**Spacing Scale (4px base unit):**

| Token | Value | Usage |
|:------|:------|:------|
| `space-1` | 4px | Icon gaps, tight spacing |
| `space-2` | 8px | Inline elements, compact spacing |
| `space-3` | 12px | Default padding, form fields |
| `space-4` | 16px | Standard gaps, card internal spacing |
| `space-6` | 24px | Section spacing, grid gaps |
| `space-8` | 32px | Large sections, page headers |
| `space-12` | 48px | Page-level spacing, major sections |

**Layout Principles:**

| Principle | Implementation |
|:----------|:---------------|
| **Container** | max-width 1280px, centered with `mx-auto` |
| **Grid** | 12-column grid with 24px gutters |
| **Card Padding** | 24px (`p-6`) standard |
| **Section Spacing** | 32px (`space-y-8`) between major sections |
| **Information Density** | High - financial data requires visibility |
| **White Space** | Strategic - breathing room around key actions |

**Responsive Breakpoints:**

| Breakpoint | Width | Usage |
|:-----------|:------|:------|
| `sm` | 640px | Tablet portrait, mobile landscape |
| `md` | 768px | Tablet landscape |
| `lg` | 1024px | Desktop |
| `xl` | 1280px | Large desktop |

### Accessibility Considerations

**Color Contrast:**

| Element | Contrast Ratio | Standard |
|:--------|:---------------|:---------|
| Body text | 4.5:1 minimum | WCAG 2.1 AA |
| Large text (18px+) | 3:1 minimum | WCAG 2.1 AA |
| Interactive elements | Visible focus states | WCAG 2.1 AA |

**Typography:**

| Requirement | Implementation |
|:------------|:---------------|
| Minimum body size | 16px |
| Line height | 1.5 for readability |
| Letter spacing | Sufficient for readability |

**Touch Targets:**

| Requirement | Implementation |
|:------------|:---------------|
| Minimum size | 44×44px |
| Spacing between targets | Adequate to prevent mis-taps |

**Focus Management:**

| Element | Focus Style |
|:--------|:------------|
| Buttons | `focus:ring-2 focus:ring-primary focus:ring-offset-2` |
| Inputs | `focus:ring-2 focus:ring-primary focus:border-primary` |
| Links | `focus:underline focus:outline-none` |

### Visual Design Principles

**1. Content First**
- Minimal chrome, maximum data visibility
- Information density optimized for financial data review
- Chrome (UI elements) supports content, doesn't compete

**2. Visual Hierarchy**
- Clear distinction: headings → data → metadata
- Size, weight, and color create information hierarchy
- Important actions are prominent, secondary actions are subdued

**3. Consistent Spacing**
- Predictable rhythm with 4px base unit
- Consistent spacing creates visual stability
- Users learn patterns quickly

**4. Semantic Colors**
- Colors convey meaning beyond aesthetics
- Success/warning/danger are universally understood
- Consistent color usage builds intuition

**5. Professional Trust**
- Conservative color palette builds credibility
- Clean typography conveys precision
- Consistent spacing shows attention to detail

---

## Design Direction Decision

### Design Directions Explored

**Direction 1: Stripe Financial (Selected)**
- **Focus:** Maximum clarity for financial data
- **Characteristics:** Clean, spacious layout with card-based organization
- **Best For:** Trust-building and financial analysis
- **Visual Weight:** Medium density with breathing room

**Direction 2: Linear Speed**
- **Focus:** Speed and efficiency for power users
- **Characteristics:** Compact, information-dense, keyboard-driven
- **Best For:** Power users prioritizing speed
- **Visual Weight:** High density, minimal whitespace

**Direction 3: Notion Flexibility**
- **Focus:** Flexible views for different user types
- **Characteristics:** Adaptable layout with view switching
- **Best For:** Teams needing customization
- **Visual Weight:** Medium density with flexibility

### Chosen Direction

**Selected:** Stripe Financial

**Rationale:**
- Aligns with Xynergy's core value of transparency and trust
- Optimized for financial data visibility
- Low learning curve for users transitioning from spreadsheets
- Excellent accessibility compliance (WCAG 2.1 AA)
- Directly applies proven patterns from Stripe Dashboard

**Key Characteristics:**
- Clean, spacious data tables
- Card-based metric organization
- Side-by-side assignment view
- Generous whitespace for clarity
- Professional blue/gray color palette

### Design Rationale

**Why Stripe Financial Works for Xynergy:**

1. **Trust-Building:** Clean layout shows data transparently, building user confidence in financial calculations

2. **Financial Focus:** Optimized layout for cost visibility and budget impact - our core differentiator

3. **Accessibility:** Excellent contrast ratios and readability for all users

4. **Familiarity:** Similar to Excel/spreadsheet layouts users already know

5. **Scalability:** Works well from 10 to 100+ employees without redesign

**Modifications for Xynergy:**
- Add prominent cost preview panel (core differentiator)
- Include real-time budget impact visualizations
- Optimize for 1280px desktop (primary use case)
- Maintain high information density for financial data

### Implementation Approach

**Layout Strategy:**

**Dashboard View:**
- Large metric cards at top (KPIs)
- Detailed data tables below
- Clean separation between summary and detail

**Resource Assignment View:**
- Side-by-side layout: Team list | Assignment form
- Real-time cost preview prominent
- Budget impact visualization

**P&L View:**
- Full-width charts with summary metrics
- Drill-down tables for details
- Export functionality accessible

**Component Patterns:**
- Data tables: Stripe-style clean tables with hover states
- Cards: Subtle shadows, clear borders
- Forms: Inline validation, clear labels
- Navigation: Sidebar with collapsible sections
