// Excerpt from `bluer` project: https://github.com/bluez/bluer/blob/8ffd4aeef3f8ab0d65dca66eb5a03f223351f586/bluer/src/sock.rs
//! System socket base.
use crate::Address;
use libc::{c_int, sockaddr, socklen_t, Ioctl, F_GETFL, F_SETFL, SOCK_CLOEXEC, SOCK_NONBLOCK};
use num_derive::FromPrimitive;
use std::{
    fmt::Debug,
    io::{Error, ErrorKind, Result},
    mem::{size_of, MaybeUninit},
    os::unix::io::{AsRawFd, IntoRawFd, RawFd},
};
use strum::{Display, EnumString};
use tokio::io::ReadBuf;

pub mod hci;
pub mod l2cap;
pub mod sys;

/// File descriptor that is closed on drop.
#[derive(Debug)]
pub struct OwnedFd {
    fd: RawFd,
    close_on_drop: bool,
}

impl OwnedFd {
    /// Create new OwnedFd taking ownership of file descriptor.
    pub unsafe fn new(fd: RawFd) -> Self {
        Self {
            fd,
            close_on_drop: true,
        }
    }
}

impl AsRawFd for OwnedFd {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl IntoRawFd for OwnedFd {
    fn into_raw_fd(mut self) -> RawFd {
        self.close_on_drop = false;
        self.fd
    }
}

impl Drop for OwnedFd {
    fn drop(&mut self) {
        if self.close_on_drop {
            unsafe { libc::close(self.fd) };
        }
    }
}

impl From<sys::bdaddr_t> for Address {
    fn from(mut addr: sys::bdaddr_t) -> Self {
        addr.b.reverse();
        Self(addr.b)
    }
}

impl From<Address> for sys::bdaddr_t {
    fn from(mut addr: Address) -> Self {
        addr.0.reverse();
        sys::bdaddr_t { b: addr.0 }
    }
}

/// Bluetooth device address type.
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Display, EnumString, FromPrimitive,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum AddressType {
    /// Classic Bluetooth (BR/EDR) address.
    #[strum(serialize = "br/edr")]
    BrEdr = sys::BDADDR_BREDR,
    /// Bluetooth Low Energy (LE) public address.
    #[strum(serialize = "public")]
    LePublic = sys::BDADDR_LE_PUBLIC,
    /// Bluetooth Low Energy (LE) random address.
    #[strum(serialize = "random")]
    LeRandom = sys::BDADDR_LE_RANDOM,
}

impl Default for AddressType {
    fn default() -> Self {
        Self::LePublic
    }
}

/// Address that is convertible to and from a system socket address.
pub trait SysSockAddr: Sized {
    /// System socket address type.
    type SysSockAddr: Sized + 'static;

    /// Convert to system socket address.
    fn into_sys_sock_addr(self) -> Self::SysSockAddr;

    /// Convert from system socket address.
    fn try_from_sys_sock_addr(addr: Self::SysSockAddr) -> Result<Self>;
}

/// Creates a socket of the specified type and returns its file descriptor.
///
/// Note: From the original code, the socket was set to non-blocking mode.
/// However, for some reasons, some devices are not accept the asynchronous
/// client. So, we're letting the caller to set the non-blocking mode on their
/// own.
pub fn socket(sa: c_int, ty: c_int, proto: c_int) -> Result<OwnedFd> {
    let fd = match unsafe { libc::socket(sa, ty | SOCK_CLOEXEC, proto) } {
        -1 => return Err(Error::last_os_error()),
        fd => unsafe { OwnedFd::new(fd) },
    };
    Ok(fd)
}

/// Binds socket to specified address.
pub fn bind<SA>(socket: &OwnedFd, sa: SA) -> Result<()>
where
    SA: SysSockAddr,
{
    let addr: SA::SysSockAddr = sa.into_sys_sock_addr();
    if unsafe {
        libc::bind(
            socket.as_raw_fd(),
            &addr as *const _ as *const sockaddr,
            size_of::<SA::SysSockAddr>() as socklen_t,
        )
    } == 0
    {
        Ok(())
    } else {
        Err(Error::last_os_error())
    }
}

/// Gets the address the socket is bound to.
pub fn getsockname<SA>(socket: &OwnedFd) -> Result<SA>
where
    SA: SysSockAddr,
{
    let mut saddr: MaybeUninit<SA::SysSockAddr> = MaybeUninit::uninit();
    let mut length = size_of::<SA::SysSockAddr>() as socklen_t;

    if unsafe {
        libc::getsockname(
            socket.as_raw_fd(),
            saddr.as_mut_ptr() as *mut _,
            &mut length,
        )
    } == -1
    {
        return Err(Error::last_os_error());
    };

    if length != size_of::<SA::SysSockAddr>() as socklen_t {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "invalid sockaddr length from getsockname",
        ));
    }
    let saddr = unsafe { saddr.assume_init() };
    SA::try_from_sys_sock_addr(saddr)
}

