use strum::Display;

// FIXME: remove unknown command
#[derive(Clone, Copy, Debug, Default, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Subcommand {
    #[default]
    Unknown,
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
}

impl Subcommand {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0x02 => Self::RequestDeviceInfo,
            0x03 => Self::SetInputReportMode,
            0x04 => Self::TriggerButtonsElapsedTime,
            0x08 => Self::SetShipmentState,
            0x10 => Self::SpiFlashRead,
            0x21 => Self::SetNfcIrMcuConfig,
            0x22 => Self::SetNfcIrMcuState,
            0x30 => Self::SetPlayerLights,
            0x40 => Self::Enable6AxisSensor,
            0x48 => Self::EnableVibration,
            _ => Self::Unknown,
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
            _ => panic!("Unknown subcommand cannot be converted to a byte."),
        }
    }

    pub fn try_to_byte(&self) -> Option<u8> {
        if let Self::Unknown = self {
            return None;
        }
        Some(self.to_byte())
    }
}
