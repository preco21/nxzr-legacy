use crate::sock;
use bluer::l2cap::{SeqPacket, Socket, SocketAddr};
use libc::{SOL_SOCKET, SO_REUSEADDR};
use std::{
    io::Result,
    os::fd::{AsRawFd, FromRawFd, RawFd},
    task::{Context, Poll},
};

use super::OwnedFd;

#[derive(Debug)]
pub struct LazySeqPacketListener {
    listener: bluer::l2cap::SeqPacketListener,
}

impl LazySeqPacketListener {
    pub fn new() -> Result<Self> {
        let socket = Socket::<SeqPacket>::new_seq_packet()?;
        let listener = bluer::l2cap::SeqPacketListener { socket };
        Ok(Self { listener })
    }

    fn as_owned_fd(&self) -> OwnedFd {
        unsafe { OwnedFd::new(self.listener.as_ref().as_raw_fd()) }
    }

    pub fn set_reuse_addr(&self) -> Result<()> {
        let fd = self.as_owned_fd();
        sock::setsockopt(&fd, SOL_SOCKET, SO_REUSEADDR, &1)
    }

    pub async fn bind(&self, sa: SocketAddr) -> Result<()> {
        self.listener.as_ref().bind(sa)
    }

    pub async fn accept(&self) -> Result<(SeqPacket, SocketAddr)> {
        self.listener.accept().await
    }

    pub fn poll_accept(&self, cx: &mut Context) -> Poll<Result<(SeqPacket, SocketAddr)>> {
        self.listener.poll_accept(cx)
    }

    pub unsafe fn from_raw_fd(fd: RawFd) -> Result<Self> {
        let listener = bluer::l2cap::SeqPacketListener {
            socket: Socket::from_raw_fd(fd)?,
        };
        Ok(Self { listener })
    }
}

impl AsRef<Socket<SeqPacket>> for LazySeqPacketListener {
    fn as_ref(&self) -> &Socket<SeqPacket> {
        &self.listener.as_ref()
    }
}

impl AsRawFd for LazySeqPacketListener {
    fn as_raw_fd(&self) -> RawFd {
        self.listener.as_raw_fd()
    }
}

impl FromRawFd for LazySeqPacketListener {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self::from_raw_fd(fd).expect("from_raw_fd failed")
    }
}
