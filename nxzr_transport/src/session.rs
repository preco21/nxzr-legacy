use crate::{
    sock::{self, l2cap::LazySeqPacketListener},
    Result,
};
use bluer::l2cap::{self, SocketAddr};
use std::os::fd::AsRawFd;
use strum::{Display, IntoStaticStr};

const DEFAULT_CTL_PSM: u32 = 17;
const DEFAULT_ITR_PSM: u32 = 19;

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum SessionErrorKind {
    Unknown,
}

#[derive(Clone, Debug, Default)]
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
    pub fn new(config: SessionConfig) -> Result<Self> {
        let control_psm = match config.control_psm {
            Some(psm) => psm,
            None => DEFAULT_CTL_PSM,
        };
        let interrupt_psm = match config.interrupt_psm {
            Some(psm) => psm,
            None => DEFAULT_ITR_PSM,
        };
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
    }

    pub fn bind() -> Result<()> {}
}
