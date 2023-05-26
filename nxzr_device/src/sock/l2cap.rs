// Excerpt from `bluer` project: https://github.com/bluez/bluer/blob/8ffd4aeef3f8ab0d65dca66eb5a03f223351f586/bluer/src/l2cap.rs
//! Logical Link Control and Adaptation Protocol (L2CAP) sockets.
//!
//! L2CAP sockets provide Bluetooth Connection Oriented Channels (CoC).
//! This enables the efficient transfer of large data streams between two devices
//! using socket-oriented programming.
//!
//! L2CAP sockets work with both Bluetooth classic (BR/EDR) and Bluetooth Low Energy (LE).
//!
use crate::sock::{
    self, sock_priv,
    sys::{
        bt_power, bt_security, sockaddr_l2, BTPROTO_L2CAP, BT_MODE, BT_PHY, BT_POWER,
        BT_POWER_FORCE_ACTIVE_OFF, BT_POWER_FORCE_ACTIVE_ON, BT_RCVMTU, BT_SECURITY,
        BT_SECURITY_FIPS, BT_SECURITY_HIGH, BT_SECURITY_LOW, BT_SECURITY_MEDIUM, BT_SECURITY_SDP,
        BT_SNDMTU, L2CAP_CONNINFO, L2CAP_LM, L2CAP_OPTIONS, SOL_L2CAP,
    },
    AddressType, OwnedFd,
};
use crate::Address;
use libc::{
    AF_BLUETOOTH, EAGAIN, EINPROGRESS, MSG_PEEK, O_NONBLOCK, SHUT_RD, SHUT_RDWR, SHUT_WR,
    SOCK_NONBLOCK, SOCK_SEQPACKET, SOL_BLUETOOTH, SOL_SOCKET, SO_ERROR, SO_RCVBUF, SO_REUSEADDR,
    SO_SNDBUF, TIOCINQ, TIOCOUTQ,
};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use std::{
    convert::{TryFrom, TryInto},
    fmt,
    io::{Error, ErrorKind, Result},
    marker::PhantomData,
    net::Shutdown,
    os::{
        raw::c_int,
        unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
    },
    task::{Context, Poll},
};
use tokio::io::{unix::AsyncFd, ReadBuf};

pub use crate::sock::sys::{l2cap_conninfo as ConnInfo, l2cap_options as Opts};

/// Possible bit values for the [link mode socket option](Socket::link_mode).
pub mod link_mode {
    pub use crate::sock::sys::{
        L2CAP_LM_AUTH as AUTH, L2CAP_LM_ENCRYPT as ENCRYPT, L2CAP_LM_FIPS as FIPS,
        L2CAP_LM_MASTER as MASTER, L2CAP_LM_RELIABLE as RELIABLE, L2CAP_LM_SECURE as SECURE,
        L2CAP_LM_TRUSTED as TRUSTED,
    };
}

/// Possible bit values for the [PHY socket option](Socket::phy).
pub mod phy {
    pub use crate::sock::sys::{
        BR1M1SLOT, BR1M3SLOT, BR1M5SLOT, EDR2M1SLOT, EDR2M3SLOT, EDR2M5SLOT, EDR3M1SLOT,
        EDR3M3SLOT, EDR3M5SLOT, LE1MRX, LE1MTX, LE2MRX, LE2MTX, LECODEDRX, LECODEDTX,
    };
}

/// First unprivileged protocol service multiplexor (PSM) for
/// Bluetooth classic (BR/EDR).
///
/// Listening on a PSM below this requires the
/// `CAP_NET_BIND_SERVICE` capability.
pub const PSM_BR_EDR_DYN_START: u16 = 0x1001;

/// First unprivileged protocol service multiplexor (PSM) for Bluetooth LE.
///
/// Listening on a PSM below this requires the
/// `CAP_NET_BIND_SERVICE` capability.
pub const PSM_LE_DYN_START: u16 = 0x80;

