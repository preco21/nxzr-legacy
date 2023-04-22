use controller::{protocol::ProtocolError, report::ReportError, state::StateError};
use event::EventError;
use thiserror::Error;

pub mod controller;
pub mod event;
pub mod protocol_control;

#[derive(Clone, Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Report(#[from] ReportError),
    #[error(transparent)]
    State(#[from] StateError),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error(transparent)]
    Event(#[from] EventError),
}

pub type Result<T> = std::result::Result<T, Error>;
