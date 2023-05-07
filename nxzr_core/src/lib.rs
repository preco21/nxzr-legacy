use controller::{protocol::ControllerProtocolError, report::ReportError, state::StateError};
use event::EventError;
use protocol::ProtocolError;
use thiserror::Error;

pub mod controller;
pub mod event;
pub mod protocol;

#[derive(Clone, Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Report(#[from] ReportError),
    #[error(transparent)]
    State(#[from] StateError),
    #[error(transparent)]
    ControllerProtocol(#[from] ControllerProtocolError),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error(transparent)]
    Event(#[from] EventError),
}

pub type Result<T> = std::result::Result<T, Error>;
