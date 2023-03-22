use super::sys::{hci_filter, HCI_FILTER, SOL_HCI};
use crate::sock::{
    self, sock_priv,
    sys::{sockaddr_hci, BTPROTO_HCI},
    OwnedFd,
};
use futures::ready;
use libc::{
    AF_BLUETOOTH, EAGAIN, EINPROGRESS, MSG_PEEK, SHUT_RD, SHUT_RDWR, SHUT_WR, SOCK_RAW, SOL_SOCKET,
    SO_ERROR, SO_RCVBUF, TIOCINQ, TIOCOUTQ,
};
use std::{
    fmt,
    io::{Error, ErrorKind, Result},
    mem::ManuallyDrop,
    net::Shutdown,
    os::{
        raw::c_int,
        unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
    },
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::io::{unix::AsyncFd, AsyncRead, AsyncWrite, ReadBuf};

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

    pub fn into_listener(self) -> Datagram {
        Datagram { socket: self }
    }

    pub async fn connect(self, sa: SocketAddr) -> Result<Stream> {
        self.connect_priv(sa).await?;
        Stream::from_socket(self)
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

    pub fn set_filter(&self, filter: hci_filter) -> Result<()> {
        sock::setsockopt(self.fd.get_ref(), SOL_HCI, HCI_FILTER, &filter)
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
        Ok(socket.into_listener())
    }

    pub async fn connect(&self, sa: SocketAddr) -> Result<()> {
        self.socket.connect_priv(sa).await
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

#[derive(Debug)]
pub struct Stream {
    socket: Socket,
}

impl Stream {
    fn from_socket(socket: Socket) -> Result<Self> {
        Ok(Self { socket })
    }

    pub async fn connect(addr: SocketAddr) -> Result<Self> {
        let socket = Socket::new()?;
        socket.bind(SocketAddr::any_raw())?;
        socket.connect(addr).await
    }

    pub fn peer_addr(&self) -> Result<SocketAddr> {
        self.socket.peer_addr_priv()
    }

    pub async fn peek(&self, buf: &mut [u8]) -> Result<usize> {
        self.socket.peek_priv(buf).await
    }

    pub fn poll_peek(&self, cx: &mut Context, buf: &mut ReadBuf) -> Poll<Result<usize>> {
        self.socket.poll_peek_priv(cx, buf)
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn split<'a>(&'a mut self) -> (stream::ReadHalf<'a>, stream::WriteHalf<'a>) {
        (stream::ReadHalf(self), stream::WriteHalf(self))
    }

    pub fn into_split(self) -> (stream::OwnedReadHalf, stream::OwnedWriteHalf) {
        let stream = Arc::new(self);
        let r = stream::OwnedReadHalf {
            stream: ManuallyDrop::new(stream.clone()),
            shutdown_on_drop: true,
            drop: true,
        };
        let w = stream::OwnedWriteHalf {
            stream,
            shutdown_on_drop: true,
        };
        (r, w)
    }

    fn poll_write_priv(&self, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        self.socket.poll_send_priv(cx, buf)
    }

    pub unsafe fn from_raw_fd(fd: RawFd) -> Result<Self> {
        Self::from_socket(Socket::from_raw_fd(fd)?)
    }
}

impl AsRef<Socket> for Stream {
    fn as_ref(&self) -> &Socket {
        &self.socket
    }
}

impl AsRawFd for Stream {
    fn as_raw_fd(&self) -> RawFd {
        self.socket.as_raw_fd()
    }
}

impl FromRawFd for Stream {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self::from_raw_fd(fd).expect("from_raw_fd failed")
    }
}

impl AsyncRead for Stream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut ReadBuf) -> Poll<Result<()>> {
        self.socket.poll_recv_priv(cx, buf)
    }
}

impl AsyncWrite for Stream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        self.poll_write_priv(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
        self.socket.poll_flush_priv(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
        self.socket.poll_shutdown_priv(cx, Shutdown::Write)
    }
}

#[allow(clippy::duplicate_mod)]
#[path = "stream_util.rs"]
pub mod stream;
