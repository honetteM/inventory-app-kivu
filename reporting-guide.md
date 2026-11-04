# TASK: Audit Existing Application and Implement a Professional Business Reporting & PDF System

Act as a senior software engineer, financial reporting specialist, database auditor, and professional UI/UX designer.

I have an existing Quincaillerie Management application. The application is already working well, and the necessary business information and transaction details are already available.

Your responsibility is to thoroughly audit the existing application and use its existing data, functionality, and business rules to implement a powerful, accurate, professional, and easy-to-understand PDF reporting system.

Do not unnecessarily rebuild working functionality.

The reporting system must support:

1. Daily Business Report
2. Weekly Business Report
3. Monthly Business Report
4. Yearly Business Report

The PDF should be inspired by the supplied Musanze Branch report, but significantly improve its readability, organization, visual hierarchy, and explanations for business owners without financial knowledge.

# PHASE 1: COMPLETE APPLICATION AUDIT — DO THIS FIRST

Before implementing anything, inspect the entire relevant application.

The application already contains the business data needed for reporting. Discover it instead of creating duplicate functionality.

Audit the following:

### A. Database and data models
Inspect:
- Sales and sales payments.
- Purchase records and purchase payments.
- Expenses.
- Cash transactions.
- Opening and closing cash records, if available.
- Bank deposits and withdrawals.
- Mobile Money transactions.
- Loans received.
- Loan repayments.
- Outstanding loans.
- Prepaid cash.
- Customer payments and debts.
- Supplier payments and debts.
- Branch information.
- Payment methods.
- Existing financial reports and calculations.

Identify the relationships between all these records.

### B. Existing business logic

Inspect the existing:
- Financial calculation services.
- Transaction recording processes.
- Cash management functionality.
- Payment methods and payment classification.
- Daily closing procedures.
- Existing report generation.
- Inventory and purchasing calculations.

Determine exactly how the application currently calculates and stores financial balances.

### C. Audit deliverable

Before making changes, provide a concise internal implementation assessment containing:

1. Existing data sources available for reporting.
2. Existing financial calculations that can be reused.
3. Missing information, if any.
4. Potential double-counting risks.
5. Recommended reporting architecture.
6. Existing PDF or printing functionality that can be reused.

Do not ask me to manually provide information already available in the application.

Do not stop at the audit. After identifying the existing architecture and data sources, proceed with implementation.

# PHASE 2: FINANCIAL RULES AND REPORTING LOGIC

These rules are mandatory.

## A. DAILY REPORT — PHYSICAL CASH MANAGEMENT

The daily report must show the actual physical cash situation of the business.

### Opening cash

Opening cash is the actual physical money remaining in the cash box at the end of the previous business day.

This amount becomes the opening cash for the next business day.

Examples:

- Yesterday's closing physical cash: RWF 15,000.
- Today's opening cash: RWF 15,000.

If the business deposits all remaining physical cash into the bank or transfers it to Mobile Money, the next day's opening cash must be zero.

If no physical cash was carried forward, opening cash is zero.

The application must use the existing daily closing records or cash management records as the source of truth.

Do not calculate opening cash from total business assets, bank balances, or Mobile Money balances.

Do not automatically assume that yesterday's total ending business balance is today's opening physical cash.

### Money received

Show actual money received during the day.

Use the existing transaction records and payment methods.

Where useful, distinguish:
- Cash received.
- Bank payments received.
- Mobile Money received.
- Other supported payment methods.

Avoid counting the same payment more than once.

### Outflow / expenses

For this reporting system:

TOTAL OUTFLOW = ACTUAL BUSINESS EXPENSES RECORDED DURING THE PERIOD.

Show expenses clearly by category.

Do not automatically classify bank deposits, Mobile Money transfers, or transfers between business accounts as expenses.

Do not mix supplier payments, loan repayments, or other financial movements with operating expenses unless the existing business classification explicitly identifies them as expenses.

Show other payment activities separately where necessary.

### Net position

NET POSITION = TOTAL MONEY RECEIVED - TOTAL EXPENSES.

Explain this in simple language.

It represents the difference between money received and expenses paid during the period.

It does not automatically represent business profit.

### Physical closing cash

Calculate actual physical cash remaining using the existing cash transactions and payment methods.

The reconciliation should follow:

