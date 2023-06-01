Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "> This script will build a new agent image for NXZR..."

# Check if there's existing distro called "nxzr-agent" and remove it.
$distro_name = "nxzr-agent"
Write-Host "> This script will build a new agent distro image for NXZR..."
$existing_agent_distro = wsl.exe --list --quiet | Where-Object { $_ -like "*$distro_name*" }
if ($existing_agent_distro) {
    # Stop and unregister the distribution.
    wsl.exe --terminate $distro_name
    wsl.exe --unregister $distro_name
    Write-Host "> The WSL distribution `"$distro_name`" has been removed."
}
else {
    Write-Host "> No existing WSL distribution found with the name `"$distro_name`"."
}

# Set default wsl version to 2
Start-Process -FilePath "wsl.exe" -ArgumentList "--set-default-version 2" -NoNewWindow -Wait

# Download a base image.
Start-Process -FilePath "wsl.exe" -ArgumentList "--install Ubuntu --web-download" -NoNewWindow -Wait

# Create a new distro called "nxzr-agent". (export / import?)

# FIXME: run script in wsl with -e script...
$command = @"
cat <<'EOF' > /etc/wsl.conf
[boot]
systemd = true
command = "systemctl start dbus-broker.service bluetooth.service"
EOF
"@.Trim()
Start-Process -FilePath "wsl.exe" -ArgumentList "-u root", "-d nxzr-agent", $command -NoNewWindow -Wait

# Run pre

# Shutdown WSL for finalizing the setup.
wsl --shutdown

# Run post
