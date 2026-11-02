# QUINCAILLERIE MANAGER

## Master Development Guide & Technical Specification

**Product:** Quincaillerie Manager
**Version:** 1.0
**Platform:** Windows Desktop Application
**Future platform:** Android and mobile devices
**Target market:** Hardware shops, building-material suppliers and construction-material distributors in Rwanda.

---

# 1. PRODUCT OBJECTIVE

Build a professional desktop application that helps hardware-shop owners manage their entire business from one system.

The application must manage:

- Products and product categories.
- Inventory and stock movements.
- Purchasing and supplier invoices.
- Sales and receipts.
- Customers and customer debts.
- Supplier balances and payments.
- Business expenses.
- Employees and permissions.
- Financial calculations.
- Daily, weekly and monthly reports.
- Business performance.

The application must be simple for shop workers but provide advanced financial control for the owner.

## Core principles

1. Speed: operations should be quick, even with thousands of products.
2. Accuracy: stock and financial calculations must be reliable.
3. Simplicity: minimize manual data entry.
4. Accountability: every important action must have a responsible user.
5. Security: users must only access authorized functions.
6. Reliability: never lose a completed transaction.
7. Scalability: prepare for future mobile and multi-branch support.

---

# 2. TECHNOLOGY STACK

Use the following stack.

| Layer             | Technology                                    |
| ----------------- | --------------------------------------------- |
| Desktop framework | Tauri 2                                       |
| Frontend          | React + TypeScript                            |
| Build tool        | Vite                                          |
| Styling           | Tailwind CSS                                  |
| UI components     | shadcn/ui                                     |
| Icons             | Lucide                                        |
| Database          | SQLite                                        |
| Database access   | SQLx or a maintained Tauri SQLite integration |
| Data validation   | Zod                                           |
| Frontend state    | Zustand                                       |
| Forms             | React Hook Form                               |
| Tables            | TanStack Table                                |
| Charts            | Recharts                                      |
| PDF reports       | PDF generation library                        |
| Testing           | Vitest + Playwright                           |
| Packaging         | Tauri bundler                                 |

### Why this architecture?

Tauri is a good choice for a desktop application because it uses the operating system's WebView instead of bundling a full Chromium browser.

SQLite provides a fast local database without requiring the shop to install or maintain a database server.

React makes it easier to reuse the interface when mobile development begins.

**Important architectural requirement:** Separate the business logic, data access and user interface. Never place inventory calculations directly inside React components.

Build an application service layer that can later be exposed through a cloud API.

---

# 3. APPLICATION STRUCTURE

Create the following project structure:

```text
quincaillerie-manager/
│
├── src/
│   ├── app/
│   │   ├── routes/
│   │   ├── providers/
│   │   └── layouts/
│   │
│   ├── components/
│   │   ├── ui/
│   │   ├── tables/
│   │   ├── forms/
│   │   └── shared/
│   │
│   ├── modules/
│   │   ├── dashboard/
│   │   ├── products/
│   │   ├── categories/
│   │   ├── inventory/
│   │   ├── purchasing/
│   │   ├── sales/
│   │   ├── customers/
│   │   ├── suppliers/
│   │   ├── expenses/
│   │   ├── payments/
│   │   ├── employees/
│   │   ├── reports/
│   │   └── settings/
│   │
│   ├── services/
│   │   ├── inventory.service.ts
│   │   ├── sales.service.ts
│   │   ├── purchase.service.ts
│   │   ├── finance.service.ts
│   │   ├── reporting.service.ts
│   │   └── backup.service.ts
│   │
│   ├── repositories/
│   │   ├── product.repository.ts
│   │   ├── inventory.repository.ts
│   │   ├── sales.repository.ts
│   │   └── purchase.repository.ts
│   │
│   ├── lib/
│   │   ├── calculations/
│   │   ├── validation/
│   │   ├── formatting/
│   │   └── permissions/
│   │
│   └── types/
│
├── src-tauri/
│   ├── src/
│   ├── migrations/
│   ├── capabilities/
│   └── tauri.conf.json
│
├── tests/
│   ├── unit/
│   ├── integration/
│   └── e2e/
│
├── docs/
│   ├── architecture.md
│   ├── database.md
│   ├── business-rules.md
│   └── testing.md
│
└── README.md
```

Keep modules independent. One module should not directly manipulate another module's database records.

---

# 4. APPLICATION NAVIGATION

Create a professional desktop layout with a persistent left sidebar.

