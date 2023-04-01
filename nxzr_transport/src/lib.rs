use strum::Display;
use tokio::sync::AcquireError;

pub mod semaphore;
pub mod sock;
pub mod transport;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Clone, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ErrorKind {
    Internal(InternalErrorKind),
}

#[derive(Clone, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
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