/// The highest protocol service multiplexor (PSM) for Bluetooth Low Energy.
pub const PSM_LE_MAX: u16 = 0xff;

/// An L2CAP socket address.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SocketAddr {
    /// Device address.
    ///
    /// When listening or binding, specify [Address::any] for any local adapter address.
    pub addr: Address,
    /// Device address type.
    pub addr_type: AddressType,
    /// Protocol service multiplexor (PSM).
    ///
    /// For classic Bluetooth (BR/EDR), listening on a PSM below [PSM_BR_EDR_DYN_START]
    /// requires the `CAP_NET_BIND_SERVICE` capability.
    /// The PSM must be odd and the last bit of the upper byte must be zero, i.e.
    /// it must follow the bit pattern `xxxxxxx0_xxxxxxx1` where `x` may be `1` or `0`.
    ///
    /// For Bluetooth Low Energy, listening on a PSM below [PSM_LE_DYN_START]
    /// requires the `CAP_NET_BIND_SERVICE` capability.
    /// The highest allowed PSM for LE is [PSM_LE_MAX].
    ///
    /// Set to 0 for listening to assign an available PSM.
    pub psm: u16,
    /// Connection identifier (CID).
    ///
    /// Should be set to 0.
    pub cid: u16,
}

impl SocketAddr {
    /// Creates a new L2CAP socket address.
    pub const fn new(addr: Address, addr_type: AddressType, psm: u16) -> Self {
        Self {
            addr,
            addr_type,
            psm,
            cid: 0,
        }
    }

    /// When specified to [Socket::bind] binds to any local adapter address
    /// using classic Bluetooth (BR/EDR) and a dynamically allocated PSM.
    pub const fn any_br_edr() -> Self {
        Self {
            addr: Address::any(),
            addr_type: AddressType::BrEdr,
            psm: 0,
            cid: 0,
        }
    }

    /// When specified to [Socket::bind] binds to any public, local adapter address
    /// using Bluetooth Low Energy and a dynamically allocated PSM.
    pub const fn any_le() -> Self {
        Self {
            addr: Address::any(),
            addr_type: AddressType::LePublic,
            psm: 0,
            cid: 0,
        }
    }
}

impl sock::SysSockAddr for SocketAddr {
    type SysSockAddr = sockaddr_l2;

    fn into_sys_sock_addr(self) -> Self::SysSockAddr {
        sockaddr_l2 {
            l2_family: AF_BLUETOOTH as _,
            l2_psm: self.psm.to_le(),
            l2_cid: self.cid.to_le(),
            l2_bdaddr: self.addr.into(),
            l2_bdaddr_type: self.addr_type as _,
        }
    }

    fn try_from_sys_sock_addr(saddr: Self::SysSockAddr) -> Result<Self> {
        if saddr.l2_family != AF_BLUETOOTH as _ {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "sockaddr_l2::l2_family is not AF_BLUETOOTH",
            ));
        }
        Ok(Self {
            addr: Address::from(saddr.l2_bdaddr),
            addr_type: AddressType::from_u8(saddr.l2_bdaddr_type).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    "invalid sockaddr_l2::l2_bdaddr_type",
                )
            })?,
            psm: u16::from_le(saddr.l2_psm),
            cid: u16::from_le(saddr.l2_cid),
        })
    }
}

/// Any bind address for connecting to specified address.
fn any_bind_addr(addr: &SocketAddr) -> SocketAddr {
    match addr.addr_type {
        AddressType::BrEdr => SocketAddr::any_br_edr(),
        AddressType::LePublic | AddressType::LeRandom => SocketAddr::any_le(),
    }
}

