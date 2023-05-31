#!/usr/bin/env bash
set -e

# This script is meant to be built-in to the binary directly, assuming that the
# script is privileged already to perform required actions.

# Mark `needrestart` config to work with a non-interactive TTY.
sed -i "/#\$nrconf{restart} = 'i';/s/.*/\$nrconf{restart} = 'a';/" /etc/needrestart/needrestart.conf

# Ensure `apt` packages are all up-to-date.
apt update && apt -y dist-upgrade

# Install required packages.
#
# Note that, in fact, we don't need to install `dbus` for the server daemon to work.
#
# However, we are just making sure that the latest `dbus` is installed in case
# of failure to use of `dbus-broker`.
apt -y install linux-tools-virtual hwdata bluez dbus dbus-broker

# Cleanup packages.
apt -y autoremove && apt -y clean

# Update WSL config to enable `systemd` and startup services.
cat <<'EOF' > /etc/wsl.conf
[boot]
systemd = true
command = "systemctl start dbus-broker.service bluetooth.service"
EOF
