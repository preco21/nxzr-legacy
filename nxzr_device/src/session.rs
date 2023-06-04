use crate::{
    sock::{self, l2cap},
    Address,
};
use thiserror::Error;

const DEFAULT_CTL_PSM: u16 = 17;
const DEFAULT_ITR_PSM: u16 = 19;

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("control/interrupt socket address must match with each other")]
    CtlItrSocketAddrMismatch,
    #[error(
        "failed to bind the target address, make sure that no `input` plugin enabled for bluetooth: {0}"
    )]
    BindFailed(std::io::Error),
    #[error("internal error: {0}")]
    Internal(SessionInternalError),
}

#[derive(Error, Debug)]
pub enum SessionInternalError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl From<std::io::Error> for SessionError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(err.into())
    }
}

#[derive(Debug, Default)]
pub struct SessionConfig {
    pub address: Option<Address>,
    pub control_psm: Option<u16>,
    pub interrupt_psm: Option<u16>,
}

#[derive(Debug)]
pub struct SessionListener {
    ctl_sock: l2cap::LazySeqPacketListener,
    itr_sock: l2cap::LazySeqPacketListener,
    addr_def: SessionAddress,
}

#[derive(Debug)]
struct SessionAddress {
    addr: Address,
    ctl_psm: u16,
    itr_psm: u16,
}

impl SessionListener {
    #[tracing::instrument(target = "session")]
    pub fn new(config: SessionConfig) -> Result<Self, SessionError> {
        tracing::info!("starting a session.");
        let control_psm = config.control_psm.unwrap_or(DEFAULT_CTL_PSM);
        let interrupt_psm = config.interrupt_psm.unwrap_or(DEFAULT_ITR_PSM);
        let ctl_sock = l2cap::LazySeqPacketListener::new()?;
        ctl_sock.enable_reuse_addr()?;
        let itr_sock = l2cap::LazySeqPacketListener::new()?;
        itr_sock.enable_reuse_addr()?;
        Ok(Self {
            ctl_sock,
            itr_sock,
            addr_def: SessionAddress {
                ctl_psm: control_psm,
                itr_psm: interrupt_psm,
                addr: config.address.unwrap_or(Address::default()),
            },
        })
    }

    #[tracing::instrument(target = "session")]
    pub async fn bind(&self) -> Result<(), SessionError> {
        tracing::info!("binding the session.");
        self.ctl_sock
            .bind(l2cap::SocketAddr {
                addr: self.addr_def.addr.into(),
                psm: self.addr_def.ctl_psm,
                addr_type: sock::AddressType::BrEdr,
                ..Default::default()
            })
            .await
            .map_err(|err| SessionError::BindFailed(err))?;
        self.itr_sock
            .bind(l2cap::SocketAddr {
                addr: self.addr_def.addr.into(),
                psm: self.addr_def.itr_psm,
                addr_type: sock::AddressType::BrEdr,
                ..Default::default()
            })
            .await
            .map_err(|err| SessionError::BindFailed(err))?;
        Ok(())
    }

    #[tracing::instrument(target = "session")]
    pub async fn listen(&self) -> Result<(), SessionError> {
        tracing::info!("start listening on the session.");
        self.ctl_sock.listen(1).await?;
        self.itr_sock.listen(1).await?;
        Ok(())
    }

    #[tracing::instrument(target = "session")]
    pub async fn accept(&self) -> Result<PairedSession, SessionError> {
        let (ctl_client, ctl_sa) = self.ctl_sock.accept().await?;
        tracing::info!(
            "accepted connection for `control` socket at psm \"{}\" from \"{}\".",
            ctl_sa.psm,
            ctl_sa.addr,
        );
        let (itr_client, itr_sa) = self.itr_sock.accept().await?;
        tracing::info!(
            "accepted connection for `interrupt` socket at psm \"{}\" from \"{}\".",
            itr_sa.psm,
            itr_sa.addr,
        );
        if ctl_sa.addr != itr_sa.addr {
            tracing::error!("assertion failed, control/interrupt socket address didn't match.");
            return Err(SessionError::CtlItrSocketAddrMismatch);
        }
        Ok(PairedSession::from_socket(
            (ctl_client, ctl_sa),
            (itr_client, itr_sa),
        ))
    }
}

#[derive(Debug, Default)]
pub struct PairedSessionConfig {
    pub reconnect_address: Address,
    pub control_psm: Option<u16>,
    pub interrupt_psm: Option<u16>,
}

#[derive(Debug)]
pub struct PairedSession {
    ctl_client: l2cap::SeqPacket,
    ctl_sa: l2cap::SocketAddr,
    itr_client: l2cap::SeqPacket,
    itr_sa: l2cap::SocketAddr,
}

impl PairedSession {
    pub async fn connect(config: PairedSessionConfig) -> Result<Self, SessionError> {
        let control_psm = config.control_psm.unwrap_or(DEFAULT_CTL_PSM);
        let interrupt_psm = config.interrupt_psm.unwrap_or(DEFAULT_ITR_PSM);
        let ctl_addr = l2cap::SocketAddr {
            addr: config.reconnect_address.into(),
            psm: control_psm,
            addr_type: sock::AddressType::BrEdr,
            ..Default::default()
        };
        let itr_addr = l2cap::SocketAddr {
            addr: config.reconnect_address.into(),
            psm: interrupt_psm,
            addr_type: sock::AddressType::BrEdr,
            ..Default::default()
        };
        Ok(Self {
            ctl_client: l2cap::SeqPacket::connect_blocking(ctl_addr).await?,
            ctl_sa: ctl_addr,
            itr_client: l2cap::SeqPacket::connect_blocking(itr_addr).await?,
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
