pub struct SendDelay(Option<u8>);

impl SendDelay {
    pub fn new(mode: Option<u8>) -> Self {
        Self(mode)
    }

    pub fn to_byte(&self) -> Option<f64> {
        let Some(mode) = self.0 else {
            // Subcommands replies only
            return Some(f64::INFINITY);
        };
        match mode {
            0x3F => Some(1.0),
            0x21 => Some(f64::INFINITY),
            // FIXME: revisit later for freq adjustment on procon 1/120
            0x30 => Some(1.0 / 60.0),
            0x31 => Some(1.0 / 60.0),
            // Unknown mode resorts to default, assuming it as 1/15
            _ => None,
        }
    }
}
