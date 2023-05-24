#!/usr/bin/env bash
set -e

# This script is meant to be built-in to the binary directly, assuming that the
# script is privileged already to perform required actions.

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
