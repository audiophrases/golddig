@echo off
REM Runs the same checks CI runs, in the same order.
REM
REM `npm run build` comes before the cargo steps because tauri::generate_context! reads the
REM built frontendDist — without dist/ present, cargo test cannot even link. The previous
REM version omitted it and reported a hardcoded test count in its banner.
setlocal
cd /d "%~dp0\.."

set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

echo ========================================================
echo   Golddig verification suite
echo ========================================================
echo.

echo [1/7] Installing npm dependencies...
call npm install
if errorlevel 1 goto :failed

echo.
echo [2/7] Svelte and TypeScript diagnostics...
call npm run check
if errorlevel 1 goto :failed

echo.
echo [3/7] Frontend unit tests...
call npm run test
if errorlevel 1 goto :failed

echo.
echo [4/7] Building the frontend bundle (required by the Rust steps)...
call npm run build
if errorlevel 1 goto :failed

echo.
echo [5/7] Rust formatting...
cargo fmt --manifest-path src-tauri/Cargo.toml --check
if errorlevel 1 goto :failed

echo.
echo [6/7] Clippy...
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
if errorlevel 1 goto :failed

echo.
echo [7/7] Rust tests...
cargo test --manifest-path src-tauri/Cargo.toml
if errorlevel 1 goto :failed

echo.
echo ========================================================
echo   All checks passed.
echo ========================================================
echo.
echo Running the latency benchmark over every pack in packs\ ...
echo (Build a real pack first for meaningful numbers:
echo    python scripts\build_all_packs.py ary^)
echo.
cargo run --release --manifest-path src-tauri/Cargo.toml --bin golddig-bench
echo.
pause
exit /b 0

:failed
echo.
echo ========================================================
echo   FAILED - see the output above.
echo ========================================================
pause
exit /b 1
