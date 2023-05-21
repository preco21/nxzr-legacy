#[cfg(target_os = "linux")]
pub async fn install_device_system_requirements() -> cmd_lib::CmdResult {
    let out = cmd_lib::run_cmd! (
        apt update
        apt upgrade -y
        sudo apt install linux-tools-virtual hwdata
        sudo update-alternatives --install /usr/local/bin/usbip usbip $(ls /usr/lib/linux-tools/*/usbip | tail -n1) 20
    )?;

    Ok(())
}

#[derive(Clone, Error, Debug)]
pub enum InstallError {
    #[error("package manager failed")]
    PackageManagerFailed(String),
    #[error("other")]
    Other,
}

/// Installs system requirements and modifies configs for the device to work.
#[tracing::instrument(target = "system")]
async fn install_device() -> Result<(), InstallError> {
    // Update the `apt` registry to the latest.
    run_system_command({
        let mut cmd = Command::new("apt");
        cmd.args(&["update"]);
        cmd
    })
    .await
    .map_err(|err| {
        InstallError::PackageManagerFailed(format!(
            "failed to update apt index: {}",
            err.to_string()
        ))
    })?;
    // Run `apt` upgrades.
    run_system_command({
        let mut cmd = Command::new("apt");
        cmd.args(&["upgrade", "-y"]);
        cmd
    })
    .await
    .map_err(|err| {
        InstallError::PackageManagerFailed(format!(
            "failed to perform upgrade: {}",
            err.to_string()
        ))
    })?;
    // Install `bluez`.
    run_system_command({
        let mut cmd = Command::new("apt");
        cmd.args(&["update"]);
        cmd
    })
    .await
    .map_err(|err| {
        InstallError::PackageManagerFailed(format!(
            "failed to update apt index: {}",
            err.to_string()
        ))
    })?;
    // Setup `usbipd`.
    // Setup `dbus-broker`.

    systemctl::Ok(())
}
