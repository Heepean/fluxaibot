@echo off
cd /d "%~dp0"
if not exist "FluxAiBot.exe" (
    echo FluxAiBot.exe not found, copying from build...
    copy /Y "target\release\local_ai_hub.exe" "FluxAiBot.exe" >nul
)
start "" "FluxAiBot.exe"