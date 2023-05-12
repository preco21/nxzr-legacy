use nxzr_core::{
    controller::{state::button::ButtonKey, ControllerType},
    protocol::{Event, Protocol, ProtocolConfig},
};
use nxzr_device::{
    device::{Device, DeviceConfig},
    helper,
    session::{SessionConfig, SessionListener},
    syscheck,
    transport::{Transport, TransportConfig},
};
use std::{io::Write, sync::Arc};
use termion::{event::Key, input::TermRead, raw::IntoRawMode};
use tokio::{signal, sync::mpsc, task, time};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    syscheck::check_system_requirements().await?;

    let (shutdown_tx, _shutdown_rx) = mpsc::channel::<()>(1);

    // FIXME: accept device id here
    let mut device = Device::new(DeviceConfig::default())
        // .await?
        // .ensure_adapter_address_switch()
        .await?;

    // FIXME: implement reconnect, currently connecting from scratch only supported
    device.check_paired_devices(true).await?;

    let address = device.address().await?;
    let session = SessionListener::new(SessionConfig {
        address: Some(nxzr_device::sock::Address::new(address.into())),
        ..Default::default()
    })?;

    if let Err(err) = session.bind().await {
        tracing::warn!("{:?}", err);
        tracing::warn!("fallback: restarting Bluetooth session due to incompatibilities with the bluez `input` plugin, disable this plugin to avoid issues.");
        tracing::info!("restarting Bluetooth service...");
        helper::restart_bluetooth_service()?;
        time::sleep(time::Duration::from_millis(1000)).await;
        // FIXME: accept device id here
        device = Device::new(DeviceConfig::default()).await?;
        session.bind().await?;
    };

    session.listen().await?;

    device.set_powered(true).await?;
    device.set_pairable(true).await?;

    // FIXME: make it customizable
    tracing::info!(
        "setting device alias to {}",
        ControllerType::ProController.name()
    );
    device
        .set_alias(ControllerType::ProController.name())
        .await?;

    tracing::info!("advertising Bluetooth SDP record...");

    // FIXME: allow ignoring errors
    let record = device.register_sdp_record().await?;

    device.set_discoverable(true).await?;
    device.ensure_device_class().await?;

    tracing::info!("waiting for Switch to connect...");

    let paired_session = session.accept().await?;
    device.set_discoverable(false).await?;
    device.set_pairable(false).await?;

    let (transport, transport_handle) =
        Transport::register(paired_session, TransportConfig::default()).await?;
    // FIXME: allow customizing config
    let (protocol, protocol_handle) = Protocol::connect(
        transport.clone(),
        ProtocolConfig {
            dev_address: nxzr_core::addr::Address::new(address.into()),
            ..Default::default()
        },
    )?;

    let mut event_rx = protocol.events().await?;

    // FIXME:
    tokio::spawn(async move {
        while let Some(evt) = event_rx.recv().await {
            tracing::warn!("{:?} {}", &evt, &evt.to_string());
            // match evt {
            //     Event::Log(log) => {
            //         tracing::info!("{:?} {}", &log, &log.to_string());
            //     }
            //     E => {}
            // }
        }
    });

    let p = Arc::new(protocol);
    let p2 = p.clone();
    // let handle = task::spawn_blocking(move || {
    //     let stdin = std::io::stdin();
    //     //setting up stdout and going into raw mode
    //     let mut stdout = std::io::stdout().into_raw_mode().unwrap();
    //     //printing welcoming message, clearing the screen and going to left top corner with the cursor
    //     // write!(stdout, r#"{}{}ctrl + q to exit, ctrl + h to print "Hello world!", alt + t to print "termion is cool""#, termion::cursor::Goto(1, 1), termion::clear::All)
    //     // .unwrap();
    //     stdout.flush().unwrap();

    //     //detecting keydown events
    //     for c in stdin.keys() {
    //         //clearing the screen and going to top left corner
    //         // write!(
    //         //     stdout,
    //         //     "{}{}",
    //         //     termion::cursor::Goto(1, 1),
    //         //     termion::clear::All
    //         // )
    //         // .unwrap();

    //         let p3 = p2.clone();
    //         let handle_key_press = move |key: ButtonKey| {
    //             tokio::spawn(async move {
    //                 println!("spawned for {}", key);
    //                 key_press(p3, key).await;
    //             });
    //         };

    //         //i reckon this speaks for itself
    //         match c.unwrap() {
    //             Key::Char('q') => break,
    //             Key::Up => {
    //                 println!("up");
    //                 handle_key_press(ButtonKey::Up);
    //             }
    //             Key::Down => handle_key_press(ButtonKey::Down),
    //             Key::Left => handle_key_press(ButtonKey::Left),
    //             Key::Right => handle_key_press(ButtonKey::Right),
    //             Key::Char('a') => handle_key_press(ButtonKey::A),
    //             Key::Char('b') => handle_key_press(ButtonKey::B),
    //             Key::Char('x') => handle_key_press(ButtonKey::X),
    //             Key::Char('y') => handle_key_press(ButtonKey::Y),
    //             _ => (),
    //         }

    //         // stdout.flush().unwrap();
    //     }
    // });

    tokio::select! {
        _ = signal::ctrl_c() => {},
        _ = p.closed() => {},
        _ = transport.closed() => {},
        _ = shutdown_tx.closed() => {},
    }

    drop(record);
    drop(protocol_handle);
    drop(transport_handle);

    p.closed().await;
    transport.closed().await;

    // handle.abort();

    Ok(())
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
