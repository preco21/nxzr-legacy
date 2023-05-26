use crate::sock::{
    self, sock_priv,
    sys::{hci_filter, sockaddr_hci, BTPROTO_HCI, HCI_FILTER, SOL_HCI},
    OwnedFd,
};
use libc::{
    AF_BLUETOOTH, EAGAIN, EINPROGRESS, MSG_PEEK, SHUT_RD, SHUT_RDWR, SHUT_WR, SOCK_CLOEXEC,
    SOCK_RAW, SOL_SOCKET, SO_ERROR, SO_RCVBUF, TIOCINQ, TIOCOUTQ,
};
use std::{
    fmt,
    io::{Error, ErrorKind, Result},
    net::Shutdown,
    os::{
        raw::c_int,
        unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
    },
    task::{Context, Poll},
};
use tokio::io::{unix::AsyncFd, ReadBuf};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SocketAddr {
    pub dev_id: u16,
}

impl SocketAddr {
    pub const fn new(dev_id: u16) -> Self {
        Self { dev_id }
    }

    pub const fn any_raw() -> Self {
        Self { dev_id: 0 }
    }
}

impl sock::SysSockAddr for SocketAddr {
    type SysSockAddr = sockaddr_hci;

    fn into_sys_sock_addr(self) -> Self::SysSockAddr {
        sockaddr_hci {
            hci_family: AF_BLUETOOTH as _,
            hci_dev: self.dev_id.into(),
            hci_channel: 0 as _,
        }
    }

    fn try_from_sys_sock_addr(saddr: Self::SysSockAddr) -> Result<Self> {
        if saddr.hci_family != AF_BLUETOOTH as _ {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "sockaddr_hci::hci_family is not AF_BLUETOOTH",
            ));
        }
        Ok(Self {
            dev_id: saddr.hci_dev,
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Filter {
    pub type_mask: u32,
    pub event_mask: [u32; 2],
    pub opcode: u16,
}

impl From<Filter> for hci_filter {
    fn from(f: Filter) -> Self {
        hci_filter {
            type_mask: f.type_mask,
            event_mask: f.event_mask,
            opcode: f.opcode,
        }
    }
}

pub struct Socket {
    fd: AsyncFd<OwnedFd>,
}

impl fmt::Debug for Socket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Socket")
            .field("fd", &self.fd.as_raw_fd())
            .finish()
    }
}

impl Socket {
    pub fn new() -> Result<Socket> {
        Ok(Self {
            fd: AsyncFd::new(sock::socket(AF_BLUETOOTH, SOCK_RAW, BTPROTO_HCI)?)?,
        })
    }

    pub fn into_datagram(self) -> Datagram {
        Datagram { socket: self }
    }

    pub fn bind(&self, sa: SocketAddr) -> Result<()> {
        sock::bind(self.fd.get_ref(), sa)
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        sock::getsockname(self.fd.get_ref())
    }

    fn peer_addr_priv(&self) -> Result<SocketAddr> {
        sock::getpeername(self.fd.get_ref())
    }

    pub fn set_filter(&self, filter: Filter) -> Result<()> {
        let f: hci_filter = filter.into();
        sock::setsockopt(self.fd.get_ref(), SOL_HCI, HCI_FILTER, &f)
    }

    pub fn recv_buffer(&self) -> Result<i32> {
        sock::getsockopt(self.fd.get_ref(), SOL_SOCKET, SO_RCVBUF)
    }

    pub fn set_recv_buffer(&self, recv_buffer: i32) -> Result<()> {
        sock::setsockopt(self.fd.get_ref(), SOL_SOCKET, SO_RCVBUF, &recv_buffer)
    }

    pub fn input_buffer(&self) -> Result<u32> {
        let value: c_int = sock::ioctl_read(self.fd.get_ref(), TIOCINQ)?;
        Ok(value as _)
    }

    pub fn output_buffer(&self) -> Result<u32> {
        let value: c_int = sock::ioctl_read(self.fd.get_ref(), TIOCOUTQ)?;
        Ok(value as _)
    }

    pub unsafe fn from_raw_fd(fd: RawFd) -> Result<Self> {
        Ok(Self {
            fd: AsyncFd::new(OwnedFd::new(fd))?,
        })
    }

    fn from_owned_fd(fd: OwnedFd) -> Result<Self> {
        Ok(Self {
            fd: AsyncFd::new(fd)?,
        })
    }

    sock_priv!();
}

impl AsRawFd for Socket {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl IntoRawFd for Socket {
    fn into_raw_fd(self) -> RawFd {
        self.fd.into_inner().into_raw_fd()
    }
}

impl FromRawFd for Socket {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self::from_raw_fd(fd).expect("from_raw_fd failed")
    }
}

#[derive(Debug)]
pub struct Datagram {
    socket: Socket,
}

impl Datagram {
    pub async fn bind(sa: SocketAddr) -> Result<Self> {
        let socket = Socket::new()?;
        socket.bind(sa)?;
        Ok(socket.into_datagram())
    }

    pub async fn connect(&self, sa: SocketAddr) -> Result<()> {
        self.socket.connect_priv(sa).await
    }

    pub fn peer_addr(&self) -> Result<SocketAddr> {
        self.socket.peer_addr_priv()
    }

    pub async fn send(&self, buf: &[u8]) -> Result<usize> {
        self.socket.send_priv(buf).await
    }

    pub fn poll_send(&self, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        self.socket.poll_send_priv(cx, buf)
    }

    pub async fn send_to(&self, buf: &[u8], target: SocketAddr) -> Result<usize> {
        self.socket.send_to_priv(buf, target).await
    }

    pub fn poll_send_to(
        &self,
        cx: &mut Context,
        buf: &[u8],
        target: SocketAddr,
    ) -> Poll<Result<usize>> {
        self.socket.poll_send_to_priv(cx, buf, target)
    }

    pub async fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        self.socket.recv_priv(buf).await
    }

    pub fn poll_recv(&self, cx: &mut Context, buf: &mut ReadBuf) -> Poll<Result<()>> {
        self.socket.poll_recv_priv(cx, buf)
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        self.socket.recv_from_priv(buf).await
    }

    pub fn poll_recv_from(&self, cx: &mut Context, buf: &mut ReadBuf) -> Poll<Result<SocketAddr>> {
        self.socket.poll_recv_from_priv(cx, buf)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.socket.shutdown_priv(how)
    }

    pub fn poll_shutdown(&self, cx: &mut Context, how: Shutdown) -> Poll<Result<()>> {
        self.socket.poll_shutdown_priv(cx, how)
    }

    pub unsafe fn from_raw_fd(fd: RawFd) -> Result<Self> {
        Ok(Self {
            socket: Socket::from_raw_fd(fd)?,
        })
    }
}

impl AsRef<Socket> for Datagram {
    fn as_ref(&self) -> &Socket {
        &self.socket
    }
}

impl AsRawFd for Datagram {
    fn as_raw_fd(&self) -> RawFd {
        self.socket.as_raw_fd()
    }
}

impl FromRawFd for Datagram {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self::from_raw_fd(fd).expect("from_raw_fd failed")
    }
}
