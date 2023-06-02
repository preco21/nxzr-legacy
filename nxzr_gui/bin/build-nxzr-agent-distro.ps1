Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "> This script will build a new agent image for NXZR..."

# Check if there's existing base `Ubuntu` distro, if so, cancel running the script.
Write-Host "> Checking whether there's base image already exists..."
$base_distro_name = "Ubuntu"
$base_ubuntu_distro_exists = (wsl.exe --list --quiet) -contains $base_distro_name
if ($base_ubuntu_distro_exists) {
    Write-Error "> The base WSL distribution `"$base_distro_name`" already exists, aborting the script due to dirty status."
    Exit 1
}

# Check if there's existing distro called "nxzr-agent" and remove it.
$distro_name = "nxzr-agent"
Write-Host "> Checking whether there's existing agent image already..."
$agent_distro_exists = (wsl.exe --list --quiet) -contains $distro_name
if ($agent_distro_exists) {
    # Stop and unregister the distribution.
    wsl.exe --terminate $distro_name
    wsl.exe --unregister $distro_name
    Write-Host "> The WSL distribution `"$distro_name`" has been removed."
}
else {
    Write-Host "> No existing WSL distribution found with the name `"$distro_name`"."
}

# Set default wsl version to 2
Write-Host "> Setting default WSL version to 2..."
wsl.exe --set-default-version 2

# Download a base image.
Write-Host "> Installing WSL distro: $base_distro_name..."
wsl.exe --install $base_distro_name --web-download

# Create temporary directory to work with.
$tempdir = New-TemporaryDirectory
Write-Host "> Using temporary directory: $tempdir"

# Set a variable pointing to home directory.
$home_dir = [System.Environment]::ExpandEnvironmentVariables("%USERPROFILE%")

# Create a new distro for the agent.
Write-Host "> Creating a new distro..."
$nxzr_agent_tar = Join-Path $tempdir "$distro_name.tar"
wsl.exe --export $base_distro_name $nxzr_agent_tar
wsl.exe --import $distro_name (Join-Path $home_dir ".wsl/$distro_name") $nxzr_agent_tar

Write-Host "> Setting `"wsl.conf`"..."
$command = @"
cat <<'EOF' > /etc/wsl.conf
[boot]
systemd = true
command = "systemctl start dbus-broker.service bluetooth.service"
EOF
"@.Trim()
Start-Process -FilePath "wsl.exe" -ArgumentList "-u root -d $distro_name $command" -NoNewWindow -Wait

# Run pre-installation setup.
Write-Host "> Running pre-installation setup..."
wsl.exe -e "$(Join-Path $PSScriptRoot "pre-wsl-distro.sh")"

# Shutdown WSL for finalizing the setup.
Write-Host "> Shutting down WSL..."
wsl.exe --shutdown

Write-Host "> Wait for WSL to shutdown completely..."
# Wait for WSL to shutdown completely.
Start-Sleep -Seconds 8

# Run post-installation setup.
Write-Host "> Running post setup..."
wsl.exe -e "$(Join-Path $PSScriptRoot "post-wsl-distro.sh")"

# Cleanup the temporary directory.
Remove-Item $tempdir -Recurse
