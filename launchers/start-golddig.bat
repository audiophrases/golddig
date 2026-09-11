@echo off
setlocal
cd /d "%~dp0\.."

set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

echo [Golddig] Starting Golddig desktop dictionary...
start "" "%CD%\src-tauri\target\release\golddig.exe"
exit /b 0