PHYSICAL CLOSING CASH =
OPENING PHYSICAL CASH
+ CASH RECEIVED
- CASH EXPENSES PAID IN CASH
- CASH TRANSFERRED TO BANK
- CASH TRANSFERRED TO MOBILE MONEY
- OTHER VERIFIED CASH WITHDRAWALS
+ VERIFIED CASH RETURNS.

Use actual transaction classifications.

Important:

Net position and physical closing cash are different measurements.

Do not force them to have the same value.

For example, when cash is deposited into the bank, the physical cash decreases but the business has not incurred an expense.

If the application has a different established cash-accounting implementation, audit it first and preserve the correct existing business rules.

### Daily report must include:

- Opening physical cash.
- Total money received.
- Total expenses.
- Net position.
- Physical cash remaining.
- Cash transferred to bank.
- Cash transferred to Mobile Money.
- Loans received.
- Loan repayments.
- Outstanding loans.
- Deposits and prepaid cash, where applicable.

Clearly separate business cash, account transfers, and liabilities.

## B. WEEKLY REPORT

The weekly report is a business performance summary.

Use Monday to Sunday as the default reporting week.

IMPORTANT: Do not display Opening Cash or Physical Closing Cash in weekly reports.

These are daily physical cash management figures.

Include:

- Total money received during the week.
- Total expenses during the week.
- Weekly net position.
- Sales performance.
- Expense breakdown.
- Loan activity.
- Outstanding loans at the end of the reporting period.
- Bank and Mobile Money movements, separately where useful.

NET POSITION = TOTAL INFLOW - TOTAL EXPENSES.

Include a daily performance table:

Date | Money Received | Expenses | Net Position

Identify:
- Day with the highest money received.
- Day with the highest expenses.
- Best net-position day.
- Total weekly business activity.

Do not invent weekly opening or closing cash balances.

## C. MONTHLY REPORT

This is a monthly business performance report.

Do not display Opening Cash or Physical Closing Cash.

Include:

- Total money received.
- Total expenses.
- Monthly net position.
- Total sales received.
- Expense breakdown.
- Loan activity.
- Outstanding loans.
- Bank and Mobile Money movements separately where useful.

NET POSITION = TOTAL INFLOW - TOTAL EXPENSES.

Provide weekly performance summaries.

Include:
- Highest-performing week by money received.
- Week with the highest expenses.
- Average daily money received.
- Average daily expenses.
- Main expense categories.
- Monthly performance explanation.

## D. YEARLY REPORT

This is an annual business performance report.

Do not display Opening Cash or Physical Closing Cash.

Include:

- Total yearly money received.
- Total yearly expenses.
- Yearly net position.
- Total sales received.
- Main expense categories.
- Total loans received.
- Total loan repayments.
- Outstanding loans at year-end.
- Bank and Mobile Money movements separately where useful.

NET POSITION = TOTAL INFLOW - TOTAL EXPENSES.

Include a monthly performance table:

Month | Money Received | Expenses | Net Position

Include:
- Best month by money received.
- Month with the highest expenses.
- Average monthly money received.
- Average monthly expenses.
- Overall annual business performance.

# PHASE 3: REPORT CONTENT AND PDF STRUCTURE

Create a professional, readable, multi-page PDF.

The report should be understandable to a shop owner without accounting knowledge.

## Page 1: Business Performance Overview

Include:

- Business logo, if available.
- Business name.
- Branch name.
- Report type.
- Reporting period.
- Report generation date.

For weekly, monthly, and yearly reports, display these summary cards:

1. Total Money Received.
2. Total Expenses.
3. Net Position.
4. Total Sales Received, if available.
5. Outstanding Loans.

For daily reports, display:

1. Opening Physical Cash.
2. Total Money Received.
3. Total Expenses.
4. Net Position.
5. Physical Cash Remaining.

Clearly explain each figure in ordinary language.

## Page 2: Where the Money Came From

Show the incoming money breakdown using actual recorded categories.

Examples:
- Sales received.
- Customer payments.
- Other incoming transactions.

Show the amount and percentage where meaningful.

Do not duplicate payments already included in sales totals.

## Page 3: Where the Money Went

Show the expense breakdown.

Include:
- Expense category.
- Description.
- Amount.
- Percentage of total expenses.

Make it easy to understand which activities consumed the most money.

Keep non-expense financial movements separate.

## Page 4: Cash Management and Financial Obligations

