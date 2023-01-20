#[derive(Clone, Debug)]
pub struct SpiFlash {
    buf: Vec<u8>,
}

impl SpiFlash {
    pub fn new() -> Self {
        Self::with_size(0x80000)
    }

    pub fn with_size(size: usize) -> Self {
        let mut inst = Self {
            buf: vec![0xFF; size],
        };
        inst.reset_stick_calibration_with_factory_default();
        inst
    }

    pub fn with_raw(buf: impl AsRef<[u8]>, size: Option<usize>, reset: bool) -> Option<Self> {
        let buf = buf.as_ref();
        let size_u = match size {
            Some(size) => size,
            None => 0x80000,
        };
        if buf.len() != size_u {
            return None;
        }
        let mut inst = Self { buf: buf.to_vec() };
        if reset {
            inst.reset_stick_calibration_with_factory_default();
        }
        Some(inst)
    }

    pub fn reset_stick_calibration_with_factory_default(&mut self) {
        // L-stick factory calibration
        self.buf[0x603D..0x6046]
            .copy_from_slice(&[0x00, 0x07, 0x70, 0x00, 0x08, 0x80, 0x00, 0x07, 0x70]);
        // R-stick factory calibration
        self.buf[0x6046..0x604F]
            .copy_from_slice(&[0x00, 0x08, 0x80, 0x00, 0x07, 0x70, 0x00, 0x07, 0x70]);
    }

    pub fn factory_l_stick_calibration(&self) -> &[u8] {
        &self.buf[0x603D..0x6046]
    }

    pub fn factory_r_stick_calibration(&self) -> &[u8] {
        &self.buf[0x6046..0x604F]
    }

    pub fn user_l_stick_calibration(&self) -> Option<&[u8]> {
        let Some(&cal1) = self.buf.get(0x8010) else {
            return None;
        };
        let Some(&cal2) = self.buf.get(0x8011) else {
            return None;
        };
        // Check if the calibration data is available
        if cal1 == 0xB2 && cal2 == 0xA1 {
            Some(&self.buf[0x8012..0x801B])
        } else {
            None
        }
    }

    pub fn user_r_stick_calibration(&self) -> Option<&[u8]> {
        let Some(&cal1) = self.buf.get(0x801B) else {
            return None;
        };
        let Some(&cal2) = self.buf.get(0x801C) else {
            return None;
        };
        // Check if the calibration data is available
        if cal1 == 0xB2 && cal2 == 0xA1 {
            Some(&self.buf[0x801D..0x8026])
        } else {
            None
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.buf.as_slice()
    }
}
