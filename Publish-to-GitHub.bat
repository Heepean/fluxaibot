@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0publish-github.ps1"
if errorlevel 1 (
    echo.
    echo Publishing stopped. Review the error above and try again.
    pause
    exit /b 1
)
echo.
echo FluxAiBot was published to GitHub.
pause
