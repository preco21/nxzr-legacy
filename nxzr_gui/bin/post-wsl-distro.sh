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

## 2. Post-setup ##

# Prepare `usbip`.
update-alternatives --install /usr/local/bin/usbip usbip `ls /usr/lib/linux-tools/*/usbip | tail -n1` 20

# Enable `dbus-broker`.
systemctl enable dbus-broker.service

# Prepare NXZR-friendly Bluetooth settings.
#
## A. Set bluetooth enabled flag to system default settings.
echo "export BLUETOOTH_ENABLED=1" > /etc/default/bluetooth
## B. Replace `bluetoothd` service definition.
sed -i 's/\(ExecStart=\/usr\/lib\/bluetooth\/bluetoothd\).*/\1 --noplugin=*/' /lib/systemd/system/bluetooth.service
## C. Restart systemd daemons.
systemctl daemon-reload
systemctl restart dbus-broker.service bluetooth.service

## 3. Cleanup ##

# Remove unrelated files.
echo "> Cleanup unnecessary files..."

# Empty bash history.
truncate -s 0 .bash_history

# Remove unused files.
rm -f .viminfo .motd_shown

# Disable login banner.
touch ~/.hushlogin
