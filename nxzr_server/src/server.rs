use nxzr_core::{
    controller::ControllerType,
    protocol::{Protocol, ProtocolConfig},
};
use nxzr_device::{
    establish_initial_connection, establish_reconnect_connection,
    transport::{Transport, TransportConfig},
    ReconnectType,
};
use tokio::sync::mpsc;

#[derive(Debug, Default)]
pub struct ServerOpts {
    pub dev_id: Option<String>,
    pub reconnect: Option<ReconnectType>,
    pub controller_type: ControllerType,
}

#[derive(Debug)]
pub struct Server {
    protocol: Protocol,
    transport: Transport,
    will_close_tx: mpsc::Sender<()>,
}

impl Server {
    #[tracing::instrument(target = "server")]
    pub async fn run(opts: ServerOpts) -> anyhow::Result<(Self, ServerHandle)> {
        let (paired_session, address, reconnect) = match opts.reconnect {
            Some(reconnect) => {
                let (paired_session, address) =
                    establish_reconnect_connection(opts.dev_id, opts.controller_type, reconnect)
                        .await?;
                (paired_session, address, true)
            }
            None => {
                let (paired_session, address) =
                    establish_initial_connection(opts.dev_id, opts.controller_type).await?;
                (paired_session, address, false)
            }
        };

        // Use that paired session for the further processing.
        let (transport, transport_handle) =
            Transport::register(paired_session, TransportConfig::default()).await?;
        let (protocol, protocol_handle) = Protocol::connect(
            transport.clone(),
            ProtocolConfig {
                dev_address: address.into(),
                controller_type: opts.controller_type,
                reconnect,
                ..Default::default()
            },
        )
        .await?;

        // Start listening for protocol events.
        let mut event_rx = protocol.events().await?;
        tokio::spawn(async move {
            while let Some(evt) = event_rx.recv().await {
                tracing::info!("protocol: {}", &evt.to_string());
            }
        });

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

    pub fn transport(&self) -> Transport {
        self.transport.clone()
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
