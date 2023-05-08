use thiserror::Error;
use tokio::process::Command;

use crate::helper;

#[derive(Clone, Error, Debug)]
pub enum SysCheckError {
    #[error("privilege error, the program is required to run as root user")]
    RootPrivilegeRequired,
    #[error("systemctl check failed")]
    SysctlFailed,
    #[error("Bluetooth service check failed: {0}")]
    BluetoothFailed(String),
    #[error("dbus service check failed: {0}")]
    DBusFailed(String),
    #[error("cli tool presence check failed: {0}")]
    CliToolFailed(String),
}

pub async fn check_system_requirements() -> Result<(), SysCheckError> {
    // Check if the program has been run as root.
    if sudo::check() != sudo::RunningAs::Root {
        return Err(SysCheckError::RootPrivilegeRequired);
    }
    // Check if the Bluetooth service is active.
    if !systemctl::exists("bluetooth.service").map_err(|_| SysCheckError::SysctlFailed)? {
        return Err(SysCheckError::BluetoothFailed(
            "Bluetooth service does not exist".to_owned(),
        ));
    };
    if !systemctl::is_active("bluetooth.service").map_err(|_| SysCheckError::SysctlFailed)? {
        return Err(SysCheckError::BluetoothFailed(
            "Bluetooth service is not active".to_owned(),
        ));
    }
    // FIXME: maybe this is not platform agnostic
    // Check if the `dbus` service is active
    if !systemctl::exists("dbus.service").map_err(|_| SysCheckError::SysctlFailed)? {
        return Err(SysCheckError::DBusFailed(
            "dbus service does not exist".to_owned(),
        ));
    };
    if !systemctl::is_active("dbus.service").map_err(|_| SysCheckError::SysctlFailed)? {
        return Err(SysCheckError::DBusFailed(
            "dbus service is not active".to_owned(),
        ));
    }
    // Check if `hciconfig` exists
    helper::run_command({
        let mut cmd = Command::new("hciconfig");
        cmd.args(&["--h"]);
        cmd
    })
    .await
    .map_err(|_| SysCheckError::CliToolFailed("hciconfig".to_owned()))?;

    // FIXME: check for bluetooth ctl?

    // FIXME: maybe this is not platform agnostic
    // Check if `bdaddr` exists
    // helper::run_command({
    //     let mut cmd = Command::new("bdaddr");
    //     cmd.args(&["-h"]);
    //     cmd
    // })
    // .await
    // .map_err(|_| SysCheckError::CliToolFailed("bdaddr".to_owned()))?;
    Ok(())
}
