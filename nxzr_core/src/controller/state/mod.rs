use super::{spi_flash::SpiFlash, ControllerType};
use button::ButtonState;
use imu::ImuState;
use stick::{StickCalibration, StickState, StickStateConfig};

pub mod button;
pub mod imu;
pub mod stick;

#[derive(Clone, Debug, thiserror::Error)]
pub enum StateError {
    // Invalid value range has been entered.
    #[error("invalid value range supplied")]
    InvalidRange,
    // Invalid scale has been entered.
    #[error("invalid scale range supplied")]
    InvalidScale,
    // There is no calibration data available.
    #[error("no calibration data is supplied, unable to call the method")]
    NoCalibrationDataAvailable,
    // The button is not available for the controller of choice.
    #[error("given button is not available")]
    ButtonNotAvailable,
}

#[derive(Debug, Default)]
pub struct ControllerStateConfig {
    pub controller: ControllerType,
    pub spi_flash: Option<SpiFlash>,
}

#[derive(Clone, Debug)]
pub struct ControllerState {
    controller: ControllerType,
    button_state: ButtonState,
    l_stick_state: StickState,
    r_stick_state: StickState,
    imu_state: ImuState,
}

impl ControllerState {
    pub fn new() -> Self {
        Self::with_config(Default::default()).unwrap()
    }

    pub fn with_config(config: ControllerStateConfig) -> Result<Self, StateError> {
        match config.spi_flash {
            Some(spi_flash) => {
                let Some(l_calibration) = StickCalibration::with_left_stick_bytes(
                    match spi_flash.user_l_stick_calibration() {
                        Some(calibration_data) => calibration_data,
                        None => spi_flash.factory_l_stick_calibration(),
                    },
                ) else {
                    return Err(StateError::NoCalibrationDataAvailable);
                };
                let mut l_stick_state = StickState::with_config(StickStateConfig {
                    calibration: Some(l_calibration),
                    ..Default::default()
                })?;
                l_stick_state.reset_to_center()?;
                let Some(r_calibration) = StickCalibration::with_right_stick_bytes(
                    match spi_flash.user_r_stick_calibration() {
                        Some(calibration_data) => calibration_data,
                        None => spi_flash.factory_r_stick_calibration(),
                    },
                ) else {
                    return Err(StateError::NoCalibrationDataAvailable);
                };
                let mut r_stick_state = StickState::with_config(StickStateConfig {
                    calibration: Some(r_calibration),
                    ..Default::default()
                })?;
                r_stick_state.reset_to_center()?;
                Ok(Self {
                    controller: config.controller,
                    button_state: ButtonState::with_controller(config.controller),
                    l_stick_state,
                    r_stick_state,
                    imu_state: ImuState::new(),
                })
            }
            None => Ok(Self {
                controller: config.controller,
                button_state: ButtonState::with_controller(config.controller),
                l_stick_state: StickState::new(),
                r_stick_state: StickState::new(),
                imu_state: ImuState::new(),
            }),
        }
    }

    pub fn controller(&self) -> ControllerType {
        self.controller
    }

    pub fn button_state(&self) -> &ButtonState {
        &self.button_state
    }

    pub fn button_state_mut(&mut self) -> &mut ButtonState {
        &mut self.button_state
    }

    pub fn l_stick_state(&self) -> &StickState {
        &self.l_stick_state
    }

    pub fn l_stick_state_mut(&mut self) -> &mut StickState {
        &mut self.l_stick_state
    }

    pub fn r_stick_state(&self) -> &StickState {
        &self.r_stick_state
    }

    pub fn r_stick_state_mut(&mut self) -> &mut StickState {
        &mut self.r_stick_state
    }

    pub fn imu_state(&self) -> &ImuState {
        &self.imu_state
    }

    pub fn imu_state_mut(&mut self) -> &mut ImuState {
        &mut self.imu_state
    }
}
