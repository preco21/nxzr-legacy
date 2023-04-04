pub struct SendDelay(Option<u8>);

impl SendDelay {
    fn to_byte(&self) -> f64 {
        let Some(mode) = self.0 else {
            // Subcommands replies only
            return f64::INFINITY;
        };
        match mode {
            0x3F => 1.0,
            0x21 => f64::INFINITY,
            // FIXME: revisit later for freq adjustment on procon 1/120
            0x30 => 1.0 / 60.0,
            0x31 => 1.0 / 60.0,
            // Unknown mode resorts to default, assuming it 1/15
            _ => 1.0 / 15.0,
        }
    }
}
