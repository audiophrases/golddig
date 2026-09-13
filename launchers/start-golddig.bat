@echo off
REM Launches the Golddig desktop app, rebuilding it first if the source is newer than the
REM binary.
REM
REM The staleness check exists because of a specific failure: the dictionary packs were
REM rebuilt in a newer format while the launcher kept starting a binary compiled before
REM that format existed, and the first lookup failed with "no such column: s.entry_id".
REM A build that is already current costs about half a second here; a launcher that
REM only built when the binary was *missing* could hand out a stale one indefinitely.
setlocal
cd /d "%~dp0\.."

set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
set "GOLDDIG_EXE=%CD%\src-tauri\target\release\golddig.exe"

if not exist "%GOLDDIG_EXE%" (
    echo [Golddig] Release binary not found. Building it now - this takes a few minutes...
    goto :build
)

REM Is anything the binary is compiled from newer than the binary?
for /f %%S in ('powershell -NoProfile -Command ^
  "$exe = (Get-Item '%GOLDDIG_EXE%').LastWriteTimeUtc;" ^
  "$src = Get-ChildItem -Recurse -File -Path 'src','src-tauri\src','src-tauri\Cargo.toml','src-tauri\tauri.conf.json','package.json' -ErrorAction SilentlyContinue | Measure-Object -Property LastWriteTimeUtc -Maximum;" ^
  "if ($src.Maximum -gt $exe) { 'stale' } else { 'fresh' }"') do set "STATE=%%S"

if "%STATE%"=="stale" (
    echo [Golddig] Source changed since the last build. Rebuilding...
    goto :build
)
goto :launch

:build
REM Close a running instance first; a locked binary made the previous build fail with
REM "Access is denied" and the stale one kept running.
taskkill /IM golddig.exe /F >nul 2>&1
call npm run build
if errorlevel 1 (
    echo [Golddig] Frontend build failed. See the output above.
    pause
    exit /b 1
)
cargo build --release --manifest-path src-tauri\Cargo.toml --bin golddig
if errorlevel 1 (
    echo [Golddig] Rust build failed. See the output above.
    pause
    exit /b 1
)
if not exist "%GOLDDIG_EXE%" (
    echo [Golddig] Build reported success but %GOLDDIG_EXE% is still missing.
    pause
    exit /b 1
)

:launch
echo [Golddig] Starting Golddig...
start "" "%GOLDDIG_EXE%"
exit /b 0
