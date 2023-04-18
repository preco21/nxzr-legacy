use super::subcommand::Subcommand;
use crate::{Error, ErrorKind, ReportErrorKind, Result};
use strum::Display;

// Ref: https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#output-reports
#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum OutputReportId {
    SubCommand,
    RumbleOnly,
    RequestIrNfcMcu,
}

impl OutputReportId {
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x01 => Some(Self::SubCommand),
            0x10 => Some(Self::RumbleOnly),
            0x11 => Some(Self::RequestIrNfcMcu),
            _ => None,
        }
    }

    pub fn to_byte(&self) -> u8 {
        match self {
            Self::SubCommand => 0x01,
            Self::RumbleOnly => 0x10,
            Self::RequestIrNfcMcu => 0x11,
        }
    }
}

const REPORT_MIN_LEN: usize = 11;

// Processes incoming messages from the host (Nintendo Switch).
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

    pub fn with_raw(data: &[u8]) -> Result<Self> {
        if data.len() < REPORT_MIN_LEN {
            return Err(Error::new(ErrorKind::Report(ReportErrorKind::TooShort)));
        }
        let [0xA2, ..] = data else {
            return Err(Error::new(ErrorKind::Report(ReportErrorKind::Malformed)));
        };
        Ok(Self { buf: data.to_vec() })
    }

    pub fn output_report_id(&self) -> Option<OutputReportId> {
        OutputReportId::from_byte(self.buf[1])
    }

    pub fn set_output_report_id(&mut self, id: OutputReportId) {
        self.buf[1] = id.to_byte();
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

    pub fn subcommand(&self) -> Option<Subcommand> {
        if self.buf.len() < 12 {
            Some(Subcommand::Empty)
        } else {
            Subcommand::from_byte(self.buf[11])
        }
    }

    pub fn set_subcommand(&mut self, subcommand: Subcommand) {
        self.buf[11] = subcommand.to_byte();
    }

    pub fn subcommand_data(&self) -> Result<&[u8]> {
        let Some(slice) = self.buf.get(12..) else {
            return Err(Error::new(ErrorKind::Report(ReportErrorKind::NoDataAvailable)));
        };
        Ok(slice)
    }

    pub fn set_subcommand_data(&mut self, data: &[u8]) {
        self.buf[12..12 + data.len()].copy_from_slice(data);
    }

    pub fn sub_0x10_spi_flash_read(&mut self, offset: u32, size: u8) -> Result<()> {
        if size > 0x1D || u32::from(size) + offset > 0x80000 {
            return Err(Error::new(ErrorKind::Report(ReportErrorKind::OutOfBounds)));
        }
        // Creates output report data with spi flash read subcommand
        self.set_output_report_id(OutputReportId::SubCommand);
        self.set_subcommand(Subcommand::SpiFlashRead);
        let mut cur_offset = offset;
        for i in 12..12 + 4 {
            self.buf[i] = (cur_offset % 0x100) as u8;
            cur_offset = cur_offset / 0x100;
        }
        self.buf[16] = size;
        Ok(())
    }

    pub fn data(&self) -> &[u8] {
        &self.buf[..]
    }
}