/// Gets the address the socket is connected to.
pub fn getpeername<SA>(socket: &OwnedFd) -> Result<SA>
where
    SA: SysSockAddr,
{
    let mut saddr: MaybeUninit<SA::SysSockAddr> = MaybeUninit::uninit();
    let mut length = size_of::<SA::SysSockAddr>() as socklen_t;

    if unsafe {
        libc::getpeername(
            socket.as_raw_fd(),
            saddr.as_mut_ptr() as *mut _,
            &mut length,
        )
    } == -1
    {
        return Err(Error::last_os_error());
    };

    if length != size_of::<SA::SysSockAddr>() as socklen_t {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "invalid sockaddr length from getpeername",
        ));
    }
    let saddr = unsafe { saddr.assume_init() };
    SA::try_from_sys_sock_addr(saddr)
}

/// Puts socket in listen mode.
pub fn listen(socket: &OwnedFd, backlog: i32) -> Result<()> {
    if unsafe { libc::listen(socket.as_raw_fd(), backlog) } == 0 {
        Ok(())
    } else {
        Err(Error::last_os_error())
    }
}

/// Accept a connection on the provided socket.
///
/// The accepted socket is set into non-blocking mode.
pub fn accept<SA>(socket: &OwnedFd) -> Result<(OwnedFd, SA)>
where
    SA: SysSockAddr,
{
    let mut saddr: MaybeUninit<SA::SysSockAddr> = MaybeUninit::uninit();
    let mut length = size_of::<SA::SysSockAddr>() as socklen_t;

    let fd = match unsafe {
        libc::accept4(
            socket.as_raw_fd(),
            saddr.as_mut_ptr() as *mut _,
            &mut length,
            SOCK_CLOEXEC | SOCK_NONBLOCK,
        )
    } {
        -1 => return Err(Error::last_os_error()),
        fd => unsafe { OwnedFd::new(fd) },
    };

    if length != size_of::<SA::SysSockAddr>() as socklen_t {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "invalid sockaddr length",
        ));
    }
    let saddr = unsafe { saddr.assume_init() };
    let sa = SA::try_from_sys_sock_addr(saddr)?;

    Ok((fd, sa))
}

/// Initiate a connection on a socket to the specified address.
pub fn connect<SA>(socket: &OwnedFd, sa: SA) -> Result<()>
where
    SA: SysSockAddr,
{
    let addr: SA::SysSockAddr = sa.into_sys_sock_addr();
    if unsafe {
        libc::connect(
            socket.as_raw_fd(),
            &addr as *const _ as *const sockaddr,
            size_of::<SA::SysSockAddr>() as socklen_t,
        )
    } == 0
    {
        Ok(())
    } else {
        Err(Error::last_os_error())
    }
}

/// Sends from buffer into socket.
pub fn send(socket: &OwnedFd, buf: &[u8], flags: c_int) -> Result<usize> {
    match unsafe {
        libc::send(
            socket.as_raw_fd(),
            buf.as_ptr() as *const _,
            buf.len(),
            flags,
        )
    } {
        -1 => Err(Error::last_os_error()),
        n => Ok(n as _),
    }
}

