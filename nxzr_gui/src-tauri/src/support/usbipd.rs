use super::wsl;
use crate::{config, util};
use tokio::time::{self, Duration};

#[derive(Debug, thiserror::Error)]
pub enum UsbipdError {
    #[error("failed to retrieve usbipd state: {0}")]
    UsbipdStateLookupFailed(String),
    #[error("failed to attach hid adapter: {0}")]
    UsbipdAttachFailed(String),
    #[error("failed to detach hid adapter: {0}")]
    UsbipdDetachFailed(String),
    #[error(transparent)]
    SystemCommandError(#[from] util::SystemCommandError),
    #[error(transparent)]
    WslError(#[from] wsl::WslError),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct AdapterInfo {
    pub id: String,
    pub serial: String,
    pub name: String,
    pub bus_id: String,
    pub hardware_id: String,
    pub is_attached: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all(deserialize = "PascalCase"))]
pub struct UsbipdState {
    pub devices: Vec<UsbipdStateDevice>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all(deserialize = "PascalCase"))]
pub struct UsbipdStateDevice {
    pub bus_id: Option<String>,
    pub client_ip_address: Option<String>,
    pub client_wsl_instance: Option<String>,
    pub description: String,
    pub instance_id: String,
    pub is_forced: bool,
    pub persisted_guid: Option<String>,
    pub stub_instance_id: Option<String>,
}

pub async fn list_hid_adapters() -> Result<Vec<AdapterInfo>, UsbipdError> {
    // Get `usbipd`'s json state
    let output = util::run_system_command({
        let mut cmd = tokio::process::Command::new("usbipd.exe");
        cmd.args(&["state"]);
        cmd
    })
    .await
    .map_err(|err| UsbipdError::UsbipdStateLookupFailed(err.to_string()))?;
    let parsed: UsbipdState = serde_json::from_str(&output)?;
    let adapter_info_vec = parsed
        .devices
        .iter()
        .filter_map(|device| {
            // If the device is disconnected, it should be treated as a nonexistent device.
            let Some(bus_id) = device.bus_id.clone() else {
                return None;
            };
            let is_attached = match &device.client_wsl_instance {
                Some(wsl_instance) => wsl_instance == config::WSL_DISTRO_NAME,
                None => false,
            };
            let Some((vid_pid, serial)) = parse_hardware_id(&device.instance_id) else {
                return None;
            };
            Some(AdapterInfo {
                id: vid_pid.clone(),
                serial,
                bus_id,
                hardware_id: vid_pid,
                name: device.description.clone(),
                is_attached,
            })
        })
        .collect();
    Ok(adapter_info_vec)
}

pub async fn attach_hid_adapter(hardware_id: &str) -> Result<(), UsbipdError> {
    // Always detach already attached adapter before attaching new one.
    detach_hid_adapter(&hardware_id).await?;
    // Wait for a second to make sure the detach is complete.
    time::sleep(Duration::from_millis(1000)).await;
    tracing::info!("attaching hid adapter to WSL: {}", hardware_id);
    util::run_system_command({
        let mut cmd = tokio::process::Command::new("usbipd.exe");
        cmd.args(&[
            "wsl",
            "attach",
            "-d",
            config::WSL_DISTRO_NAME,
            "-i",
            hardware_id,
        ]);
        cmd
    })
    .await
    .map_err(|err| UsbipdError::UsbipdAttachFailed(err.to_string()))?;
    Ok(())
}

pub async fn detach_hid_adapter(hardware_id: &str) -> Result<(), UsbipdError> {
    tracing::info!("detaching hid adapter from WSL: {}", hardware_id);
    util::run_system_command({
        let mut cmd = tokio::process::Command::new("usbipd.exe");
        cmd.args(&["wsl", "detach", "-i", hardware_id]);
        cmd
    })
    .await
    .map_err(|err| UsbipdError::UsbipdDetachFailed(err.to_string()))?;
    Ok(())
}

fn parse_hardware_id(raw: &str) -> Option<(String, String)> {
    let mut split = raw.split("\\").skip(1);
    let Some(vid_pid) = split.next() else {
        return None;
    };
    let vid_pid_split: Vec<&str> = vid_pid.split("&").collect();
    let Some(vid) = vid_pid_split[0].split("_").last() else {
        return None;
    };
    let Some(pid) = vid_pid_split[1].split("_").last() else {
        return None;
    };
    let vid_pid = format!("{}:{}", vid, pid);
    let Some(serial) = split.next() else {
        return None;
    };
    Some((vid_pid, serial.to_string()))
}

#[cfg(test)]
mod tests {
    use crate::support::usbipd::UsbipdState;

    use super::parse_hardware_id;

    #[test]
    fn usbipd_json_parse() {
        let data = r#"
        {
            "Devices": [
                {
                    "BusId": "3-2",
                    "ClientIPAddress": null,
                    "ClientWslInstance": null,
                    "Description": "USB Input Device",
                    "InstanceId": "USB\\VID_04D8&PID_EED3\\1551771897",
                    "IsForced": false,
                    "PersistedGuid": null,
                    "StubInstanceId": null
                },
                {
                    "BusId": "1-16",
                    "ClientIPAddress": "172.19.142.76",
                    "ClientWslInstance": "dev",
                    "Description": "Realtek Bluetooth 5.0 Adapter",
                    "InstanceId": "USB\\VID_0BDA&PID_8771\\00E04C239987",
                    "IsForced": false,
                    "PersistedGuid": "0b18e653-15d6-4d7d-a710-21cf68e441b0",
                    "StubInstanceId": "USB\\Vid_80EE&Pid_CAFE\\00E04C239987"
                },
                {
                    "BusId": "1-18",
                    "ClientIPAddress": null,
                    "ClientWslInstance": null,
                    "Description": "USB Input Device, Foobar",
                    "InstanceId": "USB\\VID_1532&PID_00A4\\6&104FED9E&0&18",
                    "IsForced": false,
                    "PersistedGuid": null,
                    "StubInstanceId": null
                },
                {
                    "BusId": "1-22",
                    "ClientIPAddress": null,
                    "ClientWslInstance": null,
                    "Description": "USB Input Device",
                    "InstanceId": "USB\\VID_1E71&PID_170E\\5D87148B323",
                    "IsForced": false,
                    "PersistedGuid": null,
                    "StubInstanceId": null
                }
            ]
        }"#;
        let d: UsbipdState = serde_json::from_str(data).unwrap();
        println!("{:?}", &d);
    }

    #[test]
    fn vid_pid_parse() {
        let parsed = parse_hardware_id("USB\\VID_04D8&PID_EED3\\1551771897").unwrap();
        assert_eq!(parsed.0, "04D8:EED3");
    }
}
