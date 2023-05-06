use thiserror::Error;

use crate::helper::HelperError;

#[derive(Clone, Error, Debug)]
pub enum SysCheckError {
    #[error("systemctl check failed")]
    SysctlFailed,
    #[error("bluetooth service check failed: {0}")]
    BluetoothFailed(String),
    #[error("dbus service check failed: {0}")]
    DBusFailed(String),
    #[error("cli tool presence check failed: {0}")]
    CliToolFailed(String),
}

pub async fn check_system_requirements() -> Result<(), SysCheckError> {
    // Check if the `bluetooth` service is active.
    if !systemctl::exists("bluetooth.service").map_err(|_| SysCheckError::SysctlFailed) {
        return Err(SysCheckError::BluetoothFailed(
            "bluetooth service does not exist".to_owned(),
        ));
    };
    if !systemctl::is_active("bluetooth.service").map_err(|_| SysCheckError::SysctlFailed) {
        return Err(SysCheckError::BluetoothFailed(
            "bluetooth service is not active".to_owned(),
        ));
    }
    // Check if the `dbus` service is active
    if !systemctl::exists("dbus.service").map_err(|_| SysCheckError::SysctlFailed) {
        return Err(SysCheckError::DBusFailed(
            "dbus service does not exist".to_owned(),
        ));
    };
    if !systemctl::is_active("dbus.service").map_err(|_| SysCheckError::SysctlFailed) {
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
    .map_err(|_| SysCheckError::CliToolFailed("hciconfig".to_owned()));
    // Check if `bdaddr` exists
    helper::run_command({
        let mut cmd = Command::new("bdaddr");
        cmd.args(&["-h"]);
        cmd
    })
    .await
    .map_err(|_| SysCheckError::CliToolFailed("bdaddr".to_owned()));
    Ok(())
}
