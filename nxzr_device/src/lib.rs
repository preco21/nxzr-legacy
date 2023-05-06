use device::DeviceError;
use helper::HelperError;
use session::SessionError;
use syscheck::SysCheckError;
use transport::TransportError;

pub mod device;
pub mod helper;
pub mod semaphore;
pub mod session;
pub mod sock;
pub mod syscheck;
pub mod transport;

#[derive(Clone, thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error(transparent)]
    Device(#[from] DeviceError),
    #[error(transparent)]
    Helper(#[from] HelperError),
    #[error(transparent)]
    SysCheck(#[from] SysCheckError),
}

pub type Result<T> = std::result::Result<T, Error>;
