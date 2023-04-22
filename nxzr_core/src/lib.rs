use strum::{Display, IntoStaticStr};

pub mod controller;
pub mod event;
pub mod protocol_control;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Clone, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum ErrorKind {
    // Report errors
    Report(controller::report::ReportError),
    // State errors
    State(controller::state::StateError),
    // Protocol errors
    Protocol(controller::protocol::ProtocolErrorKind),
    // Internal errors
    Internal(InternalErrorKind),
}

#[derive(Clone, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum InternalErrorKind {
    Io(std::io::ErrorKind),
    Event(event::EventError),
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
            kind: ErrorKind::Report(err),
            message: err.to_string(),
        }
    }
}

impl From<controller::state::StateError> for Error {
    fn from(err: controller::state::StateError) -> Self {
        Self {
            kind: ErrorKind::State(err),
            message: err.to_string(),
        }
    }
}

impl From<controller::protocol::ProtocolError> for Error {
    fn from(err: controller::protocol::ProtocolError) -> Self {
        Self {
            kind: ErrorKind::Protocol(err.kind),
            message: err.message,
        }
    }
}

impl From<event::EventError> for Error {
    fn from(err: event::EventError) -> Self {
        Self {
            kind: ErrorKind::Internal(InternalErrorKind::Event(err)),
            message: err.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
