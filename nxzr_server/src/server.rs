use nxzr_core::{
    controller::{state::button::ButtonKey, ControllerType},
    protocol::{Protocol, ProtocolConfig},
};
use nxzr_device::{
    device::{Device, DeviceConfig},
    session::{PairedSession, PairedSessionConfig, SessionConfig, SessionListener},
    system,
    transport::{Transport, TransportConfig},
    Address,
};
use std::{future::Future, io::Write, sync::Arc};
use termion::{event::Key, input::TermRead, raw::IntoRawMode};
use tokio::{sync::mpsc, task, time};

pub async fn run(opts: ServerOpts, shutdown: impl Future) -> anyhow::Result<()> {
    let (server, server_handle) = Server::run(opts).await?;

    let p2 = server.protocol();
    let handle = task::spawn_blocking(move || {
        let stdin = std::io::stdin();
        //setting up stdout and going into raw mode
        let mut stdout = std::io::stdout().into_raw_mode().unwrap();
        //printing welcoming message, clearing the screen and going to left top corner with the cursor
        // write!(stdout, r#"{}{}ctrl + q to exit, ctrl + h to print "Hello world!", alt + t to print "termion is cool""#, termion::cursor::Goto(1, 1), termion::clear::All)
        // .unwrap();
        stdout.flush().unwrap();

        //detecting keydown events
        for c in stdin.keys() {
            //clearing the screen and going to top left corner
            // write!(
            //     stdout,
            //     "{}{}",
            //     termion::cursor::Goto(1, 1),
            //     termion::clear::All
            // )
            // .unwrap();

            let p3 = p2.clone();
            let handle_key_press = move |key: ButtonKey| {
                tokio::spawn(async move {
                    key_press(p3, key).await;
                });
            };

            //i reckon this speaks for itself
            match c.unwrap() {
                Key::Char('q') => break,
                Key::Up => {
                    handle_key_press(ButtonKey::Up);
                }
                Key::Down => handle_key_press(ButtonKey::Down),
                Key::Left => handle_key_press(ButtonKey::Left),
                Key::Right => handle_key_press(ButtonKey::Right),
                Key::Char('a') => handle_key_press(ButtonKey::A),
                Key::Char('b') => handle_key_press(ButtonKey::B),
                Key::Char('x') => handle_key_press(ButtonKey::X),
                Key::Char('y') => handle_key_press(ButtonKey::Y),
                _ => (),
            }

            // stdout.flush().unwrap();
        }
    });
    tokio::select! {
        _ = server.will_close() => {},
        _ = shutdown => {},
    }
    drop(server_handle);
    server.closed().await;
    Ok(())
}

#[derive(Debug, Default)]
pub struct ServerOpts {
    pub dev_id: Option<String>,
    pub reconnect: Option<ReconnectType>,
    pub controller_type: ControllerType,
}

#[derive(Clone, Debug)]
pub enum ReconnectType {
    Auto,
    Manual(Address),
}

pub struct ServerHandle {
    _close_rx: mpsc::Receiver<()>,
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        // Required for drop order
    }
}

#[derive(Debug)]
pub struct Server {
    protocol: Arc<Protocol>,
    transport: Transport,
    will_close_tx: mpsc::Sender<()>,
}

impl Server {
    #[tracing::instrument(target = "server")]
    async fn run(opts: ServerOpts) -> anyhow::Result<(Self, ServerHandle)> {
        system::prepare_device().await?;
        let (paired_session, address, reconnect) = match opts.reconnect {
            Some(reconnect) => {
                let (paired_session, address) =
                    create_reconnect_connection(opts.dev_id, opts.controller_type, reconnect)
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
        let protocol = Arc::new(protocol);
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

    pub fn protocol(&self) -> Arc<Protocol> {
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

#[tracing::instrument(target = "server")]
async fn establish_initial_connection(
    dev_id: Option<String>,
    controller_type: ControllerType,
) -> anyhow::Result<(PairedSession, Address)> {
    let mut device = Device::new(DeviceConfig { id: dev_id.clone() }).await?;
    device.check_paired_switches(true).await?;
    let address = device.address().await?;
    let session = SessionListener::new(SessionConfig {
        address: Some(address),
        ..Default::default()
    })?;
    // When session is failed to bind, resort to the fallback and recreate `device`.
    if let Err(err) = session.bind().await {
        tracing::warn!("{:?}", err);
        tracing::warn!("fallback: restarting Bluetooth session due to incompatibilities with the bluez `input` plugin, disable this plugin to avoid issues.");
        tracing::info!("restarting Bluetooth service...");
        system::restart_bluetooth_service()?;
        time::sleep(time::Duration::from_millis(1000)).await;
        device = Device::new(DeviceConfig { id: dev_id }).await?;
        // If it failed again, just bail out.
        session.bind().await?;
    };
    // Start listening on the session and prepare for connection.
    session.listen().await?;
    device.set_powered(true).await?;
    device.set_pairable(true).await?;
    tracing::info!("setting device alias to {}", controller_type.name());
    device.set_alias(controller_type.name()).await?;
    tracing::info!("advertising Bluetooth SDP record...");
    let record_handle = device.register_sdp_record().await?;
    device.set_discoverable(true).await?;
    tracing::info!("setting device class...");
    device.ensure_device_class().await?;
    tracing::info!("waiting for Switch to connect...");
    // Trying to accept incoming connection.
    let paired_session = session.accept().await?;
    device.set_discoverable(false).await?;
    device.set_pairable(false).await?;
    // Drop SDP-record advertisement.
    drop(record_handle);
    Ok((paired_session, address))
}

#[tracing::instrument(target = "server")]
async fn create_reconnect_connection(
    dev_id: Option<String>,
    controller_type: ControllerType,
    reconnect: ReconnectType,
) -> anyhow::Result<(PairedSession, Address)> {
    let device = Device::new(DeviceConfig { id: dev_id.clone() }).await?;
    let target_addr: Address = match reconnect {
        ReconnectType::Auto => {
            let paired_switches = device.paired_switches().await?;
            if paired_switches.is_empty() {
                return Err(anyhow::anyhow!(
                    "failed to resolve paired switches automatically."
                ));
            }
            if paired_switches.len() > 1 {
                tracing::warn!(
                    "found the multiple paired switches, using the first one as a default."
                );
            }
            paired_switches[0].address().into()
        }
        ReconnectType::Manual(addr) => addr,
    };
    let paired_session = PairedSession::connect(PairedSessionConfig {
        reconnect_address: target_addr,
        ..Default::default()
    })
    .await?;
    Ok((paired_session, target_addr))
}

async fn key_press(p: Arc<Protocol>, key: ButtonKey) {
    let _ = p
        .update_controller_state(|state| {
            state.button_state_mut().set_button(key, true).unwrap();
        })
        .await
        .unwrap();
    time::sleep(time::Duration::from_millis(100)).await;
    let _ = p
        .update_controller_state(|state| {
            state.button_state_mut().set_button(key, false).unwrap();
        })
        .await
        .unwrap();
}
