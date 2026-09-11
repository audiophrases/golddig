# Golddig Quick Launchers

This folder contains convenient, double-clickable script launchers for Windows:

| Launcher File | What It Does |
|---|---|
| **`start-golddig.bat`** | Double-click to instantly run the native desktop application (`golddig.exe`). If the binary doesn't exist yet, it automatically triggers `npm run tauri build`. |
| **`start-dev-mode.bat`** | Double-click to start the live interactive dev environment (`npm run tauri dev`) with hot module reloading for frontend and backend. |
| **`run-tests-and-benchmarks.bat`** | Double-click to execute all verification suites: Svelte/TS typecheck, Vitest frontend tests, 16 Rust backend/importer unit tests, and the 1,000-lookup latency benchmark. Keeps the terminal open with results. |