## Main navigation

1. Dashboard
2. Point of Sale
3. Products
4. Inventory
5. Purchases
6. Sales History
7. Customers
8. Suppliers
9. Expenses
10. Payments
11. Reports
12. Employees
13. Settings

The interface should support a collapsible sidebar and keyboard shortcuts.

The default language should be English, with the architecture prepared for Kinyarwanda and French translations.

Currency: RWF.

Date format: DD/MM/YYYY.

Use configurable business timezone, defaulting to Africa/Kigali.

---

# 5. DASHBOARD

The dashboard must provide an immediate understanding of business performance.

## Financial cards

- Today's sales.
- Today's gross profit.
- Today's expenses.
- Today's net operating result.
- Cash received.
- Mobile Money received.
- Bank payments.
- Outstanding customer debts.
- Outstanding supplier balances.

## Inventory cards

- Total active products.
- Total stock valuation.
- Low-stock products.
- Out-of-stock products.
- Products requiring reordering.

## Dashboard charts

- Sales over the last seven days.
- Sales by payment method.
- Best-selling products.
- Top product categories.
- Monthly revenue and expenses.

## Recent activity

Display:

- Recent sales.
- Recent purchases.
- Recent expenses.
- Recent stock adjustments.
- Recent payments.

All dashboard figures must come from database queries and must not be hardcoded.

---

# 6. PRODUCT MANAGEMENT

This is one of the most important modules because hardware shops have many different types of materials.

Each product should contain:

- Product name.
- Internal SKU.
- Barcode (optional).
- Category.
- Brand.
- Description.
- Size or dimensions.
- Colour.
- Product image (optional).
- Base measurement unit.
- Selling units.
- Purchase price.
- Selling price.
- Minimum stock quantity.
- Current quantity.
- VAT classification.
- Active/inactive status.

## Hardware-specific examples

| Product          | Base unit    | Possible selling unit       |
| ---------------- | ------------ | --------------------------- |
| Cement           | Bag          | Bag                         |
| Steel bars       | Piece        | Bundle or piece             |
| Electrical cable | Metre        | Roll or metre               |
| Tiles            | Square metre | Box or square metre         |
| PVC pipes        | Piece        | Piece or cut length         |
| Nails            | Kilogram     | Kilogram or smaller package |
| Paint            | Litre        | Bucket or litre             |
| Roofing sheets   | Piece        | Piece                       |

### Unit conversion

A product can have several selling units.

Example:

Product: Electrical cable

- One roll = 100 metres.
- One metre = 1 base unit.

If the business receives 10 rolls:

Stock = 1,000 metres.

If a worker sells 15 metres:

Remaining stock = 985 metres.

The system must use conversion factors and decimal-safe quantities.

Never assume every product is sold by pieces.

### Product search

Implement fast search by:

- Name.
- SKU.
- Barcode.
- Brand.
- Category.
- Product specifications.

Support keyboard navigation and barcode scanners.

Use database indexes instead of loading all products into memory.

---

# 7. INVENTORY MANAGEMENT

Inventory must be based on a stock movement ledger.

Do not allow users to simply overwrite stock quantities without recording the reason.

## Movement types

- Opening stock.
- Purchase received.
- Sale.
- Customer return.
- Supplier return.
- Damaged stock.
- Stock correction.
- Stock transfer.
- Manual adjustment.

Every movement should contain:

- Product ID.
- Quantity.
- Movement type.
- Reference transaction.
- Previous quantity.
- New quantity.
- User ID.
- Date and time.
- Reason, when applicable.

## Inventory calculations

Closing stock:

Opening stock + received stock - sold stock - supplier returns - damaged stock + customer returns + adjustments.

## Stock control features

- Low-stock alerts.
- Out-of-stock alerts.
- Stock valuation.
- Product movement history.
- Stock adjustment approval.
- Physical stock counting.
- Stock discrepancy reports.
- Fast stock lookup.

Support optional negative-stock restrictions. The default should prevent sales beyond available stock.

---

# 8. PURCHASING AND SUPPLIERS

The owner is primarily responsible for purchasing.

## Purchase workflow

1. Select supplier.
2. Enter supplier invoice number.
3. Add products.
4. Enter quantities and purchase units.
5. Enter unit costs and discounts.
6. Enter applicable tax information.
7. Enter transport or additional purchase costs.
8. Record payment method and amount paid.
9. Save the purchase.
10. Confirm stock receipt.

Stock must only increase when the goods are actually received.

