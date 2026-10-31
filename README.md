# Quincaillerie Manager

Windows desktop application for managing a hardware shop, built with Tauri, React, TypeScript, and SQLite.

## Development

- `npm run dev` starts the frontend.
- `npm run tauri dev` starts the desktop application with its local SQLite database.
- `npm run build` checks TypeScript and creates the frontend production build.
- `cargo test --manifest-path src-tauri/Cargo.toml` runs the Rust/database tests.

Business setup, units, categories, and brands are available from **Settings** in the desktop application. Product creation and the searchable, paginated catalogue are available under **Products**, where category and brand are optional. Products can also have additional selling units with conversion factors to their base unit. The **Products** page allows the base-unit selling price to be changed and records each price period in the price history; purchase costs remain separate. The **Inventory** screen supports one-time opening stock, delivery/invoice-referenced stock receiving, reason-required counted-stock corrections, and stock valuation using weighted-average purchase cost. Each receipt records its cost per base unit and total cost; opening stock starts from the product's saved purchase price. The **Point of Sale** currently provides business-scoped product search and a base-unit cart with selling prices and stock availability; checkout, cashier identity, payment, and receipts are not implemented yet. Receiving is recorded when goods arrive; full purchase invoices, supplier records, and partial-delivery tracking are not implemented yet. Movement user attribution remains empty until login is implemented. Business records are stored in the local database; the frontend-only Vite preview cannot access these Tauri commands. Product creation and receipt unit costs are entered in whole RWF; saved money uses the database's hundredths-of-RWF scale, and selling-price edits support cents.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
