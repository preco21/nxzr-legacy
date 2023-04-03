use strum::{Display, IntoStaticStr};
use tokio::sync::AcquireError;
use transport::TransportErrorKind;

pub mod semaphore;
pub mod session;
pub mod sock;
pub mod transport;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum ErrorKind {
    Transport(TransportErrorKind),
    Internal(InternalErrorKind),
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum InternalErrorKind {
    Io(std::io::ErrorKind),
    AcquireError,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            message: String::new(),
        }
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

impl From<AcquireError> for Error {
    fn from(_: AcquireError) -> Self {
        Self {
            kind: ErrorKind::Internal(InternalErrorKind::AcquireError),
            message: "Semaphore closed.".to_owned(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