/// L2CAP socket security level.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash, FromPrimitive, ToPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SecurityLevel {
    /// Insecure.
    Sdp = BT_SECURITY_SDP as _,
    /// Low: authentication is requested.
    Low = BT_SECURITY_LOW as _,
    /// Medium.
    Medium = BT_SECURITY_MEDIUM as _,
    /// High.
    High = BT_SECURITY_HIGH as _,
    /// FIPS.
    Fips = BT_SECURITY_FIPS as _,
}

/// L2CAP socket security.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Security {
    /// Level.
    pub level: SecurityLevel,
    /// Key size.
    pub key_size: u8,
}

impl From<Security> for bt_security {
    fn from(s: Security) -> Self {
        bt_security {
            level: s.level as _,
            key_size: s.key_size,
        }
    }
}

impl TryFrom<bt_security> for Security {
    type Error = Error;
    fn try_from(value: bt_security) -> Result<Self> {
        Ok(Self {
            level: SecurityLevel::from_u8(value.level)
                .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "invalid bt_security::level"))?,
            key_size: value.key_size,
        })
    }
}

/// L2CAP socket flow control mode.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash, FromPrimitive, ToPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FlowControl {
    /// LE flow control.
    Le = 0x03,
    /// Extended flow control.
    Extended = 0x04,
}

/// An L2CAP socket that has not yet been converted to a [StreamListener], [Stream], [SeqPacketListener],
/// [SeqPacket] or [Datagram].
///
/// The primary use of this is to configure the socket before connecting or listening.
pub struct Socket<Type> {
    fd: AsyncFd<OwnedFd>,
    _type: PhantomData<Type>,
}

impl<Type> fmt::Debug for Socket<Type> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Socket")
            .field("fd", &self.fd.as_raw_fd())
            .finish()
    }
}

impl<Type> Socket<Type> {
    /// Bind the socket to the given address.
    pub fn bind(&self, sa: SocketAddr) -> Result<()> {
        sock::bind(self.fd.get_ref(), sa)
    }

    /// Get the local address of this socket.
    pub fn local_addr(&self) -> Result<SocketAddr> {
        sock::getsockname(self.fd.get_ref())
    }

    /// Get the peer address of this socket.
    fn peer_addr_priv(&self) -> Result<SocketAddr> {
        sock::getpeername(self.fd.get_ref())
    }

    /// Get socket security.
    ///
    /// This corresponds to the `BT_SECURITY` socket option.
    pub fn security(&self) -> Result<Security> {
        let bts: bt_security = sock::getsockopt(self.fd.get_ref(), SOL_BLUETOOTH, BT_SECURITY)?;
        Security::try_from(bts)
    }

    /// Set socket security.
    ///
    /// This corresponds to the `BT_SECURITY` socket option.
    pub fn set_security(&self, security: Security) -> Result<()> {
        let bts: bt_security = security.into();
        sock::setsockopt(self.fd.get_ref(), SOL_BLUETOOTH, BT_SECURITY, &bts)
    }

    /// Get forced power state.
    ///
    /// This corresponds to the `BT_POWER` socket option.
    pub fn is_power_forced_active(&self) -> Result<bool> {
        let value: bt_power = sock::getsockopt(self.fd.get_ref(), SOL_BLUETOOTH, BT_POWER)?;
        Ok(value.force_active == BT_POWER_FORCE_ACTIVE_ON as _)
    }

    /// Set forced power state.
    ///
    /// This corresponds to the `BT_POWER` socket option.
    pub fn set_power_forced_active(&self, power_forced_active: bool) -> Result<()> {
        let value = bt_power {
            force_active: if power_forced_active {
                BT_POWER_FORCE_ACTIVE_ON
            } else {
                BT_POWER_FORCE_ACTIVE_OFF
            } as _,
        };
        sock::setsockopt(self.fd.get_ref(), SOL_BLUETOOTH, BT_POWER, &value)
    }

