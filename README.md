# Quincaillerie Manager

Windows desktop application for managing a hardware shop, built with Tauri, React, TypeScript, and SQLite.

## Development

- `npm run dev` starts the frontend.
- `npm run tauri dev` starts the desktop application with its local SQLite database.
- `npm run build` checks TypeScript and creates the frontend production build.
- `cargo test --manifest-path src-tauri/Cargo.toml` runs the Rust/database tests.

Business setup is available from **Settings** in the desktop application. Business records are stored in the local database; the frontend-only Vite preview cannot access these Tauri commands.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
