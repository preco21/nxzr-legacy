use event::EventError;
use thiserror::Error;
use transport::TransportError;

pub mod event;
pub mod semaphore;
pub mod session;
pub mod sock;
pub mod transport;

#[derive(Clone, Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Event(#[from] EventError),
}

pub type Result<T> = std::result::Result<T, Error>;
