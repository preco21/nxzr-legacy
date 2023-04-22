use strum::Display;

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Subcommand {
    RequestDeviceInfo,
    SetInputReportMode,
    TriggerButtonsElapsedTime,
    SetShipmentState,
    SpiFlashRead,
    SetNfcIrMcuConfig,
    SetNfcIrMcuState,
    SetPlayerLights,
    Enable6AxisSensor,
    EnableVibration,
    Empty,
}

impl Subcommand {
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x02 => Some(Self::RequestDeviceInfo),
            0x03 => Some(Self::SetInputReportMode),
            0x04 => Some(Self::TriggerButtonsElapsedTime),
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

    pub fn to_byte(&self) -> u8 {
        match self {
            Self::RequestDeviceInfo => 0x02,
            Self::SetInputReportMode => 0x03,
            Self::TriggerButtonsElapsedTime => 0x04,
            Self::SetShipmentState => 0x08,
            Self::SpiFlashRead => 0x10,
            Self::SetNfcIrMcuConfig => 0x21,
            Self::SetNfcIrMcuState => 0x22,
            Self::SetPlayerLights => 0x30,
            Self::Enable6AxisSensor => 0x40,
            Self::EnableVibration => 0x48,
            // NOTE: Case of returning this variant must not happen.
            Self::Empty => 0x00,
        }
    }
}