For daily reports, explain:
- Opening physical cash.
- Cash received.
- Cash expenses.
- Cash transferred to bank.
- Cash transferred to Mobile Money.
- Physical cash remaining.

For all reports, where data exists, explain:
- Loans received.
- Loans repaid.
- Outstanding loans.
- Deposits.
- Prepaid cash.

Do not mix liabilities with available cash.

## Page 5 and Beyond: Business Performance Details

Include appropriate tables depending on the reporting period.

- Weekly report: daily breakdown.
- Monthly report: weekly breakdown.
- Yearly report: monthly breakdown.
- Daily report: detailed daily transaction and cash movement breakdown.

Repeat table headers across PDF pages.

Ensure readable font sizes and proper pagination.

## Final Section: Management Insights

Generate simple, factual explanations from actual figures.

Examples:

"Your business received RWF X during this period."

"Your recorded expenses were RWF X."

"Your net position was positive by RWF X."

"Your highest expense category was X."

"Your physical cash remaining at the end of the day was RWF X."

Do not claim that a business made a profit based only on net cash position.

Never generate unsupported recommendations.

# PHASE 4: PROFESSIONAL DESIGN

Use a modern business-report design.

Requirements:

- A4 paper.
- White background.
- Professional typography.
- Dark navy or charcoal headings.
- Clear financial summary cards.
- Consistent spacing.
- Professional tables.
- Subtle separators.
- Green for positive net position.
- Red for negative net position.
- Neutral styling for other values.
- RWF currency formatting based on the configured business currency.
- Proper negative-number formatting.
- Business logo where available.
- Page numbers.
- Business and branch identification in footers.
- Print-friendly grayscale compatibility.

Avoid unnecessary decorative graphics and overcrowded layouts.

The PDF should look like a professional business management report, not a raw database export.

# PHASE 5: REPORTING INTERFACE

Integrate reporting into the existing application.

Allow users to select:

- Daily.
- Weekly.
- Monthly.
- Yearly.

Provide the appropriate date selection for each report.

Include:

- Preview Report.
- Export PDF.
- Print Report.

The preview and exported PDF must use exactly the same calculation service and data.

Do not introduce unnecessary navigation changes or disrupt existing workflows.

# PHASE 6: ACCURACY, RECONCILIATION AND TESTING

Test the reports using actual application records.

Validate:

1. Daily opening cash matches the previous day's physical closing cash.
2. If no cash was carried forward, opening cash is zero.
3. Bank and Mobile Money transfers are not incorrectly treated as expenses.
4. Daily physical closing cash reconciles with actual cash transactions.
5. Net position equals money received minus recorded expenses.
6. Weekly totals reconcile with daily transactions.
7. Monthly totals reconcile with weekly and underlying transactions.
8. Yearly totals reconcile with monthly and underlying transactions.
9. Outstanding loans reflect all unpaid loans at the reporting date.
10. No transaction is counted twice.
11. Empty reporting periods are handled correctly.
12. Historical reports remain accurate.
13. The report preview and PDF contain identical figures.

Add automated tests for all important financial formulas.

Use realistic test data and verify calculations independently.

Do not use fabricated production data.

# PHASE 7: PERFORMANCE AND RELIABILITY

Keep the existing application fast.

- Use efficient database aggregation.
- Avoid loading unnecessary transactions into memory.
- Reuse existing services.
- Avoid repeated database queries.
- Use efficient date filtering.
- Avoid blocking the desktop interface during PDF generation.
- Handle errors gracefully.
- Ensure exported PDFs are complete and readable.

# FINAL IMPLEMENTATION INSTRUCTIONS

The existing application is already working. Treat this as an enhancement, not a complete rebuild.

Do not:
- Rewrite existing working modules unnecessarily.
- Change existing financial transaction behavior without a demonstrated need.
- Create duplicate financial records.
- Invent missing data.
- Treat bank deposits as expenses.
- Treat Mobile Money transfers as expenses.
- Show physical opening cash in weekly, monthly, or yearly reports.
- Confuse net position with accounting profit.

Complete the implementation, not just the design.

At the end, provide:

1. Summary of the application audit.
2. Existing data sources reused.
3. Files created and modified.
4. Financial formulas implemented.
5. PDF design and report sections completed.
6. Tests performed and their results.
7. Any missing data or limitations discovered.

The final result must be an accurate, powerful, readable, professional business reporting system that works with the existing application.