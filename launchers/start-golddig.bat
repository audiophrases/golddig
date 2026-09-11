@echo off
setlocal
cd /d "%~dp0\.."

set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
set "EXE_PATH=%CD%\src-tauri\target\release\golddig.exe"

echo [Golddig] Starting Golddig desktop dictionary from: %CD%
start "" "%EXE_PATH%"
exit /b 0
