use bytes::BytesMut;

#[derive(Debug, Default)]
pub struct SpiFlashConfig {
    pub buffer: Option<BytesMut>,
    pub size: Option<usize>,
    pub reset: bool,
}

#[derive(Clone, Debug)]
pub struct SpiFlash {
    buf: BytesMut,
}

impl SpiFlash {
    pub fn new() -> Self {
        Self::with_config(Default::default()).unwrap()
    }

    pub fn with_config(config: SpiFlashConfig) -> Option<Self> {
        let size = match config.size {
            Some(size) => std::cmp::max(size, 0x80000),
            None => 0x80000,
        };
        let mut should_reset = false;
        let mut buf = match config.buffer {
            Some(buf) => {
                if buf.len() != size {
                    return None;
                }
                if config.reset {
                    should_reset = true;
                }
                buf
            }
            None => {
                should_reset = true;
                let mut buf = BytesMut::with_capacity(size);
                buf.resize(size, 0xFF);
                buf
            }
        };
        if should_reset {
            // L-stick factory calibration
            buf[0x603D..0x6046]
                .copy_from_slice(&[0x00, 0x07, 0x70, 0x00, 0x08, 0x80, 0x00, 0x07, 0x70]);
            // R-stick factory calibration
            buf[0x6046..0x604F]
                .copy_from_slice(&[0x00, 0x08, 0x80, 0x00, 0x07, 0x70, 0x00, 0x07, 0x70]);
        }
        Some(Self { buf })
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
        self.buf.as_ref()
    }
}
