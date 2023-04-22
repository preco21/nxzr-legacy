use crate::sock::{self, l2cap::LazySeqPacketListener};
use bluer::l2cap::{self, SocketAddr};
use thiserror::Error;

const DEFAULT_CTL_PSM: u32 = 17;
const DEFAULT_ITR_PSM: u32 = 19;

#[derive(Clone, Error, Debug)]
pub enum SessionError {
    #[error("unknown")]
    Unknown,
    #[error("internal error: {0}")]
    Internal(SessionInternalError),
}

#[derive(Clone, Error, Debug)]
pub enum SessionInternalError {
    #[error("io: {0}")]
    Io(std::io::ErrorKind),
}

impl From<std::io::Error> for SessionError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(SessionInternalError::Io(err.kind()))
    }
}

#[derive(Debug, Default)]
pub struct SessionConfig {
    control_psm: Option<u32>,
    interrupt_psm: Option<u32>,
}

#[derive(Debug)]
pub struct Session {
    ctl_sock: LazySeqPacketListener,
    itr_sock: LazySeqPacketListener,
}

impl Session {
    pub fn new(config: SessionConfig) -> Result<Self, SessionError> {
        let control_psm = config.control_psm.unwrap_or(DEFAULT_CTL_PSM);
        let interrupt_psm = config.interrupt_psm.unwrap_or(DEFAULT_ITR_PSM);
        let ctl_sock = LazySeqPacketListener::new()?;
        ctl_sock.enable_reuse_addr()?;
        let itr_sock = LazySeqPacketListener::new()?;
        itr_sock.enable_reuse_addr()?;

        Ok(Self { ctl_sock, itr_sock })
    }

    pub fn bind() -> Result<(), SessionError> {}

    pub fn listen(&self) -> Result<(), SessionError> {
        Ok(())
    }
}