Support partial deliveries.

## Supplier management

Record:

- Supplier name.
- Contact details.
- Address.
- Tax identification information, where applicable.
- Purchase history.
- Outstanding balances.
- Payment history.

Allow the user to attach invoice photographs or PDF files.

Do not allow duplicate supplier invoices without an explicit duplicate warning and authorization.

---

# 9. SALES AND POINT OF SALE

The POS must be extremely fast.

This is where employees spend most of their time.

## Sales workflow

1. Search or scan product.
2. Add product to cart.
3. Enter quantity.
4. Select selling unit.
5. Apply authorized discount.
6. Select customer or walk-in customer.
7. Select payment method.
8. Confirm sale.
9. Update stock.
10. Generate receipt.

## Payment methods

- Cash.
- MTN Mobile Money.
- Airtel Money.
- Bank transfer.
- Credit.
- Split payments.

Support multiple payment methods for a single transaction.

## Sales rules

- Sales must have unique transaction numbers.
- Sales must record the responsible worker.
- Every sale must update inventory.
- Discounts must be recorded.
- Returns must reference the original sale when possible.
- Cancelled transactions must remain in the audit history.
- Credit sales must create customer debt records.
- Receipts must clearly show payment status.

If a customer requires an official tax invoice, use the authorized EBM process. Do not represent an ordinary application receipt as a certified EBM invoice.

---

# 10. CUSTOMER MANAGEMENT

Store:

- Customer name.
- Phone number.
- Address.
- Customer type.
- Credit limit.
- Outstanding balance.
- Transaction history.

## Credit management

When a customer buys on credit:

- Record the sale.
- Reduce inventory.
- Record revenue and cost of goods sold.
- Create an outstanding receivable.
- Record any initial payment.
- Track future repayments.

Provide:

- Customer debt statements.
- Payment reminders.
- Overdue account reports.
- Customer transaction history.

A customer repayment must never create a second sale.

---

# 11. EXPENSE MANAGEMENT

Allow authorized users to record:

- Rent.
- Electricity.
- Transport.
- Salaries.
- Internet.
- Repairs.
- Security.
- Other operational expenses.

Every expense should include:

- Expense category.
- Amount.
- Date.
- Payment method.
- Description.
- Responsible user.
- Supporting document, if available.

Distinguish operational expenses from stock purchases, owner withdrawals and other balance-sheet transactions.

Do not deduct stock purchases from profit a second time when their cost has already been recognized through cost of goods sold.

---

# 12. FINANCIAL ENGINE

All financial calculations must be centralized in the financial service layer.

Use integer RWF amounts or an appropriate precise decimal representation. Never use JavaScript floating-point arithmetic for financial calculations.

## Core formulas

Gross profit:

Net sales - Cost of goods sold.

Net operating result:

Gross profit - Operating expenses.

Customer receivables:

Credit sales - Customer repayments - Authorized adjustments.

Supplier payables:

Unpaid supplier invoices - Supplier payments - Authorized adjustments.

## Inventory costing

Use FIFO purchase-cost layers for inventory valuation and cost of goods sold.

Every received purchase creates a cost layer containing the received quantity and actual purchase cost. When stock is sold or removed, consume the oldest remaining layers first. Value on-hand stock as the sum of each remaining layer's quantity multiplied by its recorded unit cost.

For legacy stock, reconstruct remaining layers from the stock-movement ledger. Use the existing average cost only for quantities that cannot be matched to recorded movements.

Historical sales costs must never change simply because the purchase price changes later.

## VAT configuration

Build a configurable tax engine.

Support:

- Tax-inclusive prices.
- Tax-exclusive prices.
- Exempt products.
- Applicable tax rates.
- Tax breakdowns on reports.
- Different tax treatments by product.

Do not hardcode an assumed VAT treatment for every item.

Investigate current RRA requirements and the applicable EBM integration and certification pathway before enabling official tax invoicing.

---

# 13. DAILY BUSINESS REPORT

Generate an automatic PDF report at the configured closing time, default 23:00.

The report must include:

1. Business name and branch.
2. Reporting date.
3. Opening cash.
4. Total sales.
5. Sales by payment method.
6. Credit sales.
7. Customer repayments.
8. Expenses.
9. Supplier payments.
10. Purchases received.
11. Gross profit.
12. Operating result.
13. Products sold.
14. Stock adjustments.
15. Closing cash.
16. Outstanding customer debts.
17. Outstanding supplier balances.
18. Worker performance.
19. Cash discrepancies.

