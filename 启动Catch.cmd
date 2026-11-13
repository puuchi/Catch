@echo off
setlocal
set "ELECTRON_RUN_AS_NODE="
if not exist "%~dp0src\ui\generated\main.js" (
  echo UI build is missing. Run npm ci and npm run build, or use the installer.
  pause
  exit /b 1
)
if exist "%~dp0runtime-45.3.1\electron.exe" (
  start "" "%~dp0runtime-45.3.1\electron.exe" "%~dp0."
) else if exist "%~dp0runtime\electron.exe" (
  start "" "%~dp0runtime\electron.exe" "%~dp0."
) else if exist "%~dp0node_modules\electron\dist\electron.exe" (
  start "" "%~dp0node_modules\electron\dist\electron.exe" "%~dp0."
) else (
  echo Electron is missing. Run npm ci, or download the Catch installer.
  pause
  exit /b 1
)
endlocal
