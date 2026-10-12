# AIONE 4

A fullscreen, controller-first game launcher for Windows that feels like a console. Personal use, offline-first. Built with **Tauri 2** (Rust core) and **Svelte 5 + TypeScript**.

- Download: the latest **pre-release** on GitHub (`AIONE4-<version>-windows.zip`), unzip, run `AIONE4.exe`.
- Data: `%LOCALAPPDATA%\AIONE4\`.
- Documents: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/CHANGELOG.md](docs/CHANGELOG.md), [docs/KNOWN_ISSUES.md](docs/KNOWN_ISSUES.md), [docs/AIONE_DESIGN_SYSTEM.md](docs/AIONE_DESIGN_SYSTEM.md), [docs/THIRD_PARTY_ASSETS.md](docs/THIRD_PARTY_ASSETS.md).

## Development

Requirements: Node 22.12+ with npm 11, Rust 1.91.1 (`rust-toolchain.toml`), Windows 10/11 with WebView2 for running the app.

```
npm ci
npm run tauri dev        # run the app
npm run typecheck && npm run lint && npm test
npm run test:ui          # Playwright UI tests (mocked core), screenshots in ui-screenshots/
cd src-tauri && cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

CI (`.github/workflows/ci.yml`) builds and tests every push on Windows and publishes a pre-release from `main`. The version is set once, in `package.json`.