    /// Get maximum transmission unit (MTU) for sending.
    ///
    /// This corresponds to the `BT_SNDMTU` socket option or [Opts::omtu].
    ///
    /// Note that this value may not be available directly after an connection
    /// has been established and this function will return an error.
    /// In this case, try re-querying the MTU after send or receiving some data.
    pub fn send_mtu(&self) -> Result<u16> {
        match self.local_addr()?.addr_type {
            AddressType::BrEdr => Ok(self.l2cap_opts()?.omtu),
            _ => sock::getsockopt(self.fd.get_ref(), SOL_BLUETOOTH, BT_SNDMTU),
        }
    }

    /// Get maximum transmission unit (MTU) for receiving.
    ///
    /// This corresponds to the `BT_RCVMTU` socket option or [Opts::imtu].
    pub fn recv_mtu(&self) -> Result<u16> {
        match self.local_addr()?.addr_type {
            AddressType::BrEdr => Ok(self.l2cap_opts()?.imtu),
            _ => sock::getsockopt(self.fd.get_ref(), SOL_BLUETOOTH, BT_RCVMTU),
        }
    }

    /// Set receive MTU.
    ///
    /// This corresponds to the `BT_RCVMTU` socket option or [Opts::imtu].
    pub fn set_recv_mtu(&self, recv_mtu: u16) -> Result<()> {
        match self.local_addr()?.addr_type {
            AddressType::BrEdr => {
                let mut opts = self.l2cap_opts()?;
                opts.imtu = recv_mtu;
                self.set_l2cap_opts(&opts)
            }
            _ => sock::setsockopt(self.fd.get_ref(), SOL_BLUETOOTH, BT_RCVMTU, &recv_mtu),
        }
    }

