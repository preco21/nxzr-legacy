use thiserror::Error;

const HID_UUID: &str = "00001124-0000-1000-8000-00805f9b34fb";

#[derive(Clone, Error, Debug)]
pub enum DeviceError {
    #[error("internal error: {0}")]
    Internal(SessionInternalError),
}

#[derive(Clone, Error, Debug)]
pub enum SessionInternalError {
    #[error("io: {0}")]
    Io(std::io::ErrorKind),
    #[error("bluer: {0}")]
    Bluer(bluer::ErrorKind),
}

impl From<std::io::Error> for DeviceError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(SessionInternalError::Io(err.kind()))
    }
}

impl From<bluer::Error> for DeviceError {
    fn from(err: bluer::Error) -> Self {
        Self::Internal(SessionInternalError::Bluer(err.kind))
    }
}

#[derive(Debug)]
pub struct Device {
    adapter: bluer::Adapter,
    session: bluer::Session,
}

impl Device {
    pub async fn new() -> Result<Self, DeviceError> {
        let session = bluer::Session::new().await?;
        // FIXME: allow user to select other adapters
        let adapter = session.default_adapter().await?;
        Ok(Self { adapter, session })
    }

    pub fn address(&self) {}

    pub fn set_address(&self) {}

    pub fn paired_switches(&self) {}

    pub fn unpair_path(&self) {}

    pub async fn start_advertise(&self) -> Result<(), DeviceError> {
        self.adapter.set_powered(true).await?;
        self.adapter.set_pairable(true).await?;
        // TODO: set name
        // TODO: ...
        Ok(())
    }

    pub async fn stop_advertise(&self) -> Result<(), DeviceError> {
        self.adapter.set_discoverable(false).await?;
        self.adapter.set_pairable(false).await?;
        Ok(())
    }

    pub async fn set_pairable(&self, flag: bool) -> Result<(), DeviceError> {
        self.adapter.set_pairable(flag).await?;
        Ok(())
    }

    pub fn set_class(&self) {}

    pub fn set_name(&self) {}

    pub fn uuids(&self) {}

    pub fn register_sdp_record(&self) {}

    pub fn address_of_paired_path(&self) {}
}
