use super::StateError;

#[derive(Debug, Default)]
pub struct StickStateConfig {
    pub horizontal: Option<u16>,
    pub vertical: Option<u16>,
    pub calibration: Option<StickCalibration>,
}

#[derive(Clone, Debug, Default)]
pub struct StickState {
    h_stick: u16,
    v_stick: u16,
    stick_cal: Option<StickCalibration>,
}

impl StickState {
    pub fn new() -> Self {
        Self::with_config(Default::default()).unwrap()
    }

    pub fn with_config(config: StickStateConfig) -> Result<Self, StateError> {
        let horizontal = match config.horizontal {
            Some(horizontal) => {
                if horizontal >= 0x1000 {
                    return Err(StateError::InvalidRange);
                } else {
                    horizontal
                }
            }
            None => 0,
        };
        let vertical = match config.vertical {
            Some(vertical) => {
                if vertical >= 0x1000 {
                    return Err(StateError::InvalidRange);
                } else {
                    vertical
                }
            }
            None => 0,
        };
        Ok(Self {
            h_stick: horizontal,
            v_stick: vertical,
            stick_cal: config.calibration,
        })
    }

    pub fn with_raw(
        bytes: [u8; 3],
        calibration: Option<StickCalibration>,
    ) -> Result<Self, StateError> {
        let stick_h = (bytes[0] as u16) | (((bytes[1] & 0xF) as u16) << 8);
        let stick_v = ((bytes[1] >> 4) as u16) | ((bytes[2] as u16) << 4);
        Self::with_config(StickStateConfig {
            horizontal: Some(stick_h),
            vertical: Some(stick_v),
            calibration,
            ..Default::default()
        })
    }

    pub fn horizontal(&self) -> u16 {
        self.h_stick
    }

    pub fn set_horizontal(&mut self, horizontal: u16) -> Result<(), StateError> {
        if horizontal >= 0x1000 {
            return Err(StateError::InvalidRange);
        }
        self.h_stick = horizontal;
        Ok(())
    }

    pub fn set_horizontal_scale(&mut self, scale: f32) -> Result<(), StateError> {
        let Some(ref stick_cal) = self.stick_cal else {
            return Err(StateError::NoCalibrationDataAvailable);
        };
        if scale.is_nan() || scale.abs() > 1.0 {
            return Err(StateError::InvalidScale);
        }
        self.h_stick = if scale.is_sign_positive() {
            stick_cal.h_center + (stick_cal.h_max_above_center as f32 * scale).round() as u16
        } else {
            stick_cal.h_center - (stick_cal.h_max_below_center as f32 * scale).round() as u16
        };
        Ok(())
    }

    pub fn vertical(&self) -> u16 {
        self.v_stick
    }

    pub fn set_vertical(&mut self, vertical: u16) -> Result<(), StateError> {
        if vertical >= 0x1000 {
            return Err(StateError::InvalidRange);
        }
        self.v_stick = vertical;
        Ok(())
    }

    pub fn set_vertical_scale(&mut self, scale: f32) -> Result<(), StateError> {
        let Some(ref stick_cal) = self.stick_cal else {
            return Err(StateError::NoCalibrationDataAvailable);
        };
        if scale.is_nan() || scale.abs() > 1.0 {
            return Err(StateError::InvalidScale);
        }
        self.v_stick = if scale.is_sign_positive() {
            stick_cal.v_center + (stick_cal.v_max_above_center as f32 * scale).round() as u16
        } else {
            stick_cal.v_center - (stick_cal.v_max_below_center as f32 * scale).round() as u16
        };
        Ok(())
    }

    pub fn is_center(&self, radius: Option<u16>) -> Result<bool, StateError> {
        let Some(ref stick_cal) = self.stick_cal else {
            return Err(StateError::NoCalibrationDataAvailable);
        };
        let radius = radius.unwrap_or(0);
        Ok((stick_cal.h_center - radius <= self.h_stick)
            && (self.h_stick <= stick_cal.h_center + radius)
            && (stick_cal.v_center - radius <= self.v_stick)
            && (self.v_stick <= stick_cal.v_center + radius))
    }

    pub fn reset_to_center(&mut self) -> Result<(), StateError> {
        let Some(ref stick_cal) = self.stick_cal else {
            return Err(StateError::NoCalibrationDataAvailable);
        };
        self.h_stick = stick_cal.h_center;
        self.v_stick = stick_cal.v_center;
        Ok(())
    }

    pub fn set_up(&mut self) -> Result<(), StateError> {
        let Some(ref stick_cal) = self.stick_cal else {
            return Err(StateError::NoCalibrationDataAvailable);
        };
        self.h_stick = stick_cal.h_center;
        self.v_stick = stick_cal.v_center + stick_cal.v_max_above_center;
        Ok(())
    }

