use strum::{Display, IntoStaticStr};

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum ReportError {
    TooShortDataLength,
    Malformed,
    UnsupportedReportId,
    UnsupportedSubcommand,
    OutOfRange,
    Invariant,
}

impl std::error::Error for ReportError {}

pub type ReportResult<T> = Result<T, ReportError>;

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
        // 50 is a standard input report size in format: https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#standard-input-report-format
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

    pub fn subcommand_reply_data() {}

    pub fn input_report_id() {}

    pub fn set_input_report_id() {}

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

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum OutputReportId {
    Unknown,
    SubCommand,
    RumbleOnly,
    RequestIrNfcMcu,
}

impl OutputReportId {
    pub fn from_byte(byte: u8) -> OutputReportId {
        match byte {
            0x01 => OutputReportId::SubCommand,
            0x10 => OutputReportId::RumbleOnly,
            0x11 => OutputReportId::RequestIrNfcMcu,
            _ => OutputReportId::Unknown,
        }
    }

    pub fn to_byte(&self) -> u8 {
        match self {
            OutputReportId::SubCommand => 0x01,
            OutputReportId::RumbleOnly => 0x10,
            OutputReportId::RequestIrNfcMcu => 0x11,
            _ => panic!("Unknown output report id cannot be converted to a byte."),
        }
    }

    pub fn try_to_byte(&self) -> Option<u8> {
        if let Self::Unknown = self {
            return None;
        }
        Some(self.to_byte())
    }
}

// Processes incoming messages from the host(Nintendo Switch).
#[derive(Clone, Debug)]
pub struct OutputReport {
    data: Vec<u8>,
}

impl OutputReport {
    pub fn new() -> Self {
        let mut data: Vec<u8> = vec![0x00; 50];
        data[0] = 0xA2;
        Self { data }
    }

    pub fn with_data(data: impl AsRef<[u8]>) -> ReportResult<Self> {
        Self::with_data_and_length(data, 12)
    }

    pub fn with_data_and_length(data: impl AsRef<[u8]>, length: usize) -> ReportResult<Self> {
        let data_ref = data.as_ref();
        let min_len = std::cmp::max(length, 12);
        if data_ref.len() < min_len {
            return Err(ReportError::TooShortDataLength);
        }
        if data_ref[0] != 0xA2 {
            return Err(ReportError::Malformed);
        }
        Ok(Self {
            data: data_ref.to_vec(),
        })
    }

    pub fn output_report_id(&self) -> OutputReportId {
        OutputReportId::from_byte(self.data[1])
    }

    pub fn set_output_report_id(&mut self, id: OutputReportId) -> ReportResult<()> {
        match id.try_to_byte() {
            Some(byte) => {
                self.data[1] = byte;
                Ok(())
            }
            None => Err(ReportError::UnsupportedReportId),
        }
    }

    pub fn timer(&self) -> u8 {
        self.data[2]
    }

    // Sets output report timer between [0x0, 0xF]
    pub fn set_timer(&mut self, timer: u64) -> ReportResult<()> {
        self.data[2] = u8::try_from(timer % 0x10).unwrap();
        Ok(())
    }

    pub fn rumble_data(&self) -> &[u8] {
        &self.data[3..11]
    }

    pub fn subcommand(&self) -> Subcommand {
        Subcommand::from_byte(self.data[11])
    }

    pub fn set_subcommand(&mut self, subcommand: Subcommand) -> ReportResult<()> {
        match subcommand.try_to_byte() {
            Some(byte) => {
                self.data[11] = byte;
                Ok(())
            }
            None => Err(ReportError::UnsupportedSubcommand),
        }
    }

    pub fn subcommand_data(&self) -> ReportResult<&[u8]> {
        if self.data.len() < 13 {
            return Err(ReportError::TooShortDataLength);
        }
        Ok(&self.data[12..])
    }

    pub fn set_subcommand_data(&mut self, data: impl AsRef<[u8]>) -> ReportResult<()> {
        let data_ref = data.as_ref();
        self.data
            .splice(12..12 + data_ref.len(), data_ref.iter().cloned());
        Ok(())
    }

    pub fn sub_0x10_spi_flash_read(&mut self, offset: u32, size: u8) -> ReportResult<()> {
        if size > 0x1D || u32::from(size) + offset > 0x80000 {
            return Err(ReportError::OutOfRange);
        }
        self.set_output_report_id(OutputReportId::SubCommand)?;
        self.set_subcommand(Subcommand::SpiFlashRead)?;
        for i in 12..12 + 4 {
            self.data[i] = u8::try_from(offset % 0x100).unwrap();
            offset = offset / 0x100;
        }
        self.data[16] = size;
        Ok(())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data.as_slice()
    }
}