    /// Get flow control mode.
    ///
    /// This corresponds to the `BT_MODE` socket option.
    pub fn flow_control(&self) -> Result<FlowControl> {
        let value: u8 = sock::getsockopt(self.fd.get_ref(), SOL_BLUETOOTH, BT_MODE)?;
        FlowControl::from_u8(value)
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "invalid flow control mode"))
    }

    /// Set flow control mode.
    ///
    /// This corresponds to the `BT_MODE` socket option.
    pub fn set_flow_control(&self, flow_control: FlowControl) -> Result<()> {
        let value = flow_control as u8;
        sock::setsockopt(self.fd.get_ref(), SOL_BLUETOOTH, BT_MODE, &value)
    }

    /// Gets the maximum socket receive buffer in bytes.
    ///
    /// This corresponds to the `SO_RCVBUF` socket option.
    pub fn recv_buffer(&self) -> Result<i32> {
        sock::getsockopt(self.fd.get_ref(), SOL_SOCKET, SO_RCVBUF)
    }

    /// Sets the maximum socket receive buffer in bytes.
    ///
    /// This corresponds to the `SO_RCVBUF` socket option.
    pub fn set_recv_buffer(&self, recv_buffer: i32) -> Result<()> {
        sock::setsockopt(self.fd.get_ref(), SOL_SOCKET, SO_RCVBUF, &recv_buffer)
    }

    /// Gets the raw L2CAP socket options.
    ///
    /// This corresponds to the `L2CAP_OPTIONS` socket option.
    /// This is only supported by classic sockets, i.e. [SocketAddr::addr_type] is
    /// [AddressType::BrEdr].
    pub fn l2cap_opts(&self) -> Result<Opts> {
        sock::getsockopt(self.fd.get_ref(), SOL_L2CAP, L2CAP_OPTIONS)
    }

    /// Sets the raw L2CAP socket options.
    ///
    /// This corresponds to the `L2CAP_OPTIONS` socket option.
    /// This is only supported by classic sockets, i.e. [SocketAddr::addr_type] is
    /// [AddressType::BrEdr].
    pub fn set_l2cap_opts(&self, l2cap_opts: &Opts) -> Result<()> {
        sock::setsockopt(self.fd.get_ref(), SOL_L2CAP, L2CAP_OPTIONS, l2cap_opts)
    }

    /// Gets the raw L2CAP link mode bit field.
    ///
    /// Possible values are defined in the [link_mode] module.
    /// This corresponds to the `L2CAP_LM` socket option.
    pub fn link_mode(&self) -> Result<i32> {
        sock::getsockopt(self.fd.get_ref(), SOL_L2CAP, L2CAP_LM)
    }

    /// Sets the raw L2CAP link mode bit field.
    ///
    /// Possible values are defined in the [link_mode] module.
    /// This corresponds to the `L2CAP_LM` socket option.
    pub fn set_link_mode(&self, link_mode: i32) -> Result<()> {
        sock::setsockopt(self.fd.get_ref(), SOL_L2CAP, L2CAP_LM, &link_mode)
    }

    /// Gets the L2CAP socket connection information.
    ///
    /// This corresponds to the `L2CAP_CONNINFO` socket option.
    pub fn conn_info(&self) -> Result<ConnInfo> {
        sock::getsockopt(self.fd.get_ref(), SOL_L2CAP, L2CAP_CONNINFO)
    }

    /// Gets the supported PHYs bit field.
    ///
    /// Possible values are defined in the [phy] module.
    /// This corresponds to the `BT_PHY` socket option.
    pub fn phy(&self) -> Result<i32> {
        sock::getsockopt(self.fd.get_ref(), SOL_BLUETOOTH, BT_PHY)
    }

    /// Get the number of bytes in the input buffer.
    ///
    /// This corresponds to the `TIOCINQ` IOCTL.
    pub fn input_buffer(&self) -> Result<u32> {
        let value: c_int = sock::ioctl_read(self.fd.get_ref(), TIOCINQ)?;
        Ok(value as _)
    }

    /// Get the number of bytes in the output buffer.
    ///
    /// This corresponds to the `TIOCOUTQ` IOCTL.
    pub fn output_buffer(&self) -> Result<u32> {
        let value: c_int = sock::ioctl_read(self.fd.get_ref(), TIOCOUTQ)?;
        Ok(value as _)
    }

    /// Constructs a new [Socket] from the given raw file descriptor.
    ///
    /// The file descriptor must have been set to non-blocking mode.
    ///
    /// This function *consumes ownership* of the specified file descriptor.
    /// The returned object will take responsibility for closing it when the object goes out of scope.
    ///
    /// # Safety
    /// If the passed file descriptor is invalid, undefined behavior may occur.
    pub unsafe fn from_raw_fd(fd: RawFd) -> Result<Self> {
        Ok(Self {
            fd: AsyncFd::new(OwnedFd::new(fd))?,
            _type: PhantomData,
        })
    }

    fn from_owned_fd(fd: OwnedFd) -> Result<Self> {
        Ok(Self {
            fd: AsyncFd::new(fd)?,
            _type: PhantomData,
        })
    }

    sock_priv!();
}

impl<Type> AsRawFd for Socket<Type> {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl<Type> IntoRawFd for Socket<Type> {
    fn into_raw_fd(self) -> RawFd {
        self.fd.into_inner().into_raw_fd()
    }
}

impl<Type> FromRawFd for Socket<Type> {
    /// Constructs a new instance of `Self` from the given raw file
    /// descriptor.
    ///
    /// The file descriptor must have been set to non-blocking mode.
    ///
    /// # Panics
    /// Panics when the conversion fails.
    /// Use [Socket::from_raw_fd] for a non-panicking variant.
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self::from_raw_fd(fd).expect("from_raw_fd failed")
    }
}

impl Socket<SeqPacket> {
    /// Creates a new socket of sequential packet type.
    pub fn new_seq_packet() -> Result<Socket<SeqPacket>> {
        Ok(Self {
            fd: AsyncFd::new(sock::socket(
                AF_BLUETOOTH,
                SOCK_SEQPACKET | SOCK_NONBLOCK,
                BTPROTO_L2CAP,
            )?)?,
            _type: PhantomData,
        })
    }

