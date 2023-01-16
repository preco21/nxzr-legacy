use super::subcommand::Subcommand;
use super::{ReportError, ReportResult};
use strum::Display;

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

    pub fn set_timer(&mut self, timer: u64) {
        // Sets output report timer between [0x0, 0xF]
        self.data[2] = u8::try_from(timer % 0x10).unwrap();
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

    pub fn set_subcommand_data(&mut self, data: impl AsRef<[u8]>) {
        let data_ref = data.as_ref();
        self.data
            .splice(12..12 + data_ref.len(), data_ref.iter().cloned());
    }

    pub fn sub_0x10_spi_flash_read(&mut self, offset: u32, size: u8) -> ReportResult<()> {
        if size > 0x1D || u32::from(size) + offset > 0x80000 {
            return Err(ReportError::OutOfRange);
        }
        // Creates output report data with spi flash read subcommand
        let mut cur_offset = offset;
        self.set_output_report_id(OutputReportId::SubCommand)?;
        self.set_subcommand(Subcommand::SpiFlashRead)?;
        for i in 12..12 + 4 {
            self.data[i] = u8::try_from(cur_offset % 0x100).unwrap();
            cur_offset = cur_offset / 0x100;
        }
        self.data[16] = size;
        Ok(())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data.as_slice()
    }
}
