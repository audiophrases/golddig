@echo off
setlocal
cd /d "%~dp0\.."

echo ========================================================
echo   Running Golddig Verification and Test Suite
echo ========================================================
echo.

echo 1. Running Svelte and TypeScript Diagnostics (npm run check)...
call npm run check
if errorlevel 1 (
    echo [FAIL] Svelte typecheck failed.
    pause
    exit /b 1
)

echo.
echo 2. Running Frontend Unit Tests (npm run test)...
call npm run test
if errorlevel 1 (
    echo [FAIL] Vitest tests failed.
    pause
    exit /b 1
)

echo.
echo 3. Running Rust Backend and Importer Tests (cargo test)...
cargo test --manifest-path src-tauri/Cargo.toml
if errorlevel 1 (
    echo [FAIL] Cargo tests failed.
    pause
    exit /b 1
)

echo.
echo 4. Running Latency Benchmark (1,000 iterations)...
cargo run --manifest-path src-tauri/Cargo.toml --bin golddig-bench
if errorlevel 1 (
    echo [FAIL] Benchmark failed.
    pause
    exit /b 1
)

echo.
echo ========================================================
echo   ALL TESTS AND BENCHMARKS PASSED SUCCESSFULLY!
echo ========================================================
echo.
pause