    /// Creates a new socket of sequential packet type in blocking mode.
    pub fn new_blocking_seq_packet() -> Result<Socket<SeqPacket>> {
        Ok(Self {
            fd: AsyncFd::new(sock::socket(AF_BLUETOOTH, SOCK_SEQPACKET, BTPROTO_L2CAP)?)?,
            _type: PhantomData,
        })
    }

    /// Sets `SO_REUSEADDR` socket option to `1`.
    pub fn enable_reuse_addr(&self) -> Result<()> {
        sock::setsockopt(self.fd.get_ref(), SOL_SOCKET, SO_REUSEADDR, &1)
    }

    /// Convert the socket into a [SeqPacketListener].
    ///
    /// `backlog` defines the maximum number of pending connections are queued by the operating system
    /// at any given time.
    pub fn listen(self, backlog: u32) -> Result<SeqPacketListener> {
        sock::listen(
            self.fd.get_ref(),
            backlog
                .try_into()
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "invalid backlog"))?,
        )?;
        Ok(SeqPacketListener { socket: self })
    }

    /// Listen on the socket.
    ///
    /// `backlog` defines the maximum number of pending connections are queued by the operating system
    /// at any given time.
    pub fn listen_on(&self, backlog: u32) -> Result<()> {
        sock::listen(
            self.fd.get_ref(),
            backlog
                .try_into()
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "invalid backlog"))?,
        )?;
        Ok(())
    }

    /// Establish a sequential packet connection with a peer at the specified socket address.
    ///
    /// Also sets non-blocking mode after the connect.
    pub async fn connect(self, sa: SocketAddr) -> Result<SeqPacket> {
        self.connect_priv(sa).await?;
        Ok(SeqPacket { socket: self })
    }

    /// Establish a sequential packet connection with a peer at the specified socket address.
    ///
    /// Also sets non-blocking mode after the connect.
    pub async fn connect_blocking(self, sa: SocketAddr) -> Result<SeqPacket> {
        self.connect_priv(sa).await?;
        // NOTE: The `fd` should be set to non-blocking mode AFTER the connect.
        // Otherwise, some hosts may refuse to connect.
        let flags = sock::fcntl_read(self.fd.get_ref())?;
        sock::fcntl_write(self.fd.get_ref(), flags | O_NONBLOCK)?;
        Ok(SeqPacket { socket: self })
    }
}

/// An L2CAP socket server, listening for [SeqPacket] connections.
#[derive(Debug)]
pub struct SeqPacketListener {
    socket: Socket<SeqPacket>,
}

impl SeqPacketListener {
    /// Creates a new Listener, which will be bound to the specified socket address.
    ///
    /// Specify [SocketAddr::any_br_edr] or [SocketAddr::any_le] for any local adapter
    /// address with a dynamically allocated PSM.
    pub async fn bind(sa: SocketAddr) -> Result<Self> {
        let socket = Socket::<SeqPacket>::new_seq_packet()?;
        socket.bind(sa)?;
        socket.listen(1)
    }

    /// Accepts a new incoming connection from this listener.
    pub async fn accept(&self) -> Result<(SeqPacket, SocketAddr)> {
        let (socket, sa) = self.socket.accept_priv().await?;
        Ok((SeqPacket { socket }, sa))
    }

    /// Polls to accept a new incoming connection to this listener.
    pub fn poll_accept(&self, cx: &mut Context) -> Poll<Result<(SeqPacket, SocketAddr)>> {
        let (socket, sa) = std::task::ready!(self.socket.poll_accept_priv(cx))?;
        Poll::Ready(Ok((SeqPacket { socket }, sa)))
    }

