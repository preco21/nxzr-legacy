use nxzr_core::{
    controller::ControllerType,
    protocol::{Protocol, ProtocolConfig},
};
use nxzr_device::{
    session::PairedSession,
    transport::{Transport, TransportConfig},
    Address,
};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct ConnectionConfig {
    pub paired_session: PairedSession,
    pub dev_address: Address,
    pub controller_type: ControllerType,
    pub reconnect: bool,
}

#[derive(Debug)]
pub struct Connection {
    protocol: Protocol,
    transport: Transport,
    will_close_tx: mpsc::Sender<()>,
}

impl Connection {
    #[tracing::instrument(target = "connection")]
    pub async fn run(config: ConnectionConfig) -> anyhow::Result<(Self, ServerHandle)> {
        let ConnectionConfig {
            paired_session,
            dev_address,
            controller_type,
            reconnect,
        } = config;

        // Use that paired session for the further processing.
        let (transport, transport_handle) =
            Transport::register(paired_session, TransportConfig::default()).await?;
        let (protocol, protocol_handle) = Protocol::connect(
            transport.clone(),
            ProtocolConfig {
                dev_address: dev_address.into(),
                controller_type,
                reconnect,
                ..Default::default()
            },
        )
        .await?;

        let (close_tx, close_rx) = mpsc::channel(1);
        let (will_close_tx, will_close_rx) = mpsc::channel(1);
        tokio::spawn({
            let protocol = protocol.clone();
            let transport = transport.clone();
            async move {
                tokio::select! {
                    _ = protocol.closed() => {},
                    _ = transport.closed() => {},
                    _ = close_tx.closed() => {},
                }
                drop(will_close_rx);
                drop(protocol_handle);
                drop(transport_handle);
            }
        });

        Ok((
            Self {
                protocol,
                transport,
                will_close_tx,
            },
            ServerHandle {
                _close_rx: close_rx,
            },
        ))
    }

    pub fn protocol(&self) -> Protocol {
        self.protocol.clone()
    }

    pub async fn will_close(&self) {
        self.will_close_tx.closed().await;
    }

    pub async fn closed(&self) {
        self.protocol.closed().await;
        self.transport.closed().await;
    }
}

pub struct ServerHandle {
    _close_rx: mpsc::Receiver<()>,
}
