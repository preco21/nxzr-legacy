Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "> This script will build a new agent image for NXZR..."

# Check if there's existing distro called "nxzr-agent" and remove it.
$distro_name = "nxzr-agent"
Write-Host "> This script will build a new agent distro image for NXZR..."
$existing_agent_distro = wsl.exe --list --quiet | where { $_ -like "*$distro_name*" }
if (!$existing_agent_distro) {
    Write-Error "> Failed to locate desire distro of name: `"$distro_name`"."
    Exit 1
}

# Check if there's existing distro called "nxzr-agent" and remove it.

# FIXME: Create a new distro from scratch and just import it...
# https://medium.com/nerd-for-tech/create-your-own-wsl-distro-using-docker-226e8c9dbffe
# https://endjin.com/blog/2021/11/setting-up-multiple-wsl-distribution-instances

#
Start-Process -FilePath "wsl.exe"

# Create a new distro called "nxzr-agent".
wsl --set-default-version 2

wsl --install Ubuntu --web-download

# Enable `systemd` in "nxzr-agent".
$command = @"
cat <<'EOF' > /etc/wsl.conf
[boot]
systemd = true
command = "systemctl start dbus-broker.service bluetooth.service"
EOF
"@.Trim()
Start-Process -FilePath "wsl.exe" -ArgumentList "-u root", "-d nxzr-agent", $command -NoNewWindow -Wait

# Move `nxzr_server` binary into "nxzr-agent".

# Run `nxzr_server --install` to install and upgrade internal dependencies .

# Shutdown WSL for finalizing the setup.
wsl --shutdown

# Run `nxzr_server --config` to update config. (no restart required)
