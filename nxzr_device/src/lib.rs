use device::DeviceError;
use session::SessionError;
use thiserror::Error;
use transport::TransportError;

pub mod device;
pub mod semaphore;
pub mod session;
pub mod sock;
pub mod transport;

#[derive(Clone, Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error(transparent)]
    Device(#[from] DeviceError),
}

pub type Result<T> = std::result::Result<T, Error>;
