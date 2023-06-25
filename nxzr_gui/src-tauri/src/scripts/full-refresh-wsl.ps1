Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "> This script will update WSL and restart..."

# Shutdown WSL
Write-Host "> Shutting down WSL..."
Start-Process -FilePath "wsl.exe" -ArgumentList "--shutdown" -Wait -NoNewWindow

# Restart WSL
Write-Host "> Updating WSL..."
Start-Process -FilePath "wsl.exe" -ArgumentList "--update --web-download" -Wait -NoNewWindow
