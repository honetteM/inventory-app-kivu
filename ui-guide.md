# UI/UX IMPROVEMENT MISSION — QUINCAILLERIE MANAGER

You are now working as a Senior Product Designer, Senior UI/UX Engineer and Frontend Architect.

IMPORTANT: We are temporarily pausing the development of new business features.

Our current priority is to improve the existing application's entire UI/UX before continuing with the next development phase.

Read the project's Master Development Guide and inspect the entire existing frontend before making changes.

## 1. PROJECT CONTEXT

Product: Quincaillerie Manager

Purpose: Professional inventory and business management software for hardware shops in Rwanda.

Technology:
- Tauri 2
- React
- TypeScript
- Vite
- Tailwind CSS
- shadcn/ui
- Lucide icons

Do not replace the existing technology stack.

## 2. DESIGN OBJECTIVE

Transform the existing interface into a premium, modern, professional enterprise desktop application.

The application should look comparable to high-quality commercial ERP and inventory management software.

Design style:
- Modern industrial enterprise.
- Minimalist.
- Professional.
- Clean.
- Consistent.
- Information-rich without being crowded.
- Optimized for desktop productivity.

Avoid:
- Generic AI-generated dashboard designs.
- Excessive gradients.
- Excessive shadows.
- Too many rounded cards.
- Inconsistent spacing.
- Oversized typography.
- Unnecessary animations.
- Unprofessional colors.
- Inconsistent buttons and icons.

## 3. TYPOGRAPHY REDESIGN

Use Inter as the primary font.

Requirements:
- Bundle Inter locally for offline desktop operation.
- Use consistent font weights.
- Establish a complete typography scale.
- Improve readability of tables, forms and financial figures.
- Use tabular numbers for monetary values and quantities where appropriate.

Suggested typography:
- Page title: 24px / 600.
- Section heading: 16px / 600.
- Body: 14px / 400.
- Secondary text: 13px / 400.
- Table content: 13–14px.
- Small labels: 12px / 500.

Apply typography consistently across every screen.

## 4. ESTABLISH A DESIGN SYSTEM

Create centralized design tokens.

Define:
- Primary and secondary colors.
- Background colors.
- Surface colors.
- Text colors.
- Border colors.
- Success, warning and danger colors.
- Typography.
- Spacing.
- Border radii.
- Shadows.
- Focus states.

Use CSS variables compatible with the existing Tailwind and shadcn/ui setup.

Avoid creating different styling rules for the same component in different modules.

## 5. REDESIGN THE APPLICATION SHELL

Improve the overall desktop experience.

SIDEBAR:
- Professional navigation.
- Clear active navigation state.
- Consistent Lucide icons.
- Collapsible sidebar.
- Proper grouping of navigation items.
- Business logo and application identity.
- User information at the bottom.

HEADER:
- Page title.
- Breadcrumbs.
- Global search.
- Notification area if already supported.
- User profile.
- Consistent alignment.

CONTENT AREA:
- Consistent page margins.
- Proper maximum widths.
- Consistent vertical spacing.
- Responsive desktop layouts.
- Proper scrolling behavior.

## 6. REDESIGN ALL EXISTING SCREENS

Inspect every currently implemented page.

Improve:
- Dashboard.
- Login.
- Products.
- Categories.
- Inventory.
- Purchases.
- Point of Sale.
- Sales History.
- Customers.
- Suppliers.
- Expenses.
- Payments.
- Reports.
- Employees.
- Settings.

Only redesign pages and features that currently exist.

Do not create fake functionality just to fill empty pages.

## 7. TABLES AND DATA PRESENTATION

Tables are extremely important in this application.

Create a consistent professional table system.

Requirements:
- Clean headers.
- Proper column alignment.
- Readable typography.
- Consistent row heights.
- Subtle row separators.
- Good horizontal spacing.
- Clear status badges.
- Search and filtering areas.
- Pagination.
- Loading states.
- Empty states.
- Error states.

Right-align monetary amounts and quantities where appropriate.

Use tabular numerals for financial information.

## 8. FORMS AND DIALOGS

Standardize:
- Text inputs.
- Number inputs.
- Select fields.
- Date pickers.
- Search fields.
- Checkboxes.
- Radio buttons.
- Form labels.
- Validation messages.
- Dialogs.
- Confirmation dialogs.

Ensure forms are easy to understand and efficient for hardware-shop workers.

## 9. BUSINESS-SPECIFIC UX

Remember that this is not a generic SaaS dashboard.

It is hardware-shop management software.

The interface must accommodate:
- Cement bags.
- Steel bars.
- Electrical cables.
- Tiles.
- Paint.
- Pipes.
- Roofing sheets.
- Different measurement units.
- Large product catalogs.
- RWF currency.
- Daily business operations.

Prioritize fast product lookup, clear stock quantities and easy transaction workflows.

## 10. INTERACTION AND ACCESSIBILITY

Improve:
- Hover states.
- Focus states.
- Active states.
- Disabled states.
- Loading indicators.
- Keyboard navigation.
- Keyboard shortcuts where already supported.
- Tooltips for icon-only buttons.
- Consistent interaction feedback.

Make the application comfortable to use for several hours every day.

## 11. PERFORMANCE AND COMPATIBILITY

The application must remain lightweight.

Avoid:
- Unnecessary dependencies.
- Heavy animation libraries.
- Excessive component re-rendering.
- Loading unnecessary assets.
- Large background effects.

Ensure compatibility with Tauri 2 and Windows desktop WebView.

The application must continue to work offline.

## 12. STRICT DEVELOPMENT RULES

DO NOT:
- Change the database schema.
- Change business calculations.
- Change existing API contracts.
- Change application architecture.
- Delete existing features.
- Replace real data with hardcoded sample data.
- Break navigation.
- Introduce new business modules.
- Start the next development phase.

You are improving presentation and user experience only.

If you discover an existing UX problem that requires a business logic change, document it instead of silently changing the business logic.

## 13. IMPLEMENTATION WORKFLOW

Work in this order:

PHASE A — UI AUDIT
Inspect all existing screens and components.

Identify:
- Visual inconsistencies.
- Poor spacing.
- Typography problems.
- Inconsistent components.
- Navigation problems.
- Usability issues.
- Desktop layout problems.

PHASE B — DESIGN SYSTEM
Implement the typography, colors, spacing, component standards and reusable tokens.

PHASE C — APPLICATION SHELL
Redesign sidebar, header and main content layout.

PHASE D — PAGE REDESIGN
Improve every existing screen one by one.

PHASE E — QUALITY CONTROL
Run the application and inspect every redesigned page.

Check:
- No broken routes.
- No TypeScript errors.
- No missing styles.
- No broken functionality.
- No layout overflow.
- No inconsistent typography.
- No console errors.

## 14. REQUIRED DELIVERABLES

1. UI/UX audit report.
2. Centralized design system.
3. Updated typography.
4. Redesigned application shell.
5. Improved reusable components.
6. Redesigned existing pages.
7. Consistent interaction states.
8. Final UI quality-control report.

IMPORTANT FINAL INSTRUCTION:

Do not just suggest design improvements.

Inspect the actual codebase and implement the improvements directly.

Work systematically and preserve the existing working application.

Our goal is to have a production-quality interface before continuing to the next business development phase.