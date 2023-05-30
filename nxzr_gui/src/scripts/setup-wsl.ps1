Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "> This script will automatically setup WSL for NXZR..."

# FIXME: Make sure the resource kernel is in place and replace kernel=*
# Create a global `.wslconfig` to current user's folder.
Write-Host "> Creating a global `".wslconfig`" file"
$wsl_conf_content = @"
[wsl2]
kernel=c:\\Users\\plusb\\.wsl\\nxzr-bzImage
"@
$home_dir = [System.Environment]::ExpandEnvironmentVariables("%USERPROFILE%")
$wsl_conf_path = Join-Path $home_dir ".wslconfig"
if (Test-Path $wsl_conf_path) {
    Write-Host "> Existing `".wslconfig`" found, checking content integrity..."
    # Replace existing config only when actual content mismatches.
    $current_conf_content = Get-Content $wsl_conf_path -Raw
    if ($current_conf_content.Trim() -ne $wsl_conf_content.Trim()) {
        Write-Host "> Existing `".wslconfig`" does not match to the desired config, replacing it with new content after backing it up..."
        # Create a backup copy of `.wslconfig` as `wslconfig.back[.N]` if there's existing one
        $backup_path = Join-Path $home_dir "wslconfig.back"
        $index = 0
        while (Test-Path $backup_path) {
            $index++
            $backup_path = Join-Path $home_dir "wslconfig.back.$index"
        }
        Copy-Item $wsl_conf_path $backup_path -Force
        # Fill `.wslconfig` with new content
        $wsl_conf_content | Set-Content $wsl_conf_path -Force
    }
    else {
        Write-Host "> Existing `".wslconfig`" matchs to the desired content, ignoring..."
    }
}
else {
    Write-Host "> No existing `".wslconfig`" found, creating a new one"
    # If there's no file present, just create a new one
    $wsl_conf_content | Set-Content $wsl_conf_path -Force
}

# Check if there's existing distro called "nxzr-agent" and remove it.

# FIXME: Create a new distro from scratch and just import it...
# https://medium.com/nerd-for-tech/create-your-own-wsl-distro-using-docker-226e8c9dbffe
# https://endjin.com/blog/2021/11/setting-up-multiple-wsl-distribution-instances

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