    /// Constructs a new [SeqPacketListener] from the given raw file descriptor.
    ///
    /// The file descriptor must have been set to non-blocking mode.
    ///
    /// This function *consumes ownership* of the specified file descriptor.
    /// The returned object will take responsibility for closing it when the object goes out of scope.
    ///
    /// # Safety
    /// If the passed file descriptor is invalid, undefined behavior may occur.
    pub unsafe fn from_raw_fd(fd: RawFd) -> Result<Self> {
        Ok(Self {
            socket: Socket::from_raw_fd(fd)?,
        })
    }
}

impl AsRef<Socket<SeqPacket>> for SeqPacketListener {
    fn as_ref(&self) -> &Socket<SeqPacket> {
        &self.socket
    }
}

impl AsRawFd for SeqPacketListener {
    fn as_raw_fd(&self) -> RawFd {
        self.socket.as_raw_fd()
    }
}

impl FromRawFd for SeqPacketListener {
    /// Constructs a new instance of `Self` from the given raw file
    /// descriptor.
    ///
    /// The file descriptor must have been set to non-blocking mode.
    ///
    /// # Panics
    /// Panics when the conversion fails.
    /// Use [SeqPacketListener::from_raw_fd] for a non-panicking variant.
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self::from_raw_fd(fd).expect("from_raw_fd failed")
    }
}

/// An L2CAP sequential packet socket (sequenced, reliable, two-way connection-based data transmission path for
/// datagrams of fixed maximum length).
#[derive(Debug)]
pub struct SeqPacket {
    socket: Socket<SeqPacket>,
}

impl SeqPacket {
    /// Establish a sequential packet connection with a peer at the specified socket address.
    ///
    /// Uses any local Bluetooth adapter.
    pub async fn connect(addr: SocketAddr) -> Result<Self> {
        let socket = Socket::<SeqPacket>::new_seq_packet()?;
        socket.bind(any_bind_addr(&addr))?;
        socket.connect(addr).await
    }

    pub async fn connect_blocking(addr: SocketAddr) -> Result<Self> {
        let socket = Socket::<SeqPacket>::new_blocking_seq_packet()?;
        socket.bind(any_bind_addr(&addr))?;
        socket.connect_blocking(addr).await
    }

    /// Resets `SO_SNDBUF` value to `0`
    pub fn reset_sndbuf(&self) -> Result<()> {
        // Resetting socket option `SO_SNDBUF` to `0` let the OS to set the
        // value to its default like `4608`.
        // Ref: https://www.ibm.com/docs/en/ztpf/1.1.0.15?topic=apis-setsockopt-set-options-associated-socket
        let owned_fd = unsafe { OwnedFd::new(self.socket.as_raw_fd()) };
        let optval = 0;
        sock::setsockopt(&owned_fd, SOL_SOCKET, SO_SNDBUF, &optval)
    }

    /// Gets the peer address of this stream.
    pub fn peer_addr(&self) -> Result<SocketAddr> {
        self.socket.peer_addr_priv()
    }

    /// Sends a packet.
    ///
    /// The packet length must not exceed the [Self::send_mtu].
    pub async fn send(&self, buf: &[u8]) -> Result<usize> {
        self.socket.send_priv(buf).await
    }

    /// Attempts to send a packet.
    ///
    /// The packet length must not exceed the [Self::send_mtu].
    pub fn poll_send(&self, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        self.socket.poll_send_priv(cx, buf)
    }

