#[derive(Clone, thiserror::Error, Debug)]
pub enum Error {
    #[error("foobar")]
    Foobar,
}

pub async fn prepare_system_requirements() -> Result<(), Error> {
    // change bluez name -> Pro Controller... etc then restart systemctl

    // when [force clean connect] -> clear all settings to initial one

    // check wsl installed
    // wsl version check if it's v2
    // check if system can run wsl -> vm requirements
    // check usbipd installed -> maybe just include the binary

    // check wsl config is ready -> otherwise, install one
    // https://github.com/dorssel/usbipd-win/wiki/WSL-support
    // check /etc/wsl.conf is ready -> otherwise, set one and restart vm (wait 8 sec)

    // [internal vm]
    // upgrade apt
    // ㄴ sudo apt upgrade -y

    // disable windows bt

    // bluetoothctl scan off

    // install dbus broker
    // https://github.com/bus1/dbus-broker/wiki

    // setup usbipd
    // ㄴ sudo apt install linux-tools-virtual hwdata
    // ㄴ sudo update-alternatives --install /usr/local/bin/usbip usbip `ls /usr/lib/linux-tools/*/usbip | tail -n1` 20

    // setup bdaddr, hcitool, stuffs...
    Ok(())
}
