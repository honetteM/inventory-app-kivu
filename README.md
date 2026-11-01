# Quincaillerie Manager

Windows desktop application for managing a hardware shop, built with Tauri, React, TypeScript, and SQLite.

## Development

- `npm run dev` starts the frontend.
- `npm run tauri dev` starts the desktop application with its local SQLite database.
- `npm run build` checks TypeScript and creates the frontend production build.
- `cargo test --manifest-path src-tauri/Cargo.toml` runs the Rust/database tests.

Business setup, units, categories, and brands are available from **Settings** in the desktop application. Product creation and the searchable, paginated catalogue are available under **Products**, where category and brand are optional. Products can also have additional selling units with conversion factors to their base unit. The **Products** page allows the base-unit selling price to be changed and records each price period in the price history; purchase costs remain separate. The **Inventory** receiving form shows the current selling price and can optionally update it while recording a delivery; the new price is added to price history and is used by POS, while leaving it blank keeps the current selling price. Each receipt records its cost per base unit and total cost; opening stock starts from the product's saved purchase price. The **Inventory** screen also supports reason-required counted-stock corrections and stock valuation using weighted-average purchase cost. The **Point of Sale** supports business-scoped product search, a base-unit cart, paid sales using cash or one supported electronic payment method, stock deduction, and an in-app receipt. Sales and their stock movements are written atomically, and receipt lines retain the listed and charged price; a changed per-sale price requires a reason and does not change the product's regular price. Cashier attribution, customer selection, credit, split payments, discounts, returns, and printable receipts are not implemented yet. The **Suppliers** page supports business-scoped supplier contact, address, and tax details, with editing and inactive status. Supplier purchase history, balances, payments, and invoice attachments will be added with the purchasing workflow. Stock receiving is recorded when goods arrive; purchase invoices and partial-delivery tracking are not implemented yet. Movement user attribution remains empty until login is implemented. Business records are stored in the local database; the frontend-only Vite preview cannot access these Tauri commands. Product creation and receipt unit costs are entered in whole RWF; saved money uses the database's hundredths-of-RWF scale, and selling-price edits support cents.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