    /// Receives a packet.
    ///
    /// The provided buffer must be of length [Self::recv_mtu], otherwise
    /// the packet may be truncated.
    pub async fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        self.socket.recv_priv(buf).await
    }

    /// Attempts to receive a packet.
    ///
    /// The provided buffer must be of length [Self::recv_mtu], otherwise
    /// the packet may be truncated.
    pub fn poll_recv(&self, cx: &mut Context, buf: &mut ReadBuf) -> Poll<Result<()>> {
        self.socket.poll_recv_priv(cx, buf)
    }

    /// Shuts down the read, write, or both halves of this connection.
    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.socket.shutdown_priv(how)
    }

    /// Maximum transmission unit (MTU) for sending.
    pub fn send_mtu(&self) -> Result<usize> {
        self.socket.send_mtu().map(|v| v.into())
    }

    /// Maximum transmission unit (MTU) for receiving.
    pub fn recv_mtu(&self) -> Result<usize> {
        self.socket.recv_mtu().map(|v| v.into())
    }

    /// Constructs a new [SeqPacket] from the given raw file descriptor.
    ///
    /// The file descriptor must have been set to non-blocking mode.
    ///
    /// This function *consumes ownership* of the specified file descriptor.
    /// The returned object will take responsibility for closing it when the object goes out of scope.
    ///
    /// # Safety
    /// If the passed file descriptor is invalid, undefined behavior may occur.
    pub unsafe fn from_raw_fd(fd: RawFd) -> Result<Self> {
        Ok(Self {
            socket: Socket::from_raw_fd(fd)?,
        })
    }
}

impl AsRef<Socket<SeqPacket>> for SeqPacket {
    fn as_ref(&self) -> &Socket<SeqPacket> {
        &self.socket
    }
}

impl AsRawFd for SeqPacket {
    fn as_raw_fd(&self) -> RawFd {
        self.socket.as_raw_fd()
    }
}

impl FromRawFd for SeqPacket {
    /// Constructs a new instance of `Self` from the given raw file
    /// descriptor.
    ///
    /// The file descriptor must have been set to non-blocking mode.
    ///
    /// # Panics
    /// Panics when the conversion fails.
    /// Use [SeqPacket::from_raw_fd] for a non-panicking variant.
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self::from_raw_fd(fd).expect("from_raw_fd failed")
    }
}

#[derive(Debug)]
pub struct LazySeqPacketListener {
    socket: Socket<SeqPacket>,
}

impl LazySeqPacketListener {
    // Creates a new Listener without immediate binding.
    pub fn new() -> Result<Self> {
        let socket = Socket::<SeqPacket>::new_seq_packet()?;
        Ok(Self { socket })
    }

    // Sets `SO_REUSEADDR` socket option to `1`.
    pub fn enable_reuse_addr(&self) -> Result<()> {
        self.socket.enable_reuse_addr()?;
        Ok(())
    }

    // Binds the socket.
    pub async fn bind(&self, sa: SocketAddr) -> Result<()> {
        self.socket.bind(sa)?;
        Ok(())
    }

    // Accepts a new incoming connection from this listener.
    pub async fn accept(&self) -> Result<(SeqPacket, SocketAddr)> {
        let (socket, sa) = self.socket.accept_priv().await?;
        Ok((SeqPacket { socket }, sa))
    }

    // Polls to accept a new incoming connection to this listener.
    pub fn poll_accept(&self, cx: &mut Context) -> Poll<Result<(SeqPacket, SocketAddr)>> {
        let (socket, sa) = std::task::ready!(self.socket.poll_accept_priv(cx))?;
        Poll::Ready(Ok((SeqPacket { socket }, sa)))
    }

    // Start listen on the socket.
    pub async fn listen(&self, backlog: u32) -> Result<()> {
        self.socket.listen_on(backlog)?;
        Ok(())
    }

    pub unsafe fn from_raw_fd(fd: RawFd) -> Result<Self> {
        Ok(Self {
            socket: Socket::from_raw_fd(fd)?,
        })
    }
}

impl AsRef<Socket<SeqPacket>> for LazySeqPacketListener {
    fn as_ref(&self) -> &Socket<SeqPacket> {
        &self.socket
    }
}

impl AsRawFd for LazySeqPacketListener {
    fn as_raw_fd(&self) -> RawFd {
        self.socket.as_raw_fd()
    }
}

impl FromRawFd for LazySeqPacketListener {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self::from_raw_fd(fd).expect("from_raw_fd failed")
    }
}
