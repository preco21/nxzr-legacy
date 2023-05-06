
#[derive(Clone, thiserror::Error, Debug)]
pub enum Error {
    #[error("transparent")]
}

pub type Result<T> = std::result::Result<T, Error>;


pub async fn prepare_for_prerequisites() {
    // check wsl installed
    // check if system can run wsl -> vm requirements
    // check usbipd installed -> maybe just include the binary

    // check wsl config is ready -> otherwise, install one
    // https://github.com/dorssel/usbipd-win/wiki/WSL-support
    // check /etc/wsl.conf is ready -> otherwise, set one and restart vm (wait 8 sec)

    // [internal vm]
    // upgrade apt
    // ㄴ sudo apt upgrade -y

    // install dbus broker
    // https://github.com/bus1/dbus-broker/wiki

    // setup usbipd
    // ㄴ sudo apt install linux-tools-virtual hwdata
    // ㄴ sudo update-alternatives --install /usr/local/bin/usbip usbip `ls /usr/lib/linux-tools/*/usbip | tail -n1` 20

    // setup bdaddr, hcitool, stuffs...
}

pub async fn check_prerequisites() {

}
pub async fn check_system_requirements() {
    systemctl::exists("bluetooth.service")
    // check systemctl is ready

    // check dbus, bluetooth systemctl is ready
    // ㄴ if not installed, bail
    // ㄴ if installed, start the service
    // ㄴ if misconfigured, set config and start the service

    // check bluetooth related tool, bdaddr, hcitool is ready
}

pub async fn run_command(mut command: Command) -> Result<(), HelperError> {
    let output = command.output().await?;
    if !output.status.success() {
        return Err(HelperError::CommandFailed(
            std::str::from_utf8(output.stderr.as_ref())?.to_owned(),
        ));
    }
    Ok(())
}
