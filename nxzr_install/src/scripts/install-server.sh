#!/usr/bin/env bash
set -e

# This script is meant to be built-in to the binary directly, assuming that the
# script is privileged already to perform required actions.

# Ensure `apt` packages are all up-to-date.
apt update && apt -y dist-upgrade

# Mark `needrestart` config to work with a non-interactive TTY.
sed -i "/#\$nrconf{restart} = 'i';/s/.*/\$nrconf{restart} = 'a';/" /etc/needrestart/needrestart.conf

# Install required packages.
#
# Note that, in fact, we don't need to install `dbus` for the server daemon to work.
#
# However, we are just making sure that the latest `dbus` is installed in case
# of failure to use of `dbus-broker`.
apt -y install linux-tools-virtual hwdata bluez dbus dbus-broker

# Cleanup packages.
apt -y autoremove && apt -y clean

# Prepare `usbip`.
update-alternatives --install /usr/local/bin/usbip usbip `ls /usr/lib/linux-tools/*/usbip | tail -n1` 20

# Enable `dbus-broker`.
systemctl enable dbus-broker.service

# Prepare NXZR-friendly Bluetooth settings.
#
## A. Set bluetooth enabled flag to system default settings.
echo "export BLUETOOTH_ENABLED=1" > /etc/default/bluetooth
## B. Replace `bluetoothd` service definition.
sed -i 's/\(ExecStart=\/usr\/lib\/bluetooth\/bluetoothd\).*/\1 --noplugin=input,sap,avrcp,a2dp,a2dp-source,hfp,spp,opp,hdp/' /lib/systemd/system/bluetooth.service
## C. Restart systemd daemons.
systemctl daemon-reload
systemctl restart dbus-broker.service bluetooth.service
