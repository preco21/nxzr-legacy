use std::time::Duration;
use strum::{Display, IntoStaticStr};

pub mod controller;
pub mod protocol_control;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum ErrorKind {
    // Report errors
    Report(ReportErrorKind),
    // State errors
    State(StateErrorKind),
    // Protocol errors
    Protocol(ProtocolErrorKind),
    // Internal errors
    Internal(InternalErrorKind),
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum ReportErrorKind {
    // Invalid value range has been entered.
    InvalidRange,
    // Indicates that given data has not enough length. Usually used in constructors.
    TooShort,
    // Indicates that given data is malformed thus cannot be processed. Usually used in constructors.
    Malformed,
    // Returned when accessing/processing data that do not support given bounds.
    OutOfBounds,
    // There's no data for a value within a range. Usually used instead of
    // `OutOfBounds` for a return value of getter methods where `OutOfBounds` is
    // not appropriate. Since it's more descriptive to indicate that you are
    // accessing no-existent data than just saying data out-of-bounds.
    NoDataAvailable,
    // Returned if invariant violation happens.
    Invariant,
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum StateErrorKind {
    // Invalid value range has been entered.
    InvalidRange,
    // There is no calibration data available.
    NoCalibrationDataAvailable,
    // The button is not available for the controller of choice.
    ButtonNotAvailable,
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum ProtocolErrorKind {
    // Failed to parse output report.
    OutputReportParsingFailed,
    // Failed to create input report.
    InputReportCreationFailed,
    // Write operation in protocol is too slow.
    WriteTooSlow(Duration),
    // Returned if invariant violation happens.
    Invariant,
    // Feature is not implemented.
    NotImplemented,
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum InternalErrorKind {
    Io(std::io::ErrorKind),
    EventSubscriptionFailed,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            message: String::new(),
        }
    }

    pub(crate) fn with_message(kind: ErrorKind, message: String) -> Self {
        Self { kind, message }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.message.is_empty() {
            write!(f, "{}", &self.kind)
        } else {
            write!(f, "{}: {}", &self.kind, &self.message)
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self {
            kind: ErrorKind::Internal(InternalErrorKind::Io(err.kind())),
            message: err.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
