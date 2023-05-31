#!/usr/bin/env bash
set -e

if [ $EUID -ne 0 ]; then
  echo "This script must be run as root."
  exit 1
fi

## 1. Setup ##

# Disable history for the moment.
unset HISTFILE

# Go to home directory.
cd ~

## 2. Install or upgrade dependencies ##

# Mark `needrestart` config to work with a non-interactive TTY.
sed -i "/#\$nrconf{restart} = 'i';/s/.*/\$nrconf{restart} = 'a';/" /etc/needrestart/needrestart.conf

# Upgrade all installed dependencies to latest as well as the distro.
echo "> Checking for updates..."
apt update && apt -y dist-upgrade

# Install required packages.
#
# Note that, in fact, we don't need to install `dbus` for the server daemon to work.
#
# However, we are just making sure that the latest `dbus` is installed in case
# of failure to use of `dbus-broker`.
echo "> Installing required dependencies..."
apt -y install linux-tools-virtual hwdata bluez dbus dbus-broker

# Do some cleanup.
echo "> Running some cleanup..."
apt -y autoremove && apt -y clean

# Update WSL config to enable `systemd` and startup services.
cat <<'EOF' > /etc/wsl.conf
[boot]
systemd = true
command = "systemctl start dbus-broker.service bluetooth.service"
EOF

## 4. Reboot ##

# Make sure the update-manager-core exists.
echo "> Installing update-manager-core..."
apt install update-manager-core

# Finally, run upgrade for the distro.
echo "> Trying to upgrade distro to latest version..."
do-release-upgrade -d

# Reboot is required!
if test -f /var/run/reboot-required; then
  if [[ -n "$IS_WSL" || -n "$WSL_DISTRO_NAME" ]]; then
    echo "A reboot is required to finish installing updates. But it seems you are in WSL environment, which requires manual shutdown. Exit the terminal session, then run 'wsl --shutdown' to reboot WSL manually."
  else
    read -p "A reboot is required to finish installing updates. Press [ENTER] to reboot now, or [CTRL+C] to cancel and reboot later."
    reboot
  fi
else
  echo "A reboot is not required. Exiting..."
fi
