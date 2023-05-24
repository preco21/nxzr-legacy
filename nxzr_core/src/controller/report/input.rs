use super::{subcommand::Subcommand, ReportError};
use crate::controller::ControllerType;
use bytes::BytesMut;
use strum::Display;

// Ref: https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#input-reports
#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum InputReportId {
    // 0x3F Default input reports
    Default,
    // 0x21 Standard input reports used for subcommand replies
    Standard,
    // 0x30 Standard full mode - input reports with IMU data instead of subcommand replies
    Imu,
    // 0x31 Standard full mode - input reports with NFC/IR data plus to IMU data
    NfcIrMcu,
    // 0x32, 0x33 Unknown ids - treats as standard full input reports, maybe for handshake?
    Unknown1,
    Unknown2,
}

impl InputReportId {
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x3F => Some(Self::Default),
            0x21 => Some(Self::Standard),
            0x30 => Some(Self::Imu),
            0x31 => Some(Self::NfcIrMcu),
            0x32 => Some(Self::Unknown1),
            0x33 => Some(Self::Unknown2),
            _ => None,
        }
    }

    pub fn to_byte(&self) -> u8 {
        match self {
            Self::Default => 0x3F,
            Self::Standard => 0x21,
            Self::Imu => 0x30,
            Self::NfcIrMcu => 0x31,
            Self::Unknown1 => 0x32,
            Self::Unknown2 => 0x33,
        }
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

// Length of 50 is a standard input report size in format.
// See: https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#standard-input-report-format
const REPORT_MIN_LEN: usize = 50;
const SUBCOMMAND_OFFSET: usize = 16;

// Processes outgoing messages from the controller to the host(Nintendo Switch).
#[derive(Clone, Debug)]
pub struct InputReport {
    buf: BytesMut,
}

impl InputReport {
    pub fn new() -> Self {
        let mut buf = BytesMut::with_capacity(363);
        buf.resize(363, 0x00);
        buf[0] = 0xA1;
        Self { buf }
    }

    pub fn with_raw(data: BytesMut) -> Result<Self, ReportError> {
        if data.len() < REPORT_MIN_LEN {
            return Err(ReportError::TooShort);
        }
        let [0xA1, ..] = &data[..] else {
            return Err(ReportError::Malformed);
        };
        Ok(Self { buf: data })
    }

    pub fn fill_default_report(&mut self, controller_type: ControllerType) {
        // Ref: https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#input-0x3f
        self.buf[1..3].copy_from_slice(&[0x28, 0xCA, 0x08]);
        match controller_type {
            ControllerType::JoyConL | ControllerType::JoyConR => {
                self.buf[4..11].copy_from_slice(&[0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80]);
            }
            ControllerType::ProController => {
                self.buf[4..11].copy_from_slice(&[0x40, 0x8A, 0x4F, 0x8A, 0xD0, 0x7E, 0xDF, 0x7F]);
            }
        }
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

    pub fn input_report_id(&self) -> Option<InputReportId> {
        InputReportId::from_byte(self.buf[1])
    }

    pub fn set_input_report_id(&mut self, id: InputReportId) {
        self.buf[1] = id.to_byte();
    }

    pub fn set_timer(&mut self, timer: u64) {
        // Sets input report timer [0x00-0xFF], usually set by the transport
        self.buf[2] = (timer % 0x100) as u8;
    }

    pub fn set_misc(&mut self) {
        // Indicates battery level + connection info
        self.buf[3] = 0x8E;
    }

    pub fn set_button(&mut self, button_status: &[u8; 3]) {
        self.buf[4..7].copy_from_slice(button_status);
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
        // FIXME: revisit -> handle gyro
        for i in 14..50 {
            self.buf[i] = 0x00;
        }
    }

    // Returns `true` if the total data length matches 313
    pub fn set_ir_nfc_data(&mut self, data: &[u8]) -> Result<bool, ReportError> {
        if data.len() > 313 {
            return Err(ReportError::OutOfBounds);
        }
        self.buf[50..50 + data.len()].copy_from_slice(data);
        Ok(data.len() == 313)
    }

    pub fn response_subcommand(&self) -> Option<Subcommand> {
        Subcommand::from_byte(self.buf[15])
    }

    pub fn set_response_subcommand(&mut self, id: Subcommand) -> Result<(), ReportError> {
        self.buf[15] = id.to_byte();
        Ok(())
    }

    pub fn sub_0x02_device_info(
        &mut self,
        mac_addr: [u8; 6],
        fm_version: Option<[u8; 2]>,
        controller_type: ControllerType,
    ) -> Result<(), ReportError> {
        let fm_version = fm_version.unwrap_or([0x04, 0x00]);
        self.set_response_subcommand(Subcommand::RequestDeviceInfo)?;
        self.buf[SUBCOMMAND_OFFSET..SUBCOMMAND_OFFSET + 2].copy_from_slice(&fm_version);
        self.buf[SUBCOMMAND_OFFSET + 2] = controller_type.id();
        self.buf[SUBCOMMAND_OFFSET + 3] = 0x02;
        self.buf[SUBCOMMAND_OFFSET + 4..SUBCOMMAND_OFFSET + 10].copy_from_slice(&mac_addr);
        self.buf[SUBCOMMAND_OFFSET + 10] = 0x01;
        // FIXME: To figure out SPI settings to set controller colors
        // https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/spi_flash_notes.md#x6000-factory-configuration-and-calibration
        // self.buf[SUBCOMMAND_OFFSET + 11] = controller_type.device_info_color();
        self.buf[SUBCOMMAND_OFFSET + 11] = 0x00;
        Ok(())
    }

    pub fn sub_0x10_spi_flash_read(
        &mut self,
        offset: u64,
        size: u8,
        data: &[u8],
    ) -> Result<(), ReportError> {
        if size > 0x1D || data.len() != size.into() {
            return Err(ReportError::OutOfBounds);
        }
        // Creates input report data with spi flash read subcommand
        self.set_response_subcommand(Subcommand::SpiFlashRead)?;
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
        commands: &[TriggerButtonsElapsedTimeCommand],
    ) -> Result<(), ReportError> {
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

    pub fn as_buf(&self) -> &[u8] {
        let Some(id) = self.input_report_id() else {
            return &self.buf[..51];
        };
        match id {
            InputReportId::Default | InputReportId::Standard => &self.buf[..51],
            InputReportId::Imu => &self.buf[..50],
            InputReportId::NfcIrMcu | InputReportId::Unknown1 | InputReportId::Unknown2 => {
                &self.buf[..363]
            }
        }
    }
}

impl AsRef<[u8]> for InputReport {
    fn as_ref(&self) -> &[u8] {
        &self.buf
    }
}

impl AsMut<[u8]> for InputReport {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.buf
    }
}
