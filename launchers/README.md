# Golddig quick launchers

Double-clickable helpers for Windows. Open this folder in File Explorer.

| Launcher | What it does |
| --- | --- |
| **`create-shortcut.bat`** | Generates `Golddig.lnk` for this machine, with the app icon and the repository root as the working directory. Run this first. The shortcut is **not** committed — a `.lnk` stores an absolute path, so a committed one would point at whichever machine produced it. |
| **`Golddig.lnk`** | Created by the script above. Double-click to run `golddig.exe`; you can copy it to the Desktop or pin it to Start or the taskbar. |
| **`start-golddig.bat`** | Launches the desktop app, running `npm run tauri build` first if the binary is missing and reporting the failure if that build does not succeed. |
| **`start-dev-mode.bat`** | Starts `npm run tauri dev` — the native window with hot reload for both the frontend and the Rust core. |
| **`run-tests-and-benchmarks.bat`** | Runs the full verification sequence and keeps the console open: frontend typecheck, Vitest, `cargo fmt --check`, Clippy, the Rust test suite, then the latency benchmark. |

## First run

The app needs at least one dictionary pack. Only the tiny 7-entry test fixture ships with
the repository, so build a real one before expecting real results:

```sh
python scripts/build_all_packs.py ary    # ~6 MB, quickest way to see it working
python scripts/build_all_packs.py        # everything, ~7 GB of downloads
```

Packs are discovered at startup from the app's resource directory, its per-user data
directory, and the working directory. The status bar reports how many loaded and from where,
and names any pack that failed — it no longer claims success when nothing was found.
