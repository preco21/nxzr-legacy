Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Enable systemd in WSL.
$command = @"
cat <<'EOF' > /etc/wsl.conf
[boot]
systemd = true
command = "systemctl start dbus-broker.service bluetooth.service"
EOF
"@.Trim()

Start-Process -FilePath "wsl.exe" -ArgumentList "-u root",$command -NoNewWindow -Wait

Read-Host -Prompt "Press enter key to continue"
