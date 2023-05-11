use strum::Display;

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Subcommand {
    RequestDeviceInfo,
    SetInputReportMode,
    TriggerButtonsElapsedTime,
    SetHciState,
    SetShipmentState,
    SpiFlashRead,
    SetNfcIrMcuConfig,
    SetNfcIrMcuState,
    SetPlayerLights,
    Enable6AxisSensor,
    EnableVibration,
    // Can be set when a shorter report that cannot determine subcommand has received.
    Empty,
}

impl Subcommand {
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x02 => Some(Self::RequestDeviceInfo),
            0x03 => Some(Self::SetInputReportMode),
            0x04 => Some(Self::TriggerButtonsElapsedTime),
            0x06 => Some(Self::SetHciState),
            0x08 => Some(Self::SetShipmentState),
            0x10 => Some(Self::SpiFlashRead),
            0x21 => Some(Self::SetNfcIrMcuConfig),
            0x22 => Some(Self::SetNfcIrMcuState),
            0x30 => Some(Self::SetPlayerLights),
            0x40 => Some(Self::Enable6AxisSensor),
            0x48 => Some(Self::EnableVibration),
            _ => None,
        }
    }

    pub fn to_byte(&self) -> Option<u8> {
        match self {
            Self::RequestDeviceInfo => Some(0x02),
            Self::SetInputReportMode => Some(0x03),
            Self::TriggerButtonsElapsedTime => Some(0x04),
            Self::SetHciState => Some(0x06),
            Self::SetShipmentState => Some(0x08),
            Self::SpiFlashRead => Some(0x10),
            Self::SetNfcIrMcuConfig => Some(0x21),
            Self::SetNfcIrMcuState => Some(0x22),
            Self::SetPlayerLights => Some(0x30),
            Self::Enable6AxisSensor => Some(0x40),
            Self::EnableVibration => Some(0x48),
            Self::Empty => None,
        }
    }
}
