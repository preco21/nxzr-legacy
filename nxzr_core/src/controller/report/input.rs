use super::{ReportError, ReportResult};
use strum::Display;

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum InputReportId {
    Unknown,
    // 0x21 Standard input reports used for subcommand replies
    Standard,
    // 0x30 Full input reports with IMU data instead of subcommand replies
    Full,
    // 0x31 Full input reports with NFC/IR data plus to IMU data
    FullWithData,
}

impl InputReportId {
    pub fn from_byte(byte: u8) -> InputReportId {
        match byte {
            0x21 => InputReportId::Standard,
            0x30 => InputReportId::Full,
            0x31 => InputReportId::FullWithData,
            _ => InputReportId::Unknown,
        }
    }

    pub fn to_byte(&self) -> u8 {
        match self {
            InputReportId::Standard => 0x21,
            InputReportId::Full => 0x30,
            InputReportId::FullWithData => 0x31,
            _ => panic!("Unknown input report id cannot be converted to a byte."),
        }
    }

    pub fn try_to_byte(&self) -> Option<u8> {
        if let Self::Unknown = self {
            return None;
        }
        Some(self.to_byte())
    }
}

// Processes outgoing messages from the controller to the host(Nintendo Switch).
#[derive(Clone, Debug)]
pub struct InputReport {
    data: Vec<u8>,
}

impl InputReport {
    pub fn new() -> Self {
        let mut data: Vec<u8> = vec![0x00; 363];
        data[0] = 0xA1;
        Self { data }
    }

    pub fn with_data(data: impl AsRef<[u8]>) -> ReportResult<Self> {
        // Length of 50 is a standard input report size in format
        // See: https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#standard-input-report-format
        Self::with_data_and_length(data, 50)
    }

    pub fn with_data_and_length(data: impl AsRef<[u8]>, length: usize) -> ReportResult<Self> {
        let data_ref = data.as_ref();
        let min_len = std::cmp::max(length, 50);
        if data_ref.len() < min_len {
            return Err(ReportError::TooShortDataLength);
        }
        if data_ref[0] != 0xA1 {
            return Err(ReportError::Malformed);
        }
        Ok(Self {
            data: data_ref.to_vec(),
        })
    }

    pub fn clear_subcommand(&mut self) -> ReportResult<()> {
        for i in 14..51 {
            self.data[i] = 0x00;
        }
        Ok(())
    }

    pub fn stick_data(&self) -> &[u8] {
        &self.data[7..13]
    }

    pub fn subcommand_reply_data(&self) -> &[u8] {
        &self.data[16..51]
    }

    pub fn input_report_id(&self) -> InputReportId {
        InputReportId::from_byte(self.data[1])
    }

    pub fn set_input_report_id(&mut self, id: InputReportId) -> ReportResult<()> {}

    pub fn set_timer() {}

    pub fn set_misc() {}

    pub fn set_button_status() {}

    pub fn set_stick_status() {}

    pub fn set_left_analog_stick() {}

    pub fn set_right_analog_stick() {}

    pub fn set_vibrator_input() {}

    pub fn ack() {}

    pub fn set_ack() {}

    pub fn set_6axis_data() {}

    pub fn set_ir_nfc_data() {}

    pub fn reply_to_subcommand_id() {}

    // FIXME:
    // pub fn get_reply_to_subcommand_id() {}

    pub fn sub_0x02_device_info() {}

    pub fn sub_0x10_spi_flash_read() {}

    pub fn sub_0x04_trigger_buttons_elapsed_time() {}

    pub fn as_bytes(&self) -> &[u8] {
        &self.data.as_slice()
    }
}
