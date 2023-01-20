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
    buf: Vec<u8>,
}

impl OutputReport {
    pub fn new() -> Self {
        let mut buf: Vec<u8> = vec![0x00; 50];
        buf[0] = 0xA2;
        Self { buf }
    }

    pub fn with_raw(buf: impl AsRef<[u8]>, report_size: Option<usize>) -> ReportResult<Self> {
        let buf_r = buf.as_ref();
        let min_len = match report_size {
            Some(report_size) => std::cmp::max(report_size, 12),
            None => 12,
        };
        if buf_r.len() < min_len {
            return Err(ReportError::TooShort);
        }
        if buf_r[0] != 0xA2 {
            return Err(ReportError::Malformed);
        }
        Ok(Self {
            buf: buf_r.to_vec(),
        })
    }

    pub fn output_report_id(&self) -> OutputReportId {
        OutputReportId::from_byte(self.buf[1])
    }

    pub fn set_output_report_id(&mut self, id: OutputReportId) -> ReportResult<()> {
        match id.try_to_byte() {
            Some(byte) => {
                self.buf[1] = byte;
                Ok(())
            }
            None => Err(ReportError::UnsupportedReportId),
        }
    }

    pub fn timer(&self) -> u8 {
        self.buf[2]
    }

    pub fn set_timer(&mut self, timer: u64) {
        // Sets output report timer between [0x0, 0xF]
        self.buf[2] = (timer % 0x10) as u8;
    }

    pub fn rumble_data(&self) -> &[u8] {
        &self.buf[3..11]
    }

    pub fn subcommand(&self) -> Subcommand {
        Subcommand::from_byte(self.buf[11])
    }

    pub fn set_subcommand(&mut self, subcommand: Subcommand) -> ReportResult<()> {
        match subcommand.try_to_byte() {
            Some(byte) => {
                self.buf[11] = byte;
                Ok(())
            }
            None => Err(ReportError::UnsupportedSubcommand),
        }
    }

    pub fn subcommand_data(&self) -> ReportResult<&[u8]> {
        let Some(slice) = self.buf.get(12..) else {
            return Err(ReportError::NoData);
        };
        Ok(slice)
    }

    pub fn set_subcommand_data(&mut self, data: impl AsRef<[u8]>) {
        let data_r = data.as_ref();
        self.buf[12..12 + data_r.len()].copy_from_slice(data_r);
    }

    pub fn sub_0x10_spi_flash_read(&mut self, offset: u32, size: u8) -> ReportResult<()> {
        if size > 0x1D || u32::from(size) + offset > 0x80000 {
            return Err(ReportError::OutOfBounds);
        }
        // Creates output report data with spi flash read subcommand
        self.set_output_report_id(OutputReportId::SubCommand)?;
        self.set_subcommand(Subcommand::SpiFlashRead)?;
        let mut cur_offset = offset;
        for i in 12..12 + 4 {
            self.buf[i] = (cur_offset % 0x100) as u8;
            cur_offset = cur_offset / 0x100;
        }
        self.buf[16] = size;
        Ok(())
    }

    pub fn data(&self) -> &[u8] {
        &self.buf.as_slice()
    }
}
