use controller::{protocol::ProtocolError, report::ReportError, state::StateError};
use event::EventError;
use thiserror::Error;

pub mod controller;
pub mod event;
pub mod protocol_control;

#[derive(Clone, Error, Debug)]
pub enum Error {
    #[error("report error: {0}")]
    Report(#[from] ReportError),
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("event error: {0}")]
    Event(#[from] EventError),
}

pub type Result<T> = std::result::Result<T, Error>;
