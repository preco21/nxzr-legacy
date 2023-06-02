Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "> This script will build a new agent image for NXZR..."

# Check if there's existing distro called "nxzr-agent" and remove it.
$distro_name = "nxzr-agent"
Write-Host "> This script will build a new agent distro image for NXZR..."
$agent_distro_not_exists = (wsl.exe --list --quiet) -notcontains $distro_name
if ($agent_distro_not_exists) {
    Write-Error "> Failed to locate desire distro of name: `"$distro_name`"."
    Exit 1
}

# Set a variable pointing to home directory.
$home_dir = [System.Environment]::ExpandEnvironmentVariables("%USERPROFILE%")

# Try exports...
Write-Host "> Exporting `"$distro_name`" to `"$home_dir`" ..."
wsl --export $distro_name (Join-Path $home_dir "nxzr-agent.tar")
