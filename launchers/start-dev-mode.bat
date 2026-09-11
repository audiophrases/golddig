@echo off
setlocal
cd /d "%~dp0\.."

set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

echo [Golddig] Starting interactive development mode (hot-reloading)...
where npm >nul 2>&1
if errorlevel 1 (
    echo [ERROR] npm is not installed or not in PATH.
    pause
    exit /b 1
)

call npm run tauri dev
if errorlevel 1 (
    echo.
    echo [ERROR] Dev mode exited with an error.
    pause
)
