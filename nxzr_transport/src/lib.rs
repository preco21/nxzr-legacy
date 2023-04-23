use thiserror::Error;
use transport::TransportError;

pub mod semaphore;
pub mod session;
pub mod sock;
pub mod transport;

#[derive(Clone, Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Transport(#[from] TransportError),
}

pub type Result<T> = std::result::Result<T, Error>;
