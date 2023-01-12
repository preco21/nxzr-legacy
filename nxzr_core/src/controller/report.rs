use strum::{Display, EnumString};

#[derive(
    Clone, Copy, Default, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Display, EnumString,
)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Display, EnumString)]
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
    pub fn from_data(data: Option<Vec<u8>>, length: Option<usize>) -> Self {
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

// Processes outgoing messages from the controller to the host(Nintendo Switch).
#[derive(Clone, Debug)]
pub struct InputReport;

impl InputReport {
    pub fn clear_subcommand() {}
    pub fn get_stick_data() {}
    pub fn get_subcommand_reply_data() {}
    pub fn get_input_report_id() {}
    pub fn set_input_report_id() {}
    pub fn set_timer() {}
    pub fn set_misc() {}
    pub fn set_button_status() {}
    pub fn set_stick_status() {}
    pub fn set_left_analog_stick() {}
    pub fn set_right_analog_stick() {}
    pub fn set_vibrator_input() {}
    pub fn get_ack() {}
    pub fn set_ack() {}
    pub fn set_6axis_data() {}
    pub fn set_ir_nfc_data() {}
    pub fn reply_to_subcommand_id() {}
    pub fn get_reply_to_subcommand_id() {}
    pub fn sub_0x02_device_info() {}
    pub fn sub_0x10_spi_flash_read() {}
    pub fn sub_0x04_trigger_buttons_elapsed_time() {}

    pub fn into_bytes() -> Vec<u8> {
        Vec::new()
    }

    pub fn to_string() -> String {
        String::new()
    }
}

// Processes incoming messages from the host(Nintendo Switch).
#[derive(Clone, Debug)]
pub struct OutputReport;

impl OutputReport {
    pub fn get_output_report_id() {}

    pub fn set_output_report_id() {}

    pub fn get_timer() {}

    pub fn set_timer() {}

    pub fn get_rumble_data() {}

    pub fn get_subcommand() {}

    pub fn set_subcommand() {}

    pub fn get_subcommand_data() {}

    pub fn set_subcommand_data() {}

    pub fn sub_0x10_spi_flash_read() {}

    pub fn into_bytes() -> Vec<u8> {
        Vec::new()
    }

    pub fn to_string() -> String {
        String::new()
    }
}
