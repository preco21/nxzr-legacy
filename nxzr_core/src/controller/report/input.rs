use super::{subcommand::Subcommand, ReportError, ReportResult};
use crate::controller::ControllerType;
use strum::Display;

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum InputReportId {
    Unknown,
    // 0x3F Default input report for sending input packets to the host
    Default,
    // 0x21 Standard input reports used for subcommand replies
    Standard,
    // 0x30 Full input reports with IMU data instead of subcommand replies
    Imu,
    // 0x31 Full input reports with NFC/IR data plus to IMU data
    ImuWithNfcIrData,
}

impl InputReportId {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0x3F => Self::Default,
            0x21 => Self::Standard,
            0x30 => Self::Imu,
            0x31 => Self::ImuWithNfcIrData,
            _ => Self::Unknown,
        }
    }

    pub fn to_byte(&self) -> u8 {
        match self {
            Self::Standard => 0x21,
            Self::Imu => 0x30,
            Self::ImuWithNfcIrData => 0x31,
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

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TriggerButtonsElapsedTimeCommand {
    LeftTrigger(u32),
    RightTrigger(u32),
    ZLeftTrigger(u32),
    ZRightTrigger(u32),
    SLeftTrigger(u32),
    SRightTrigger(u32),
    Home(u32),
}

const SUBCOMMAND_OFFSET: usize = 16;

// Processes outgoing messages from the controller to the host(Nintendo Switch).
#[derive(Clone, Debug)]
pub struct InputReport {
    buf: Vec<u8>,
}

impl InputReport {
    pub fn new() -> Self {
        let mut buf: Vec<u8> = vec![0x00; 363];
        buf[0] = 0xA1;
        Self { buf }
    }

    pub fn with_raw(data: impl AsRef<[u8]>, report_size: Option<usize>) -> ReportResult<Self> {
        let buf = data.as_ref();
        // Length of 50 is a standard input report size in format
        // See: https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#standard-input-report-format
        let min_len = match report_size {
            Some(report_size) => std::cmp::max(report_size, 50),
            None => 50,
        };
        if buf.len() < min_len {
            return Err(ReportError::TooShort);
        }
        let [0xA1, ..] = buf else {
            return Err(ReportError::Malformed);
        };
        Ok(Self { buf: buf.to_vec() })
    }

    pub fn clear_subcommand(&mut self) {
        // Clear subcommand reply data of 0x21 input reports
        for i in 14..51 {
            self.buf[i] = 0x00;
        }
    }

    pub fn stick_data(&self) -> &[u8] {
        &self.buf[7..13]
    }

    pub fn subcommand_reply_data(&self) -> &[u8] {
        &self.buf[16..51]
    }

    pub fn input_report_id(&self) -> InputReportId {
        InputReportId::from_byte(self.buf[1])
    }

    pub fn set_input_report_id(&mut self, id: InputReportId) -> ReportResult<()> {
        match id.try_to_byte() {
            Some(byte) => {
                self.buf[1] = byte;
                Ok(())
            }
            None => Err(ReportError::UnsupportedReportId),
        }
    }

    pub fn set_timer(&mut self, timer: u64) {
        // Sets input report timer [0x00-0xFF], usually set by the transport
        self.buf[2] = (timer % 0x100) as u8;
    }

    pub fn set_misc(&mut self) {
        // Indicates battery level + connection info
        self.buf[3] = 0x8E;
    }

    pub fn set_button_status(&mut self, button_status: [u8; 3]) {
        self.buf[4..7].copy_from_slice(&button_status);
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
        self.buf[7..10].copy_from_slice(&stick_status);
    }

    pub fn set_right_analog_stick(&mut self, stick_status: [u8; 3]) {
        self.buf[10..13].copy_from_slice(&stick_status);
    }

    pub fn set_vibrator_input(&mut self) {
        self.buf[13] = 0x80
    }

    pub fn ack(&self) -> u8 {
        return self.buf[14];
    }

    pub fn set_ack(&mut self, ack: u8) {
        self.buf[14] = ack;
    }

    pub fn set_6axis_data(&mut self) {
        // FIXME: revisit
        for i in 14..50 {
            self.buf[i] = 0x00;
        }
    }

    // Returns `true` if the total data length matches 313
    pub fn set_ir_nfc_data(&mut self, data: impl AsRef<[u8]>) -> ReportResult<bool> {
        let data = data.as_ref();
        if data.len() > 313 {
            return Err(ReportError::OutOfBounds);
        }
        self.buf[50..50 + data.len()].copy_from_slice(data);
        Ok(data.len() == 313)
    }

    pub fn reply_to_subcommand_id(&self) -> Subcommand {
        Subcommand::from_byte(self.buf[15])
    }

    pub fn set_reply_to_subcommand_id(&mut self, id: Subcommand) -> ReportResult<()> {
        match id.try_to_byte() {
            Some(byte) => {
                self.buf[15] = byte;
                Ok(())
            }
            None => Err(ReportError::UnsupportedSubcommand),
        }
    }

    pub fn sub_0x02_device_info(
        &mut self,
        mac_addr: [u8; 6],
        fm_version: Option<[u8; 2]>,
        controller_type: ControllerType,
    ) -> ReportResult<()> {
        let Some(controller_id) = controller_type.try_id() else {
            return Err(ReportError::Invariant);
        };
        let fm_version_u = match fm_version {
            Some(version) => version,
            None => [0x04, 0x00],
        };
        self.set_reply_to_subcommand_id(Subcommand::RequestDeviceInfo)?;
        self.buf[SUBCOMMAND_OFFSET..SUBCOMMAND_OFFSET + 2].copy_from_slice(&fm_version_u);
        self.buf[SUBCOMMAND_OFFSET + 2] = controller_id;
        self.buf[SUBCOMMAND_OFFSET + 3] = 0x02;
        self.buf[SUBCOMMAND_OFFSET + 4..SUBCOMMAND_OFFSET + 10].copy_from_slice(&mac_addr);
        self.buf[SUBCOMMAND_OFFSET + 10] = 0x01;
        self.buf[SUBCOMMAND_OFFSET + 11] = 0x01;
        Ok(())
    }

    pub fn sub_0x10_spi_flash_read(
        &mut self,
        offset: u32,
        size: u8,
        data: impl AsRef<[u8]>,
    ) -> ReportResult<()> {
        let data = data.as_ref();
        if size > 0x1D || data.len() != size.into() {
            return Err(ReportError::OutOfBounds);
        }
        // Creates input report data with spi flash read subcommand
        self.set_reply_to_subcommand_id(Subcommand::SpiFlashRead)?;
        let mut cur_offset = offset;
        // Write offset to data
        for i in SUBCOMMAND_OFFSET..SUBCOMMAND_OFFSET + 4 {
            self.buf[i] = (cur_offset % 0x100) as u8;
            cur_offset = cur_offset / 0x100;
        }
        self.buf[20] = size;
        self.buf[21..21 + data.len()].copy_from_slice(data);
        Ok(())
    }

    pub fn sub_0x04_trigger_buttons_elapsed_time(
        &mut self,
        commands: impl AsRef<[TriggerButtonsElapsedTimeCommand]>,
    ) -> ReportResult<()> {
        let commands = commands.as_ref();
        const MAX_MS: u32 = 10 * 0xFFFF;
        let mut set = |offset: usize, ms: u32| {
            let value = (ms / 10) as u16;
            self.buf[SUBCOMMAND_OFFSET + offset] = (value & 0xFF) as u8;
            self.buf[SUBCOMMAND_OFFSET + offset + 1] = ((value & 0xFF00) >> 8) as u8;
        };
        for command in commands {
            match *command {
                TriggerButtonsElapsedTimeCommand::LeftTrigger(ms) => {
                    if ms > MAX_MS {
                        return Err(ReportError::Invariant);
                    }
                    set(0, ms);
                }
                TriggerButtonsElapsedTimeCommand::RightTrigger(ms) => {
                    if ms > MAX_MS {
                        return Err(ReportError::Invariant);
                    }
                    set(2, ms);
                }
                TriggerButtonsElapsedTimeCommand::ZLeftTrigger(ms) => {
                    if ms > MAX_MS {
                        return Err(ReportError::Invariant);
                    }
                    set(4, ms);
                }
                TriggerButtonsElapsedTimeCommand::ZRightTrigger(ms) => {
                    if ms > MAX_MS {
                        return Err(ReportError::Invariant);
                    }
                    set(6, ms);
                }
                TriggerButtonsElapsedTimeCommand::SLeftTrigger(ms) => {
                    if ms > MAX_MS {
                        return Err(ReportError::Invariant);
                    }
                    set(8, ms);
                }
                TriggerButtonsElapsedTimeCommand::SRightTrigger(ms) => {
                    if ms > MAX_MS {
                        return Err(ReportError::Invariant);
                    }
                    set(10, ms);
                }
                TriggerButtonsElapsedTimeCommand::Home(ms) => {
                    if ms > MAX_MS {
                        return Err(ReportError::Invariant);
                    }
                    set(12, ms);
                }
            };
        }
        Ok(())
    }

    pub fn data(&self) -> &[u8] {
        match self.input_report_id() {
            InputReportId::Standard => &self.buf[..51],
            InputReportId::Imu => &self.buf[..50],
            InputReportId::ImuWithNfcIrData => &self.buf[..363],
            _ => &self.buf[..51],
        }
    }
}
