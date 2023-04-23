use crate::sock::{
    l2cap::{self, LazySeqPacketListener, SocketAddr},
    Address,
};
use thiserror::Error;

const DEFAULT_CTL_PSM: u16 = 17;
const DEFAULT_ITR_PSM: u16 = 19;

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
    control_psm: Option<u16>,
    interrupt_psm: Option<u16>,
    device_address: Option<Address>,
}

#[derive(Debug)]
pub struct SessionListener {
    ctl_sock: LazySeqPacketListener,
    itr_sock: LazySeqPacketListener,
    addr_def: SessionAddressDef,
}

#[derive(Debug)]
struct SessionAddressDef {
    control_psm: u16,
    interrupt_psm: u16,
    device_address: Address,
}

impl SessionListener {
    #[tracing::instrument(target = "session")]
    pub fn new(config: SessionConfig) -> Result<Self, SessionError> {
        tracing::info!("initiating a session");
        let control_psm = config.control_psm.unwrap_or(DEFAULT_CTL_PSM);
        let interrupt_psm = config.interrupt_psm.unwrap_or(DEFAULT_ITR_PSM);
        let ctl_sock = LazySeqPacketListener::new()?;
        ctl_sock.enable_reuse_addr()?;
        let itr_sock = LazySeqPacketListener::new()?;
        itr_sock.enable_reuse_addr()?;
        Ok(Self {
            ctl_sock,
            itr_sock,
            addr_def: SessionAddressDef {
                control_psm,
                interrupt_psm,
                device_address: config.device_address.unwrap_or(Address::default()),
            },
        })
    }

    #[tracing::instrument(target = "session")]
    pub async fn bind(&self) -> Result<(), SessionError> {
        tracing::info!("binding the session");
        self.ctl_sock
            .bind(SocketAddr {
                addr: self.addr_def.device_address,
                psm: self.addr_def.control_psm,
                ..Default::default()
            })
            .await?;
        self.itr_sock
            .bind(SocketAddr {
                addr: self.addr_def.device_address,
                psm: self.addr_def.interrupt_psm,
                ..Default::default()
            })
            .await?;
        // FIXME: implement a fallback when bluez input plugin is enabled
        Ok(())
    }

    #[tracing::instrument(target = "session")]
    pub async fn listen(&self) -> Result<(), SessionError> {
        tracing::info!("start listening on the session");
        self.ctl_sock.listen(1).await?;
        self.itr_sock.listen(1).await?;
        Ok(())
    }

    pub async fn accept(&self) -> Result<PairedSession, SessionError> {
        let ctl_pair = self.ctl_sock.accept().await?;
        let itr_pair = self.itr_sock.accept().await?;
        Ok(PairedSession::from_socket(ctl_pair, itr_pair))
    }
}

#[derive(Debug)]
pub struct PairedSession {
    ctl_client: l2cap::SeqPacket,
    ctl_sa: l2cap::SocketAddr,
    itr_client: l2cap::SeqPacket,
    itr_sa: l2cap::SocketAddr,
}

impl PairedSession {
    pub async fn connect(config: SessionConfig) -> Result<Self, SessionError> {
        let control_psm = config.control_psm.unwrap_or(DEFAULT_CTL_PSM);
        let interrupt_psm = config.interrupt_psm.unwrap_or(DEFAULT_ITR_PSM);
        let ctl_addr = SocketAddr {
            addr: config.device_address.unwrap_or(Address::default()),
            psm: control_psm,
            ..Default::default()
        };
        let itr_addr = SocketAddr {
            addr: config.device_address.unwrap_or(Address::default()),
            psm: interrupt_psm,
            ..Default::default()
        };
        Ok(Self {
            ctl_client: l2cap::SeqPacket::connect(ctl_addr).await?,
            ctl_sa: ctl_addr,
            itr_client: l2cap::SeqPacket::connect(itr_addr).await?,
            itr_sa: itr_addr,
        })
    }

    pub(crate) fn from_socket(
        ctl_pair: (l2cap::SeqPacket, l2cap::SocketAddr),
        itr_pair: (l2cap::SeqPacket, l2cap::SocketAddr),
    ) -> Self {
        let (ctl_client, ctl_sa) = ctl_pair;
        let (itr_client, itr_sa) = itr_pair;
        Self {
            ctl_client,
            ctl_sa,
            itr_client,
            itr_sa,
        }
    }

    pub fn ctl_client(&self) -> &l2cap::SeqPacket {
        &self.ctl_client
    }

    pub fn ctl_socket_addr(&self) -> l2cap::SocketAddr {
        self.ctl_sa
    }

    pub fn itr_client(&self) -> &l2cap::SeqPacket {
        &self.itr_client
    }

    pub fn itr_socket_addr(&self) -> l2cap::SocketAddr {
        self.itr_sa
    }
}