    pub fn set_down(&mut self) -> Result<(), StateError> {
        let Some(ref stick_cal) = self.stick_cal else {
            return Err(StateError::NoCalibrationDataAvailable);
        };
        self.h_stick = stick_cal.h_center;
        self.v_stick = stick_cal.v_center - stick_cal.v_max_below_center;
        Ok(())
    }

    pub fn set_right(&mut self) -> Result<(), StateError> {
        let Some(ref stick_cal) = self.stick_cal else {
            return Err(StateError::NoCalibrationDataAvailable);
        };
        self.h_stick = stick_cal.h_center + stick_cal.h_max_above_center;
        self.v_stick = stick_cal.v_center;
        Ok(())
    }

    pub fn set_left(&mut self) -> Result<(), StateError> {
        let Some(ref stick_cal) = self.stick_cal else {
            return Err(StateError::NoCalibrationDataAvailable);
        };
        self.h_stick = stick_cal.h_center - stick_cal.h_max_below_center;
        self.v_stick = stick_cal.v_center;
        Ok(())
    }

    pub fn calibration(&self) -> Result<&StickCalibration, StateError> {
        let Some(ref stick_cal) = self.stick_cal else {
            return Err(StateError::NoCalibrationDataAvailable);
        };
        Ok(&stick_cal)
    }

    pub fn set_calibration(&mut self, calibration: StickCalibration) -> Result<(), StateError> {
        self.stick_cal = Some(calibration);
        Ok(())
    }

    pub fn to_buf(&self) -> [u8; 3] {
        let byte_1 = (self.h_stick & 0xFF) as u8;
        let byte_2 = ((self.h_stick >> 8) as u8) | (((self.v_stick & 0xF) as u8) << 4);
        let byte_3 = (self.v_stick >> 4) as u8;
        [byte_1, byte_2, byte_3]
    }
}

#[derive(Clone, Debug)]
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

    pub fn with_left_stick_bytes(bytes: &[u8]) -> Option<Self> {
        let bytes_padded = to_u16_bytes(bytes);
        if bytes_padded.len() < 9 {
            return None;
        }
        let h_max_above_center: u16 = (bytes_padded[1] << 8) & 0xF00 | bytes_padded[0];
        let v_max_above_center = (bytes_padded[2] << 4) | (bytes_padded[1] >> 4);
        let h_center = (bytes_padded[4] << 8) & 0xF00 | bytes_padded[3];
        let v_center = (bytes_padded[5] << 4) | (bytes_padded[4] >> 4);
        let h_max_below_center = (bytes_padded[7] << 8) & 0xF00 | bytes_padded[6];
        let v_max_below_center = (bytes_padded[8] << 4) | (bytes_padded[7] >> 4);
        Some(StickCalibration::new(
            h_center,
            v_center,
            h_max_above_center,
            v_max_above_center,
            h_max_below_center,
            v_max_below_center,
        ))
    }

    pub fn with_right_stick_bytes(bytes: &[u8]) -> Option<Self> {
        let bytes_padded = to_u16_bytes(bytes);
        if bytes_padded.len() < 9 {
            return None;
        }
        let h_center: u16 = (bytes_padded[1] << 8) & 0xF00 | bytes_padded[0];
        let v_center = (bytes_padded[2] << 4) | (bytes_padded[1] >> 4);
        let h_max_below_center = (bytes_padded[4] << 8) & 0xF00 | bytes_padded[3];
        let v_max_below_center = (bytes_padded[5] << 4) | (bytes_padded[4] >> 4);
        let h_max_above_center = (bytes_padded[7] << 8) & 0xF00 | bytes_padded[6];
        let v_max_above_center = (bytes_padded[8] << 4) | (bytes_padded[7] >> 4);
        Some(StickCalibration::new(
            h_center,
            v_center,
            h_max_above_center,
            v_max_above_center,
            h_max_below_center,
            v_max_below_center,
        ))
    }
}

fn to_u16_bytes(bytes: &[u8]) -> Vec<u16> {
    bytes
        .iter()
        .cloned()
        .map(|e| u16::from(e))
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::{StickCalibration, StickState};
    use crate::controller::spi_flash::SpiFlash;

    #[test]
    fn stick_calibration() {
        let spi_flash = SpiFlash::new();
        let l_cal =
            StickCalibration::with_left_stick_bytes(&spi_flash.factory_l_stick_calibration())
                .unwrap();
        let stick_state = StickState::with_config(super::StickStateConfig {
            calibration: Some(l_cal),
            ..Default::default()
        });
        println!("{:?}", &stick_state);
    }
}
