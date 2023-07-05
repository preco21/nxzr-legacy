// Source: https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/imu_sensor_notes.md#:~:text=gyro_vector_component%20%3D%20gyro_raw_component%20*%200.070f%20(%3D4588/65535)
// The equation is: gyro_vector_component = gyro_raw_component * G_GAIN / SENSOR_RES
// Where `SENSOR_RES` is 16bit, thus `65535`, `G_GAIN` is the degrees per second sensitivity range.
const GYROSCOPE_COEFF: f32 = 0.07;
const GYROSCOPE_SENSITIVITY_MULTIPLIER: f32 = 57.3;

// Minimum value must be greater than 0.
const DEFAULT_SENSITIVITY: i32 = 3000;

/// 6-Axis sensor state
/// https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/imu_sensor_notes.md
///
/// TODO: Currently only supports for gyro y/z axes state.
#[derive(Clone, Debug)]
pub struct ImuState {
    x: i32,
    y: i32,
    // z: u16,
}

impl ImuState {
    // TODO: add invert axes options
    pub fn new() -> Self {
        Self {
            x: 0,
            y: 0,
            /* z: 0 */
        }
    }

    pub fn set_horizontal(&mut self, x: u16) {
        self.x = if x & 0x8000 != 0 {
            (x ^ 0xFFFF) as i32 * -1 - 1
        } else {
            x as i32
        };
    }

    pub fn set_vertical(&mut self, y: u16) {
        self.y = if y & 0x8000 != 0 {
            (y ^ 0xFFFF) as i32 * -1 - 1
        } else {
            y as i32
        };
    }

    pub fn to_buf(&self) -> [u8; 36] {
        let gyro_x: i32 = 0;
        let gyro_y: i32 = ((self.y / DEFAULT_SENSITIVITY) as f32 * GYROSCOPE_SENSITIVITY_MULTIPLIER
            / GYROSCOPE_COEFF) as i32;
        let gyro_z: i32 = (-(self.x / DEFAULT_SENSITIVITY) as f32
            * GYROSCOPE_SENSITIVITY_MULTIPLIER
            / GYROSCOPE_COEFF) as i32;
        // The 6-axis data is repeated 3 times.
        let mut buf = [0u8; 36];
        // Gyro 1 / round 1
        buf[6] = (gyro_x & 0xFF) as u8;
        buf[7] = ((gyro_x >> 8) & 0xFF) as u8;
        // Gyro 1 / round 2
        buf[18] = (gyro_x & 0xFF) as u8;
        buf[19] = ((gyro_x >> 8) & 0xFF) as u8;
        // Gyro 1 / round 3
        buf[30] = (gyro_x & 0xFF) as u8;
        buf[31] = ((gyro_x >> 8) & 0xFF) as u8;
        // Gyro 2 / round 1
        buf[8] = (gyro_y & 0xFF) as u8;
        buf[9] = ((gyro_y >> 8) & 0xFF) as u8;
        // Gyro 2 / round 2
        buf[20] = (gyro_y & 0xFF) as u8;
        buf[21] = ((gyro_y >> 8) & 0xFF) as u8;
        // Gyro 2 / round 3
        buf[32] = (gyro_y & 0xFF) as u8;
        buf[33] = ((gyro_y >> 8) & 0xFF) as u8;
        // Gyro 3 / round 1
        buf[10] = (gyro_z & 0xFF) as u8;
        buf[11] = ((gyro_z >> 8) & 0xFF) as u8;
        // Gyro 3 / round 2
        buf[22] = (gyro_z & 0xFF) as u8;
        buf[23] = ((gyro_z >> 8) & 0xFF) as u8;
        // Gyro 3 / round 3
        buf[34] = (gyro_z & 0xFF) as u8;
        buf[35] = ((gyro_z >> 8) & 0xFF) as u8;
        buf
    }
}
