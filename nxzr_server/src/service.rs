use nxzr_core::{controller, protocol};
use nxzr_device::{connection, device, session};
use nxzr_proto::{
    connect_switch_response, connection_event, nxzr_server::Nxzr, ButtonControlStreamRequest,
    ButtonControlStreamResponse, ConnectSwitchRequest, ConnectSwitchResponse, ConnectionEvent,
    ConnectionMetadata, Error as ProtoError, GetDeviceStatusRequest, GetDeviceStatusResponse,
    GetProtocolStateRequest, GetProtocolStateResponse, ImuControlStreamRequest,
    ImuControlStreamResponse, ReconnectSwitchRequest, ReconnectSwitchResponse,
    StickControlStreamRequest, StickControlStreamResponse,
};
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    time::SystemTime,
};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::UnboundedReceiverStream, Stream};
use tokio_util::sync::CancellationToken;
use tonic::{async_trait, Request, Response, Status, Streaming};

type ServiceResult<T> = Result<Response<T>, Status>;
type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

#[derive(Debug, Error)]
pub enum NxzrServiceError {
    #[error("connection state must be `NotConnected` in order to connect/reconnect")]
    ConnectionStateInvariant,
    #[error("stream closed by peer")]
    StreamClosed,
    #[error("failed to connect to peer")]
    ConnectionFailed,
    #[error(transparent)]
    DeviceError(#[from] device::DeviceError),
    #[error(transparent)]
    SessionError(#[from] session::SessionError),
    #[error(transparent)]
    ConnectionError(#[from] connection::ConnectionError),
    #[error(transparent)]
    ProtocolError(#[from] protocol::ProtocolError),
}

impl From<NxzrServiceError> for Status {
    fn from(err: NxzrServiceError) -> Self {
        Self::internal(err.to_string())
    }
}

#[derive(Debug)]
pub struct NxzrService {
    device: Arc<device::Device>,
    conn_state: Arc<Mutex<ConnectionState>>,
    shutdown_token: CancellationToken,
    shutdown_complete_tx: mpsc::Sender<()>,
}

#[derive(Debug)]
enum ConnectionState {
    NotConnected,
    Connecting,
    Connected(Arc<connection::Connection>),
    Disconnecting,
}

impl NxzrService {
    pub async fn new(
        device: Arc<device::Device>,
        shutdown_token: CancellationToken,
        shutdown_complete_tx: mpsc::Sender<()>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            device,
            conn_state: Arc::new(Mutex::new(ConnectionState::NotConnected)),
            shutdown_token,
            shutdown_complete_tx,
        })
    }
}

#[async_trait]
impl Nxzr for NxzrService {
    #[tracing::instrument(target = "service")]
    async fn get_device_status(
        &self,
        _req: Request<GetDeviceStatusRequest>,
    ) -> ServiceResult<GetDeviceStatusResponse> {
        let adapter_addr = self
            .device
            .address()
            .await
            .map_err(|err| NxzrServiceError::from(err))?;
        let paired_switch_addresses = self
            .device
            .paired_switches()
            .await
            .map_err(|err| NxzrServiceError::from(err))?
            .iter()
            .map(|dev| dev.address().to_string())
            .collect::<Vec<_>>();
        Ok(Response::new(GetDeviceStatusResponse {
            adapter_address: adapter_addr.to_string(),
            paired_switch_addresses,
        }))
    }

    type ConnectSwitchStream = ResponseStream<ConnectSwitchResponse>;
    #[tracing::instrument(target = "service")]
    async fn connect_switch(
        &self,
        req: Request<ConnectSwitchRequest>,
    ) -> ServiceResult<Self::ConnectSwitchStream> {
        // Start connection.
        {
            let mut guard = self.conn_state.lock().unwrap();
            let conn_state = &*guard;
            match conn_state {
                ConnectionState::NotConnected => {}
                _ => return Err(NxzrServiceError::ConnectionStateInvariant.into()),
            }
            *guard = ConnectionState::Connecting;
        }

        let (stream_tx, stream_rx) =
            mpsc::unbounded_channel::<Result<ConnectSwitchResponse, Status>>();

        tokio::spawn({
            let shutdown_complete_guard = self.shutdown_complete_tx.clone();
            let shutdown_token = self.shutdown_token.clone();
            let device = self.device.clone();
            let conn_state = self.conn_state.clone();
            async move {
                let connect_switch_fut = handle_connect_switch(device, stream_tx.clone());
                let res = tokio::select! {
                    res = connect_switch_fut => Some(res),
                    _ = stream_tx.closed() => None,
                    _ = shutdown_token.cancelled() => None,
                };
                match res {
                    Some(Ok((conn, conn_handle))) => {
                        let conn = Arc::new(conn);
                        // Set connected.
                        {
                            let mut guard = conn_state.lock().unwrap();
                            *guard = ConnectionState::Connected(conn.clone());
                        }
                        // Send Event: Connected
                        let _ = stream_tx.send(create_res(create_event(
                            connection_event::Kind::Log(connection_event::EventLog {
                                kind: connection_event::EventLogKind::Connected.into(),
                                message: "Connected to Switch.".to_string(),
                                ..Default::default()
                            }),
                        )));
                        // FIXME: Handle/move Send Evnet: Connecting here
                        // Wait for either ends to be closed.
                        tokio::select! {
                            _ = conn.will_close() => {
                                tracing::warn!("terminating connection due to connection lost");
                            },
                            _ = stream_tx.closed() => {
                                tracing::warn!("terminating connection due to stream closed");
                            },
                            _ = shutdown_token.cancelled() => {
                                tracing::warn!("terminating connection due to shutdown signal");
                            },
                        }
                        // Send Event: Disconnecting
                        let _ = stream_tx.send(create_res(create_event(
                            connection_event::Kind::Log(connection_event::EventLog {
                                kind: connection_event::EventLogKind::Disconnecting.into(),
                                message: "Disconnection in progress...".to_string(),
                                ..Default::default()
                            }),
                        )));
                        drop(conn_handle);
                        conn.closed().await;
                    }
                    Some(Err(err)) => {
                        tracing::warn!("failed to connect: {}", err);
                        let _ = stream_tx.send(Err(err.into()));
                    }
                    None => {
                        tracing::warn!("stream closed");
                        let _ = stream_tx.send(Err(NxzrServiceError::StreamClosed.into()));
                    }
                }
                // Set disconnected.
                {
                    let mut guard = conn_state.lock().unwrap();
                    *guard = ConnectionState::NotConnected;
                }
                // Send Event: Disconnected
                let _ = stream_tx.send(create_res(create_event(connection_event::Kind::Log(
                    connection_event::EventLog {
                        kind: connection_event::EventLogKind::Disconnected.into(),
                        message: "Successfully disconnected from Switch.".to_string(),
                        ..Default::default()
                    },
                ))));
                drop(shutdown_complete_guard);
            }
        });

        let output_stream = UnboundedReceiverStream::new(stream_rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::ConnectSwitchStream
        ))
    }

    type ReconnectSwitchStream = ResponseStream<ReconnectSwitchResponse>;
    #[tracing::instrument(target = "service")]
    async fn reconnect_switch(
        &self,
        req: Request<ReconnectSwitchRequest>,
    ) -> ServiceResult<Self::ReconnectSwitchStream> {
        // let target_addr: Address = match reconnect {
        //     ReconnectType::Auto => {
        //         let paired_switches = device.paired_switches().await?;
        //         if paired_switches.is_empty() {
        //             return Err(DeviceConnectionError::FailedToResolvePairedSwitches);
        //         }
        //         if paired_switches.len() > 1 {
        //             tracing::warn!(
        //                 "found the multiple paired switches, using the first one as a default."
        //             );
        //         }
        //         paired_switches[0].address().into()
        //     }
        //     ReconnectType::Manual(addr) => addr,
        // };
        // let paired_session = session::PairedSession::connect(session::PairedSessionConfig {
        //     reconnect_address: target_addr,
        //     ..Default::default()
        // })
        // .await?;

        unimplemented!()
    }

    #[tracing::instrument(target = "service")]
    async fn get_protocol_state(
        &self,
        req: Request<GetProtocolStateRequest>,
    ) -> ServiceResult<GetProtocolStateResponse> {
        unimplemented!()
    }

    type ButtonControlStreamStream = ResponseStream<ButtonControlStreamResponse>;
    #[tracing::instrument(target = "service")]
    async fn button_control_stream(
        &self,
        req: Request<Streaming<ButtonControlStreamRequest>>,
    ) -> ServiceResult<Self::ButtonControlStreamStream> {
        unimplemented!()
    }

    type StickControlStreamStream = ResponseStream<StickControlStreamResponse>;
    #[tracing::instrument(target = "service")]
    async fn stick_control_stream(
        &self,
        req: Request<Streaming<StickControlStreamRequest>>,
    ) -> ServiceResult<Self::StickControlStreamStream> {
        unimplemented!()
    }

    type ImuControlStreamStream = ResponseStream<ImuControlStreamResponse>;
    #[tracing::instrument(target = "service")]
    async fn imu_control_stream(
        &self,
        req: Request<Streaming<ImuControlStreamRequest>>,
    ) -> ServiceResult<Self::ImuControlStreamStream> {
        unimplemented!()
    }

    // type ConnectSwitchStream = ResponseStream<ConnectSwitchResponse>;
    // type LogStreamStream = ResponseStream<LogStreamResponse>;
    // async fn log_stream(
    //     &self,
    //     req: Request<LogStreamRequest>,
    // ) -> ServiceResult<Self::LogStreamStream> {
    //     let out_stream = BroadcastStream::new(self.tracing_json_tx.subscribe())
    //         .filter_map(|v| v.ok())
    //         .map(|val| Ok::<_, Status>(LogStreamResponse { tracing_json: val }));
    //     Ok(Response::new(Box::pin(out_stream) as Self::LogStreamStream))
    // }
}

async fn handle_connect_switch(
    device: Arc<device::Device>,
    stream_tx: mpsc::UnboundedSender<Result<ConnectSwitchResponse, Status>>,
) -> Result<(connection::Connection, connection::ConnectionHandle), NxzrServiceError> {
    let create_res = |res: connect_switch_response::Res| {
        Ok(ConnectSwitchResponse {
            res: Some(res),
            ..Default::default()
        })
    };
    let create_event = |evt: connection_event::Kind| {
        connect_switch_response::Res::Event(ConnectionEvent {
            kind: Some(evt),
            ..Default::default()
        })
    };

    // Send Event: Connecting
    let _ = stream_tx.send(create_res(create_event(connection_event::Kind::Log(
        connection_event::EventLog {
            kind: connection_event::EventLogKind::Connecting.into(),
            message: "Connecting to Switch as initial connection.".to_string(),
            ..Default::default()
        },
    ))));

    let controller_type = controller::ControllerType::ProController;
    let session_listener = connection::create_session_listener(&device).await?;
    let paired_session =
        connection::establish_initial_connection(&device, &session_listener, controller_type)
            .await?;
    let adapter_address = device.address().await?;
    let target_address = paired_session.target_address();
    let (conn, conn_handle) = connection::Connection::run(connection::ConnectionConfig {
        paired_session,
        controller_type,
    })
    .await?;

    // Listen for protocol events.
    tokio::spawn({
        let stream_tx = stream_tx.clone();
        let mut event_rx = conn.protocol().events().await?;
        async move {
            while let Some(evt) = event_rx.recv().await {
                // Log to the tracing stream as well as gRPC responses.
                tracing::info!("protocol event: {}", &evt.to_string());
                // We limit only few events to be actually sent over the
                // wire, for example, protocol's `Closed` event is
                // ignored as we need to handle it after a cleanup.
                if let Some(evt) = map_protocol_event_to_event_kind(evt) {
                    let _ = stream_tx.send(create_res(create_event(evt)));
                }
            }
        }
    });

    // Metadata
    let _ = stream_tx.send(create_res(connect_switch_response::Res::Metadata(
        ConnectionMetadata {
            adapter_address: adapter_address.to_string(),
            target_address: target_address.to_string(),
            ..Default::default()
        },
    )));

    Ok((conn, conn_handle))
}

fn map_protocol_event_to_event_kind(
    protocol_event: protocol::Event,
) -> Option<connection_event::Kind> {
    match protocol_event {
        protocol::Event::Log(log) => Some(connection_event::Kind::Log(match log {
            protocol::LogType::PairingEnded => connection_event::EventLog {
                kind: connection_event::EventLogKind::PairingEnded.into(),
                message: "Protocol has been marked as paired.".to_string(),
            },
            protocol::LogType::SubcommandReceived(subcommand) => connection_event::EventLog {
                kind: connection_event::EventLogKind::SubcommandReceived.into(),
                message: format!("Subcommand received: {}", subcommand),
            },
            _ => return None,
        })),
        protocol::Event::Error(err) => Some(connection_event::Kind::Error(ProtoError {
            message: err.to_string(),
            timestamp: Some(SystemTime::now().into()),
            ..Default::default()
        })),
        protocol::Event::Warning(warn) => Some(connection_event::Kind::Error(ProtoError {
            message: warn.to_string(),
            timestamp: Some(SystemTime::now().into()),
            ..Default::default()
        })),
    }
}
