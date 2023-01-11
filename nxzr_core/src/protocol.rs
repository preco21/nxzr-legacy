use strum::{Display, EnumString};

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Display, EnumString)]
pub enum Subcommand {
    RequestDeviceInfo = 0x02,
    SetInputReportMode = 0x03,
    TriggerButtonsElapsedTime = 0x04,
    SetShipmentState = 0x08,
    SpiFlashRead = 0x10,
    SetNfcIrMcuConfig = 0x21,
    SetNfcIrMcuState = 0x22,
    SetPlayerLights = 0x30,
    Enable6AxisSensor = 0x40,
    EnableVibration = 0x48,
}

impl Subcommand {
    pub fn from_byte(value: u8) -> Self {
        Subcommand::Enable6AxisSensor
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Display, EnumString)]
pub enum SwitchResponse {
    NoData,
    Malformed,
    TooShort,
    UnknownSubcommand,
    RequestDeviceInfo,
    SetShipment,
    SpiMode,
    SetMode,
    TriggerButtons,
    ToggleImu,
    EnableVibration,
    SetPlayer,
    SetNfcIrState,
    SetNfcIrConfig,
}

struct ControllerProtocol;
