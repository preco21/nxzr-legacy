pub mod controller;

use strum::{Display, IntoStaticStr};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum ErrorKind {
    Internal(InternalErrorKind),
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum InternalErrorKind {
    Io(std::io::ErrorKind),
    ControllerReportError(controller::report::ReportError),
    ControllerStateError(controller::state::StateError),
    EventSubFailed,
    ProtocolError,
    InputReportCreationFailed,
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

impl From<controller::report::ReportError> for Error {
    fn from(err: controller::report::ReportError) -> Self {
        Self {
            kind: ErrorKind::Internal(InternalErrorKind::ControllerReportError(err)),
            message: String::new(),
        }
    }
}

impl From<controller::state::StateError> for Error {
    fn from(err: controller::state::StateError) -> Self {
        Self {
            kind: ErrorKind::Internal(InternalErrorKind::ControllerStateError(err)),
            message: String::new(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
