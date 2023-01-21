use strum::{Display, IntoStaticStr};

pub mod button;
pub mod stick;

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum StateError {
    // Invalid value range has been entered.
    InvalidRange,
    // There is no calibration data available.
    NoCalibrationDataAvailable,
    // Returned if any invariant violation happens.
    Invariant,
}

impl std::error::Error for StateError {}

pub type StateResult<T> = Result<T, StateError>;

#[derive(Clone, Debug)]
pub struct ControllerState;
