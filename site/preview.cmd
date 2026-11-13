@echo off
setlocal
cd /d "%~dp0.."
node site/tools/serve.cjs
if errorlevel 1 pause
