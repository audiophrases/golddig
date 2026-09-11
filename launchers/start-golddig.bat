@echo off
REM Launches the Golddig desktop app, building it first if needed.
REM
REM Both READMEs claimed this script auto-builds; it did not, and if the binary was
REM missing it exited 0 with no window and no message.
setlocal
cd /d "%~dp0\.."

set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
set "GOLDDIG_EXE=%CD%\src-tauri\target\release\golddig.exe"

if not exist "%GOLDDIG_EXE%" (
    echo [Golddig] Release binary not found. Building it now - this takes a few minutes...
    call npm run tauri build
    if errorlevel 1 (
        echo [Golddig] Build failed. See the output above.
        pause
        exit /b 1
    )
)

if not exist "%GOLDDIG_EXE%" (
    echo [Golddig] Build reported success but %GOLDDIG_EXE% is still missing.
    pause
    exit /b 1
)

echo [Golddig] Starting Golddig...
start "" "%GOLDDIG_EXE%"
exit /b 0
