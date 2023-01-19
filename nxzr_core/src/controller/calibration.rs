pub struct StickCalibration {
    pub h_center: u16,
    pub v_center: u16,
    pub h_max_above_center: u16,
    pub v_max_above_center: u16,
    pub h_max_below_center: u16,
    pub v_max_below_center: u16,
}

impl StickCalibration {
    pub fn new(
        h_center: u16,
        v_center: u16,
        h_max_above_center: u16,
        v_max_above_center: u16,
        h_max_below_center: u16,
        v_max_below_center: u16,
    ) -> Self {
        Self {
            h_center,
            v_center,
            h_max_above_center,
            v_max_above_center,
            h_max_below_center,
            v_max_below_center,
        }
    }
}

pub struct LeftStickCalibration {
    stick_calibration: StickCalibration,
}

fn to_u16_bytes(bytes: impl AsRef<[u8]>) -> Vec<u16> {
    bytes
        .as_ref()
        .iter()
        .cloned()
        .map(|e| u16::from(e))
        .collect::<Vec<_>>()
}

impl LeftStickCalibration {
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Self {
        let bytes_padded = to_u16_bytes(bytes);
        let h_max_above_center: u16 = (bytes_padded[1] << 8) & 0xF00 | bytes_padded[0];
        let v_max_above_center = (bytes_padded[2] << 4) | (bytes_padded[1] >> 4);
        let h_center = (bytes_padded[4] << 8) & 0xF00 | bytes_padded[3];
        let v_center = (bytes_padded[5] << 4) | (bytes_padded[4] >> 4);
        let h_max_below_center = (bytes_padded[7] << 8) & 0xF00 | bytes_padded[6];
        let v_max_below_center = (bytes_padded[8] << 4) | (bytes_padded[7] >> 4);
        LeftStickCalibration {
            stick_calibration: StickCalibration::new(
                h_center,
                v_center,
                h_max_above_center,
                v_max_above_center,
                h_max_below_center,
                v_max_below_center,
            ),
        }
    }
}

struct RightStickCalibration {
    stick_calibration: StickCalibration,
}

// impl RightStickCalibration {
//     fn from_bytes(_9bytes: &[u8]) -> Self {
//         let h_center = (_9bytes[1] << 8) & 0xF00 | _9bytes[0] as i32
