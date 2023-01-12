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

impl SwitchResponse {
    pub fn from_subcommand(subcommand: Subcommand) -> Self {
        SwitchResponse::NoData
    }

    pub fn to_byte() -> u8 {
        0x00
    }
}

// Processes outgoing messages from the controller to the host(Nintendo Switch).
#[derive(Clone, Copy, Debug)]
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
#[derive(Clone, Copy, Debug)]
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
