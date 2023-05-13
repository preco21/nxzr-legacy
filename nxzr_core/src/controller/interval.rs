#[derive(Debug, Clone)]
pub struct SendInterval(pub Option<u8>);

impl SendInterval {
    pub fn new(mode: Option<u8>) -> Self {
        Self(mode)
    }

    pub fn to_byte(&self) -> Option<f64> {
        let Some(mode) = self.0 else {
            // For initial interval when no `mode` specified or subcommands replies
            return Some(f64::INFINITY);
        };
        match mode {
            0x3F => Some(1.0),
            0x21 => Some(f64::INFINITY),
            // FIXME: revisit later for freq adjustment on procon 1/120
            0x30 => Some(1.0 / 15.0),
            0x31 => Some(1.0 / 15.0),
            // 0x30 => Some(1.0 / 60.0),
            // 0x31 => Some(1.0 / 60.0),
            // Unknown `mode` should be handled by caller and resort to the default interval.
            _ => None,
        }
    }

    pub fn default_byte() -> f64 {
        1.0 / 15.0
    }
}
