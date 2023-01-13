use strum::Display;

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
            _ => {
                panic!("Invalid subcommand cannot be converted to a byte.")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SwitchResponseKind {
    NotInitialized,
    NoData,
    Malformed,
    TooShort,
    UnknownSubcommand,
    Subcommand(Subcommand),
}

#[derive(Clone, Debug)]
pub struct SwitchResponse {
    pub kind: SwitchResponseKind,
    pub payload: Option<Vec<u8>>,
    pub subcommand: Option<Vec<u8>>,
    pub subcommand_id: Option<u8>,
}

impl SwitchResponse {
    pub fn from_data(data: Option<Vec<u8>>) -> Self {
        Self::from_data_with_length(data, None)
    }

    pub fn from_data_with_length(data: Option<Vec<u8>>, length: Option<usize>) -> Self {
        if data.is_none() {
            return Self {
                kind: SwitchResponseKind::NoData,
                payload: None,
                subcommand: None,
                subcommand_id: None,
            };
        }
        let data_unwrapped = data.unwrap();
        if data_unwrapped.len() < 13 || data_unwrapped.len() < length.unwrap_or(50) {
            return Self {
                kind: SwitchResponseKind::TooShort,
                payload: None,
                subcommand: None,
                subcommand_id: None,
            };
        }
        // All output reports are prepended with 0xA2
        if data_unwrapped[0] != 0xA2 {
            return Self {
                kind: SwitchResponseKind::Malformed,
                payload: None,
                subcommand: None,
                subcommand_id: None,
            };
        }
        let mut payload = vec![0; 11];
        payload.clone_from_slice(&data_unwrapped[..11]);
        let mut subcommand: Vec<u8> = Vec::new();
        subcommand.extend_from_slice(&data_unwrapped[11..]);
        let subcommand_id = subcommand[0];
        Self {
            kind: SwitchResponseKind::Subcommand(Subcommand::from_byte(subcommand_id)),
            payload: Some(payload),
            subcommand: Some(subcommand),
            subcommand_id: Some(subcommand_id),
        }
    }
}
