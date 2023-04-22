use crate::sock::{self, l2cap::LazySeqPacketListener};
use bluer::l2cap::{self, SocketAddr};
use std::os::fd::AsRawFd;
use strum::{Display, IntoStaticStr};
use thiserror::Error;

const DEFAULT_CTL_PSM: u32 = 17;
const DEFAULT_ITR_PSM: u32 = 19;

#[derive(Clone, Error, Debug)]
pub enum SessionError {
    Unknown,
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
        ctl_sock.set_reuse_addr()?;
        let itr_sock = LazySeqPacketListener::new()?;
        ctl_sock.set_reuse_addr()?;

        Ok(Self { ctl_sock, itr_sock })
        // let socket = l2cap::Socket::<SeqPacket>::new_seq_packet()?;
        // socket.bind(sa)?;
        // socket.listen(1);
        // let ctl_sock = l2cap::SeqPacketListener::bind();
        // let itr_sock =
        // Self { ctl_sock, itr_sock }
        // FIXME: https://www.ibm.com/docs/en/ztpf/1.1.0.15?topic=apis-setsockopt-set-options-associated-socket
        // SO_SNDBUF
    }

    pub fn bind() -> Result<()> {}

    pub fn listen() -> Result<()> {}
}