The system must calculate closing cash using actual cash movements.

Purchases, sales, expenses and repayments must not be mixed into one simple cash calculation.

Allow reports to be regenerated after authorized corrections, while preserving the original audit history.

### Current implementation status

The first reporting slice is a manually selected, business-scoped daily summary. It includes completed sales and payment methods, unpaid credit sales, customer repayments, recorded expenses, supplier payments, received purchases, FIFO cost of goods sold, gross profit, operating result, current customer and supplier balances, and stock-adjustment event counts.

Cash ledger migration 015 adds distinct cash, bank and MoMo accounts, immutable classified movements, and confirmed transfers that post paired source/destination movements. It distinguishes customer trade-debt repayments from business-loan movements. Existing financial records are intentionally not backfilled because their actual custody account, branch and responsible user are unknown.

Local owner sign-in now uses Argon2id password hashes and an in-memory session. The first owner can bootstrap a new or existing business; creating another business provisions the same owner credentials in that business. Application data commands require a signed-in owner and are scoped to that owner's selected business.

Worker account management and granular role permissions; posting existing sale, repayment, expense and supplier-payment flows into cash accounts; account management UI; authenticated cash approvals; opening/closing balances and carry-forward; branch attribution; detailed adjustment reporting; and PDF export remain pending. Cash closing remains disabled until role-specific permission checks and the closing workflow are implemented. Timestamp-based report activity currently follows the computer's local timezone; applying each business's configured timezone is also pending.

---

# 14. USERS AND PERMISSIONS

Implement role-based access control.

## Owner / Administrator

Full access:

- Business settings.
- Products.
- Inventory.
- Purchases.
- Sales.
- Financial reports.
- Employees.
- Permissions.
- Backups.
- Audit logs.

## Manager

- Manage stock.
- Manage purchases.
- Manage sales.
- Manage customers.
- View permitted reports.
- Approve authorized adjustments

* Access POS.
* Search products.
* Record sales.
* Record authorized customer payments.
* Print receipts.
* View permitted stock information.

Workers must not automatically have access to:

- Purchase costs.
- Business profit.
- Owner withdrawals.
- Sensitive financial reports.
- Employee permissions.

Every important action must record the authenticated user.

---

# 15. DATABASE DESIGN

Create normalized relational tables.

Minimum tables:

### Business and security

- businesses
- branches
- users
- roles
- permissions
- user_roles
- audit_logs

### Products

- products
- categories
- brands
- units
- product_units
- product_prices

### Inventory

- inventory_balances
- stock_movements
- stock_counts
- stock_count_items

### Purchasing

- suppliers
- purchases
- purchase_items
- supplier_payments

### Sales

- customers
- sales
- sale_items
- sale_payments
- sales_returns
- sales_return_items

### Finance

- expenses
- expense_categories
- customer_payments
- cash_accounts
- cash_movements
- tax_rates

### System

- attachments
- document_sequences
- settings
- backup_history
- schema_migrations

Use UUIDs for business entities to prepare for synchronization with future mobile applications.

Include created_at, updated_at and appropriate status fields.

Use foreign keys, database constraints and indexes.

Add database migrations from the first development day.

---

# 16. PERFORMANCE REQUIREMENTS

The application must feel immediate.

Target requirements:

- Application startup: ideally under 3 seconds on a suitable modern computer.
- Product search: under 200 ms for normal indexed searches.
- POS operations: immediate visual feedback.
- Dashboard: avoid unnecessary full-table scans.
- Reports: generate asynchronously for large datasets.
- Support at least 20,000 products without loading all products simultaneously.
- Paginate large transaction histories.
- Use virtualized tables for large datasets.
- Avoid excessive React re-rendering.
- Keep database transactions short.
- Add appropriate SQLite indexes.
- Enable WAL mode where appropriate.
- Use foreign-key enforcement.
- Use prepared statements.
- Avoid expensive calculations on every component render.

Do not sacrifice transaction integrity for speed.

---

# 17. BACKUP AND SECURITY

This is financial business data. Protect it.

Implement:

- Password hashing with a modern password-hashing algorithm.
- Secure session handling.
- Automatic database backups.
- Manual backup and restore.
- Encrypted backup files.
- Database integrity checks.
- Audit logs.
- Protected application configuration.
- Safe attachment storage.
- Permission checks in the backend, not only the UI.

Back up before database migrations.

Do not expose the SQLite database through an unrestricted filesystem path.

