use crate::shared::{self, Address, SystemCommandError};
use tokio::process::Command;

#[tracing::instrument(target = "helper")]
pub(crate) async fn set_adapter_address(
    adapter_name: &str,
    address: Address,
) -> Result<(), SystemCommandError> {
    tracing::info!(
        "resetting Bluetooth adapter ({}) with address \"{:?}\".",
        adapter_name,
        address
    );
    // Set Bluetooth adapter address by adapter name.
    //
    // The user will need to install `apt-get install bluez-utils`.
    // FIXME: update to not use bdaddr
    // run_command({
    //     let mut cmd = Command::new("bdaddr");
    //     cmd.args(&["-i", adapter_name, &address.to_string()]);
    //     cmd
    // })
    // .await?;
    // Reset Bluetooth adapter by running `hciconfig`.
    shared::run_system_command({
        let mut cmd = Command::new("hciconfig");
        cmd.args(&[adapter_name, "reset"]);
        cmd
    })
    .await?;
    // Restart Bluetooth service.
    restart_bluetooth_service()?;
    Ok(())
}

#[tracing::instrument(target = "helper")]
pub(crate) async fn set_device_class(
    adapter_name: &str,
    class: u32,
) -> Result<u32, SystemCommandError> {
    let str_class: String = format!("0x{:X}", class);
    tracing::info!(
        "setting device class of adapter {:?} to {:?}.",
        adapter_name,
        str_class.as_str()
    );
    shared::run_system_command({
        let mut cmd = Command::new("hciconfig");
        cmd.args(&[adapter_name, "class", str_class.as_str()]);
        cmd
    })
    .await?;
    Ok(class)
}

pub fn restart_bluetooth_service() -> Result<(), SystemCommandError> {
    systemctl::restart("bluetooth.service")?;
    Ok(())
}
