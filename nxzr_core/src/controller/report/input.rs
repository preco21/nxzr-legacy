use super::{subcommand::Subcommand, ReportError, ReportResult};
use crate::controller::kind::ControllerInfo;
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
        Self::with_data_and_len(data, 50)
    }

    pub fn with_data_and_len(data: impl AsRef<[u8]>, len: usize) -> ReportResult<Self> {
        let data_r = data.as_ref();
        let min_len = std::cmp::max(len, 50);
        if data_r.len() < min_len {
            return Err(ReportError::TooShort);
        }
        if data_r[0] != 0xA1 {
            return Err(ReportError::Malformed);
        }
        Ok(Self {
            data: data_r.to_vec(),
        })
    }

    pub fn clear_subcommand(&mut self) {
        // Clear subcommand reply data of 0x21 input reports
        for i in 14..51 {
            self.data[i] = 0x00;
        }
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

    pub fn set_input_report_id(&mut self, id: InputReportId) -> ReportResult<()> {
        match id.try_to_byte() {
            Some(byte) => {
                self.data[1] = byte;
                Ok(())
            }
            None => Err(ReportError::UnsupportedReportId),
        }
    }

    pub fn set_timer(&mut self, timer: u64) {
        // Sets input report timer [0x00-0xFF], usually set by the transport
        self.data[2] = u8::try_from(timer % 0x100).unwrap();
    }

    pub fn set_misc(&mut self) {
        // Indicates battery level + connection info
        self.data[3] = 0x8E;
    }

    pub fn set_button_status(&mut self, button_status: [u8; 3]) {
        self.data.splice(4..7, button_status);
    }

    pub fn set_analog_stick(
        &mut self,
        left_stick_status: Option<[u8; 3]>,
        right_stick_status: Option<[u8; 3]>,
    ) {
        match left_stick_status {
            Some(bytes) => self.set_left_analog_stick(bytes),
            None => self.set_left_analog_stick([0, 0, 0]),
        }
        match right_stick_status {
            Some(bytes) => self.set_right_analog_stick(bytes),
            None => self.set_right_analog_stick([0, 0, 0]),
        }
    }

    pub fn set_left_analog_stick(&mut self, stick_status: [u8; 3]) {
        self.data.splice(7..10, stick_status);
    }

    pub fn set_right_analog_stick(&mut self, stick_status: [u8; 3]) {
        self.data.splice(10..13, stick_status);
    }

    pub fn set_vibrator_input(&mut self) {
        // TODO: revisit
        self.data[13] = 0x80
    }

    pub fn ack(&self) -> u8 {
        return self.data[14];
    }

    pub fn set_ack(&mut self, ack: u8) {
        self.data[14] = ack;
    }

    pub fn set_6axis_data(&mut self) {
        // FIXME: revisit
        for i in 14..50 {
            self.data[i] = 0x00;
        }
    }

    // Returns `true` if the total data length matches 313
    pub fn set_ir_nfc_data(&mut self, data: impl AsRef<[u8]>) -> ReportResult<bool> {
        let data_r = data.as_ref();
        if data_r.len() > 313 {
            return Err(ReportError::OutOfBounds);
        }
        self.data
            .splice(50..50 + data_r.len(), data_r.iter().cloned());
        Ok(data_r.len() == 313)
    }

    pub fn reply_to_subcommand_id(&self) -> Subcommand {
        Subcommand::from_byte(self.data[15])
    }

    pub fn set_reply_to_subcommand_id(&mut self, id: Subcommand) -> ReportResult<()> {
        match id.try_to_byte() {
            Some(byte) => {
                self.data[15] = byte;
                Ok(())
            }
            None => Err(ReportError::UnsupportedSubcommand),
        }
    }

    pub fn sub_0x02_device_info(
        &mut self,
        mac_addr: [u8; 6],
        fm_version: Option<[u8; 2]>,
        controller_info: ControllerInfo,
    ) -> ReportResult<()> {
        let fm_version_u = match fm_version {
            Some(version) => version,
            None => [0x04, 0x00],
        };
        self.set_reply_to_subcommand_id(Subcommand::RequestDeviceInfo)?;
        self.data.splice(16..16 + 2, fm_version_u);
        self.data[16 + 2] = controller_info.id;
        self.data[16 + 3] = 0x02;
        self.data.splice(16 + 4..16 + 10, mac_addr);
        self.data[16 + 10] = 0x01;
        self.data[16 + 11] = 0x01;
        Ok(())
    }

    pub fn sub_0x10_spi_flash_read(
        &mut self,
        offset: u32,
        size: u8,
        data: impl AsRef<[u8]>,
    ) -> ReportResult<()> {
        let data_r = data.as_ref();
        if size > 0x1D || data_r.len() != size.into() {
            return Err(ReportError::OutOfBounds);
        }
        // Creates input report data with spi flash read subcommand
        self.set_reply_to_subcommand_id(Subcommand::SpiFlashRead)?;
        let mut cur_offset = offset;
        // Write offset to data
        for i in 16..16 + 4 {
            self.data[i] = u8::try_from(cur_offset % 0x100).unwrap();
            cur_offset = cur_offset / 0x100;
        }
        self.data[20] = size;
        self.data
            .splice(21..21 + data_r.len(), data_r.iter().cloned());
        Ok(())
    }

    pub fn sub_0x04_trigger_buttons_elapsed_time(&self) {}

    pub fn as_bytes(&self) -> &[u8] {
        &self.data.as_slice()
    }
}
