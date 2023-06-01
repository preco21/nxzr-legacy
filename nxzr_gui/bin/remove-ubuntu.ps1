Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "> This script will remove distro: Ubuntu..."

# Check if there's existing base `Ubuntu` distro, if so, cancel running the script.
Write-Host "> Checking whether there's base image already exists..."
$base_distro_name = "Ubuntu"
$base_ubuntu_distro_not_exists = (wsl.exe --list --quiet) -notcontains $base_distro_name
if ($base_ubuntu_distro_not_exists) {
    Write-Error "> The base WSL distribution `"$base_distro_name`" does not exists, aborting..."
    Exit 1
}

Write-Host "> Removing distro: $base_distro_name..."
wsl.exe --terminate $base_distro_name
wsl.exe --unregister $base_distro_name
wsl.exe --shutdown
