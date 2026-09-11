@echo off
setlocal
cd /d "%~dp0\.."

set "EXE_PATH=%~dp0\..\src-tauri\target\release\golddig.exe"

if not exist "%EXE_PATH%" (
    echo [Golddig] Release binary not found. Building release binary...
    where npm >nul 2>&1
    if errorlevel 1 (
        echo [ERROR] npm is not found in PATH.
        pause
        exit /b 1
    )
    call npm run tauri build
    if not exist "%EXE_PATH%" (
        echo [ERROR] Build failed or executable was not produced.
        pause
        exit /b 1
    )
)

echo [Golddig] Starting Golddig desktop dictionary...
start "" "%EXE_PATH%"
exit /b 0