/// Sends from buffer into socket using destination address.
pub fn sendto<SA>(socket: &OwnedFd, buf: &[u8], flags: c_int, sa: SA) -> Result<usize>
where
    SA: SysSockAddr,
{
    let addr: SA::SysSockAddr = sa.into_sys_sock_addr();
    match unsafe {
        libc::sendto(
            socket.as_raw_fd(),
            buf.as_ptr() as *const _,
            buf.len(),
            flags,
            &addr as *const _ as *const sockaddr,
            size_of::<SA::SysSockAddr>() as socklen_t,
        )
    } {
        -1 => Err(Error::last_os_error()),
        n => Ok(n as _),
    }
}

/// Receive from socket into buffer.
pub fn recv(socket: &OwnedFd, buf: &mut ReadBuf, flags: c_int) -> Result<usize> {
    let unfilled = unsafe { buf.unfilled_mut() };
    match unsafe {
        libc::recv(
            socket.as_raw_fd(),
            unfilled.as_mut_ptr() as *mut _,
            unfilled.len(),
            flags,
        )
    } {
        -1 => Err(Error::last_os_error()),
        n => {
            let n = n as usize;
            unsafe {
                buf.assume_init(n);
            }
            buf.advance(n);
            Ok(n)
        }
    }
}

/// Receive from socket into buffer with source address.
pub fn recvfrom<SA>(socket: &OwnedFd, buf: &mut ReadBuf, flags: c_int) -> Result<(usize, SA)>
where
    SA: SysSockAddr,
{
    let unfilled = unsafe { buf.unfilled_mut() };
    let mut saddr: MaybeUninit<SA::SysSockAddr> = MaybeUninit::uninit();
    let mut length = size_of::<SA::SysSockAddr>() as socklen_t;
    match unsafe {
        libc::recvfrom(
            socket.as_raw_fd(),
            unfilled.as_mut_ptr() as *mut _,
            unfilled.len(),
            flags,
            saddr.as_mut_ptr() as *mut _,
            &mut length,
        )
    } {
        -1 => Err(Error::last_os_error()),
        n => {
            let n = n as usize;
            unsafe {
                buf.assume_init(n);
            }
            buf.advance(n);

            if length != size_of::<SA::SysSockAddr>() as socklen_t {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "invalid sockaddr length",
                ));
            }
            let saddr = unsafe { saddr.assume_init() };
            let sa = SA::try_from_sys_sock_addr(saddr)?;

            Ok((n, sa))
        }
    }
}

/// Shut down part of a socket.
pub fn shutdown(socket: &OwnedFd, how: c_int) -> Result<()> {
    if unsafe { libc::shutdown(socket.as_raw_fd(), how) } == 0 {
        Ok(())
    } else {
        Err(Error::last_os_error())
    }
}

/// Get socket option.
pub fn getsockopt<T>(socket: &OwnedFd, level: c_int, optname: c_int) -> Result<T> {
    let mut optval: MaybeUninit<T> = MaybeUninit::uninit();
    let mut optlen: socklen_t = size_of::<T>() as _;
    if unsafe {
        libc::getsockopt(
            socket.as_raw_fd(),
            level,
            optname,
            optval.as_mut_ptr() as *mut _,
            &mut optlen,
        )
    } == -1
    {
        return Err(Error::last_os_error());
    }
    if optlen != size_of::<T>() as _ {
        return Err(Error::new(ErrorKind::InvalidInput, "invalid size"));
    }
    let optval = unsafe { optval.assume_init() };
    Ok(optval)
}

/// Set socket option.
pub fn setsockopt<T>(socket: &OwnedFd, level: c_int, optname: i32, optval: &T) -> Result<()> {
    let optlen: socklen_t = size_of::<T>() as _;
    if unsafe {
        libc::setsockopt(
            socket.as_raw_fd(),
            level,
            optname,
            optval as *const _ as *const _,
            optlen,
        )
    } == -1
    {
        return Err(Error::last_os_error());
    }
    Ok(())
}

/// Perform an IOCTL that reads a single value.
pub fn ioctl_read<T>(socket: &OwnedFd, request: Ioctl) -> Result<T> {
    let mut value: MaybeUninit<T> = MaybeUninit::uninit();
    let ret = unsafe { libc::ioctl(socket.as_raw_fd(), request, value.as_mut_ptr()) };
    if ret == -1 {
        return Err(Error::last_os_error());
    }
    let value = unsafe { value.assume_init() };
    Ok(value)
}

