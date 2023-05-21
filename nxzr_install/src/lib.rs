use thiserror::Error;

#[derive(Clone, thiserror::Error, Debug)]
pub enum Error {
    #[error("foobar")]
    Foobar,
}

// One for windows

// systemctl install
//
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

    // install dbus broker
    // https://github.com/bus1/dbus-broker/wiki

    // setup usbipd
    // ㄴ sudo apt install linux-tools-virtual hwdata
    // ㄴ sudo update-alternatives --install /usr/local/bin/usbip usbip `ls /usr/lib/linux-tools/*/usbip | tail -n1` 20
    // echo 'export BLUETOOTH_ENABLED=1' | sudo tee /etc/default/bluetooth
    // code /etc/bluetooth/main.conf
    Ok(())
}

// One for linux
pub async fn ensure_system_requirements() -> Result<(), SysCheckError> {
    // sudo systemctl daemon-reload
    // sudo systemctl restart bluetooth
}

#[derive(Clone, Error, Debug)]
pub enum SystemCommandError {
    #[error("failed to execute a command: {0}")]
    CommandFailed(String),
    #[error("internal error: {0}")]
    Internal(SystemCommandInternalError),
}

#[derive(Clone, Error, Debug)]
pub enum SystemCommandInternalError {
    #[error("utf8: {0}")]
    Utf8Error(std::str::Utf8Error),
    #[error("io: {kind}; {message}")]
    Io {
        kind: std::io::ErrorKind,
        message: String,
    },
}

impl From<std::str::Utf8Error> for SystemCommandError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::Internal(SystemCommandInternalError::Utf8Error(err))
    }
}

impl From<std::io::Error> for SystemCommandError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(SystemCommandInternalError::Io {
            kind: err.kind(),
            message: err.to_string(),
        })
    }
}

async fn run_system_command(mut command: Command) -> Result<(), SystemCommandError> {
    let output = command.output().await?;
    if !output.status.success() {
        return Err(SystemCommandError::CommandFailed(
            std::str::from_utf8(&output.stderr)?.to_owned(),
        ));
    }
    Ok(())
}