Do not allow workers to delete completed transactions.

Use reversal transactions for corrections.

---

# 18. FUTURE MOBILE EXPANSION

Do not build mobile functionality in the first release.

However, prepare for it now.

The architecture must separate:

- User interface.
- Application services.
- Business rules.
- Database repositories.

When mobile development begins, introduce a centralized backend API and PostgreSQL server.

The desktop application can then synchronize its local data with the server.

The mobile application will use the same business rules through that API.

Plan for:

- Unique transaction identifiers.
- Idempotent synchronization.
- Conflict detection.
- Offline transaction queues.
- User and device identification.
- Audit trails.
- Secure authentication.

Do not assume that SQLite databases can simply be shared between multiple computers. Each device will eventually need its own local data store and synchronization mechanism.

---

# 19. DEVELOPMENT PHASES

## Phase 1 — Working foundation

Build:

- Desktop application shell.
- Login.
- Business configuration.
- User permissions.
- Database.
- Product management.
- Categories.
- Units and conversions.

**Completion condition:** The owner can create products and manage their information.

## Phase 2 — Inventory

Build:

- Stock ledger.
- Opening stock.
- Stock receiving.
- Stock adjustments.
- Low-stock alerts.
- Stock valuation.

**Completion condition:** Every stock change is traceable.

## Phase 3 — Sales and purchasing

Build:

- POS.
- Shopping cart.
- Receipts.
- Purchase invoices.
- Supplier management.
- Customer management.
- Credit management.

**Completion condition:** A real purchase and sale can be completed correctly.

## Phase 4 — Finance and reporting

Build:

- Expense tracking.
- Customer repayments.
- Supplier payments.
- Financial calculations.
- Cash reconciliation.
- Daily PDF reports.

**Completion condition:** The owner can understand the financial position of the business.

## Phase 5 — Production readiness

Build:

- Automated backups.
- Restore testing.
- Performance testing.
- Security testing.
- Installer.
- Error logging.
- User documentation.

**Completion condition:** The software can be installed and used reliably in a real quincaillerie.

---

# 20. TESTING REQUIREMENTS

The coding agent must write tests for important business operations.

Test these scenarios:

1. Purchase 100 bags of cement.
2. Sell 15 bags.
3. Return 2 bags from a customer.
4. Record 5 damaged bags.
5. Confirm the remaining quantity.
6. Purchase the same product at a different cost.
7. Verify FIFO stock value and cost of goods sold across purchase layers.
8. Make a partially paid sale.
9. Record customer repayment.
10. Verify outstanding customer debt.
11. Record a supplier payment.
12. Reconcile closing cash.
13. Cancel a sale through an authorized reversal.
14. Restart the application and verify persisted records.
15. Restore a backup and verify data integrity.

All these tests must pass before the first production deployment.

---

# 21. INSTRUCTIONS FOR THE AI CODING AGENT

Act as a senior software architect, senior full-stack developer, database engineer, financial systems engineer and professional UI/UX designer.

Follow these rules:

1. Read this complete guide before starting.
2. Build a working product, not merely a collection of interface mockups.
3. Implement the database and business logic before connecting the screens.
4. Use reusable components and modular services.
5. Do not duplicate business calculations.
6. Never hardcode dashboard values.
7. Never lose stock movement history.
8. Never use floating-point arithmetic for money.
9. Validate all transactions.
10. Add automated tests as each module is implemented.
11. Do not proceed to the next phase while critical tests are failing.
12. Keep the interface clean, modern and professional.
13. Use realistic quincaillerie sample data for development.
14. Keep the application usable without an internet connection.
15. Maintain documentation as the application evolves.

### Required deliverables

- Complete source code.
- Database migrations.
- Working Windows desktop application.
- Installer.
- Sample business data.
- Automated tests.
- Backup and restore functionality.
- Technical documentation.
- User guide.

---

# 22. DEFINITION OF SUCCESS

The first version is successful when a real hardware shop can:

- Register all its products.
- Record opening stock.
- Receive purchases.
- Sell products.
- Manage different product units.
- Track customer debts.
- Track supplier payments.
- Record expenses.
- Monitor workers.
- Calculate profit accurately.
- Reconcile cash.
- Generate daily PDF reports.
- Back up and restore business data.

The application must be reliable enough to become the primary operational system for the shop.

**Final development philosophy: Build the smallest complete business-management system first. Make every transaction accurate. Make every screen fast. Expand only after the core product works in a real shop.**