/// Perform an IOCTL that writes a single value.
#[allow(dead_code)]
pub fn ioctl_write<T>(socket: &OwnedFd, request: Ioctl, value: &T) -> Result<c_int> {
    let ret = unsafe { libc::ioctl(socket.as_raw_fd(), request, value as *const _) };
    if ret == -1 {
        return Err(Error::last_os_error());
    }
    Ok(ret)
}

/// Perform an FCNTL that reads a single value.
pub fn fcntl_read(socket: &OwnedFd) -> Result<c_int> {
    let ret = unsafe { libc::fcntl(socket.as_raw_fd(), F_GETFL, 0) };
    if ret == -1 {
        return Err(Error::last_os_error());
    }
    Ok(ret)
}

/// Perform an FCNTL that writes a single value.
pub fn fcntl_write(socket: &OwnedFd, flags: c_int) -> Result<c_int> {
    let ret = unsafe { libc::fcntl(socket.as_raw_fd(), F_SETFL, flags) };
    if ret == -1 {
        return Err(Error::last_os_error());
    }
    Ok(ret)
}

/// Private socket implementation functions.
macro_rules! sock_priv {
    () => {
        #[allow(dead_code)]
        async fn accept_priv(&self) -> Result<(Self, SocketAddr)> {
            let (fd, sa) = loop {
                let mut guard = self.fd.readable().await?;
                match guard.try_io(|inner| sock::accept(inner.get_ref())) {
                    Ok(result) => break result,
                    Err(_would_block) => continue,
                }
            }?;

            let socket = Self::from_owned_fd(fd)?;
            Ok((socket, sa))
        }

        #[allow(dead_code)]
        fn poll_accept_priv(&self, cx: &mut Context) -> Poll<Result<(Self, SocketAddr)>> {
            let (fd, sa) = loop {
                let mut guard = std::task::ready!(self.fd.poll_read_ready(cx))?;
                match guard.try_io(|inner| sock::accept(inner.get_ref())) {
                    Ok(result) => break result,
                    Err(_would_block) => continue,
                }
            }?;

            let socket = Self::from_owned_fd(fd)?;
            Poll::Ready(Ok((socket, sa)))
        }

        async fn connect_priv(&self, sa: SocketAddr) -> Result<()> {
            match sock::connect(self.fd.get_ref(), sa) {
                Ok(()) => Ok(()),
                Err(err)
                    if err.raw_os_error() == Some(EINPROGRESS)
                        || err.raw_os_error() == Some(EAGAIN) =>
                {
                    loop {
                        let mut guard = self.fd.writable().await?;
                        match guard.try_io(|inner| {
                            let err: c_int =
                                sock::getsockopt(inner.get_ref(), SOL_SOCKET, SO_ERROR)?;
                            match err {
                                0 => Ok(()),
                                EINPROGRESS | EAGAIN => Err(ErrorKind::WouldBlock.into()),
                                _ => Err(Error::from_raw_os_error(err)),
                            }
                        }) {
                            Ok(result) => break result,
                            Err(_would_block) => continue,
                        }
                    }?;
                    Ok(())
                }
                Err(err) => Err(err),
            }
        }

        #[allow(dead_code)]
        async fn send_priv(&self, buf: &[u8]) -> Result<usize> {
            loop {
                let mut guard = self.fd.writable().await?;
                match guard.try_io(|inner| sock::send(inner.get_ref(), buf, 0)) {
                    Ok(result) => return result,
                    Err(_would_block) => continue,
                }
            }
        }

        fn poll_send_priv(&self, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
            loop {
                let mut guard = std::task::ready!(self.fd.poll_write_ready(cx))?;
                match guard.try_io(|inner| sock::send(inner.get_ref(), buf, 0)) {
                    Ok(result) => return Poll::Ready(result),
                    Err(_would_block) => continue,
                }
            }
        }

        #[allow(dead_code)]
        async fn send_to_priv(&self, buf: &[u8], target: SocketAddr) -> Result<usize> {
            loop {
                let mut guard = self.fd.writable().await?;
                match guard.try_io(|inner| sock::sendto(inner.get_ref(), buf, 0, target)) {
                    Ok(result) => return result,
                    Err(_would_block) => continue,
                }
            }
        }

        #[allow(dead_code)]
        fn poll_send_to_priv(
            &self,
            cx: &mut Context,
            buf: &[u8],
            target: SocketAddr,
        ) -> Poll<Result<usize>> {
            loop {
                let mut guard = std::task::ready!(self.fd.poll_write_ready(cx))?;
                match guard.try_io(|inner| sock::sendto(inner.get_ref(), buf, 0, target)) {
                    Ok(result) => return Poll::Ready(result),
                    Err(_would_block) => continue,
                }
            }
        }

        #[allow(dead_code)]
        async fn recv_priv(&self, buf: &mut [u8]) -> Result<usize> {
            let mut buf = ReadBuf::new(buf);
            loop {
                let mut guard = self.fd.readable().await?;
                match guard.try_io(|inner| sock::recv(inner.get_ref(), &mut buf, 0)) {
                    Ok(result) => return result,
                    Err(_would_block) => continue,
                }
            }
        }

        fn poll_recv_priv(&self, cx: &mut Context, buf: &mut ReadBuf) -> Poll<Result<()>> {
            loop {
                let mut guard = std::task::ready!(self.fd.poll_read_ready(cx))?;
                match guard.try_io(|inner| sock::recv(inner.get_ref(), buf, 0)) {
                    Ok(result) => return Poll::Ready(result.map(|_| ())),
                    Err(_would_block) => continue,
                }
            }
        }

        #[allow(dead_code)]
        async fn recv_from_priv(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
            let mut buf = ReadBuf::new(buf);
            loop {
                let mut guard = self.fd.readable().await?;
                match guard.try_io(|inner| sock::recvfrom(inner.get_ref(), &mut buf, 0)) {
                    Ok(result) => return result,
                    Err(_would_block) => continue,
                }
            }
        }

        #[allow(dead_code)]
        fn poll_recv_from_priv(
            &self,
            cx: &mut Context,
            buf: &mut ReadBuf,
        ) -> Poll<Result<SocketAddr>> {
            loop {
                let mut guard = std::task::ready!(self.fd.poll_read_ready(cx))?;
                match guard.try_io(|inner| sock::recvfrom(inner.get_ref(), buf, 0)) {
                    Ok(result) => return Poll::Ready(result.map(|(_n, sa)| sa)),
                    Err(_would_block) => continue,
                }
            }
        }

        #[allow(dead_code)]
        async fn peek_priv(&self, buf: &mut [u8]) -> Result<usize> {
            let mut buf = ReadBuf::new(buf);
            loop {
                let mut guard = self.fd.readable().await?;
                match guard.try_io(|inner| sock::recv(inner.get_ref(), &mut buf, MSG_PEEK)) {
                    Ok(result) => return result,
                    Err(_would_block) => continue,
                }
            }
        }

        #[allow(dead_code)]
        fn poll_peek_priv(&self, cx: &mut Context, buf: &mut ReadBuf) -> Poll<Result<usize>> {
            loop {
                let mut guard = std::task::ready!(self.fd.poll_read_ready(cx))?;
                match guard.try_io(|inner| sock::recv(inner.get_ref(), buf, MSG_PEEK)) {
                    Ok(result) => return Poll::Ready(result),
                    Err(_would_block) => continue,
                }
            }
        }

        #[allow(dead_code)]
        fn poll_flush_priv(&self, _cx: &mut Context) -> Poll<Result<()>> {
            // Flush is a no-op.
            Poll::Ready(Ok(()))
        }

        fn shutdown_priv(&self, how: Shutdown) -> Result<()> {
            let how = match how {
                Shutdown::Read => SHUT_RD,
                Shutdown::Write => SHUT_WR,
                Shutdown::Both => SHUT_RDWR,
            };
            sock::shutdown(self.fd.get_ref(), how)?;
            Ok(())
        }

        #[allow(dead_code)]
        fn poll_shutdown_priv(&self, _cx: &mut Context, how: Shutdown) -> Poll<Result<()>> {
            self.shutdown_priv(how)?;
            Poll::Ready(Ok(()))
        }
    };
}

pub(crate) use sock_priv;
