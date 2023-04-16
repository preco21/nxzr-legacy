use super::{spi_flash::SpiFlash, ControllerType};
use button::ButtonState;
use stick::{StickCalibration, StickState, StickStateConfig};
use strum::{Display, IntoStaticStr};

pub mod button;
pub mod stick;

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum StateError {
    // Invalid value range has been entered.
    InvalidRange,
    // There is no calibration data available.
    NoCalibrationDataAvailable,
    // The button is not available for the controller of choice.
    ButtonNotAvailable,
    // Unable to create stick calibration instance from the given data.
    UnableToCreateStickCalibration,
    // Returned if any invariant violation happens.
    Invariant,
}

impl std::error::Error for StateError {}

pub type StateResult<T> = Result<T, StateError>;

#[derive(Debug, Default)]
pub struct ControllerStateConfig {
    pub controller: ControllerType,
    pub spi_flash: Option<SpiFlash>,
}

#[derive(Clone, Debug, Default)]
pub struct ControllerState {
    controller: ControllerType,
    button_state: ButtonState,
    l_stick_state: StickState,
    r_stick_state: StickState,
}

impl ControllerState {
    pub fn new() -> Self {
        Self::with_config(Default::default()).unwrap()
    }

    pub fn with_config(config: ControllerStateConfig) -> StateResult<Self> {
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
                })
            }
            None => Ok(Self {
                controller: config.controller,
                button_state: ButtonState::with_controller(config.controller),
                l_stick_state: StickState::new(),
                r_stick_state: StickState::new(),
            }),
        }
    }

    pub fn controller(&self) -> ControllerType {
        self.controller
    }

    pub fn button_state(&self) -> &ButtonState {
        &self.button_state
    }

    pub fn l_stick_state(&self) -> &StickState {
        &self.l_stick_state
    }

    pub fn r_stick_state(&self) -> &StickState {
        &self.r_stick_state
    }
}
