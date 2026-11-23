@echo off
setlocal
cd /d "%~dp0src-tauri"

where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust/Cargo was not found. Install Rust from https://rustup.rs then run this file again.
  pause
  exit /b 1
)

echo Starting Hiwcha Noise Cancel...
cargo run
set "exit_code=%errorlevel%"
if not "%exit_code%"=="0" (
  echo.
  echo The application could not start. Review the message above.
  pause
)
exit /b %exit_code%
