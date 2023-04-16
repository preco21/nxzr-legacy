use super::{
    helper::SendDelay,
    report::{
        input::{InputReport, InputReportId, TriggerButtonsElapsedTimeCommand},
        output::{OutputReport, OutputReportId},
        subcommand::Subcommand,
    },
    spi_flash::SpiFlash,
    state::ControllerState,
    ControllerType,
};
use crate::{Error, ErrorKind, InternalErrorKind, Result};
use async_trait::async_trait;
use std::{future::Future, sync::Mutex, time::Duration};
use strum::{Display, IntoStaticStr};
use tokio::{
    sync::{mpsc, oneshot, watch, Notify},
    time,
};

// mod control;

#[async_trait]
pub trait TransportRead {
    async fn read(&self) -> std::io::Result<&[u8]>;
}
#[async_trait]
pub trait TransportWrite {
    async fn write(&self, buf: &[u8]) -> std::io::Result<()>;
}
pub trait TransportCombined: TransportRead + TransportWrite {}

#[derive(Debug)]
struct Shared {
    state: Mutex<State>,
}

#[derive(Debug, Clone)]
struct State {
    pub is_pairing: bool,
    pub send_delay: f64,
    pub report_mode: Option<u8>,
    pub connected_at: Option<time::Instant>,
    pub controller_state: ControllerState,
    pub spi_flash: Option<SpiFlash>,
}

impl Shared {
    pub fn new(controller_state: ControllerState) -> Self {
        Self {
            state: Mutex::new(State {
                is_pairing: false,
                send_delay: 1.0 / 15.0,
                report_mode: None,
                connected_at: None,
                controller_state,
                spi_flash: None,
            }),
        }
    }
    pub fn get(&self) -> State {
        self.state.lock().unwrap().clone()
    }

    pub fn modify<R>(&self, mut f: impl FnMut(&mut State) -> R) -> R {
        let mut write_lock = self.state.lock().unwrap();
        f(&mut write_lock)
    }

    pub fn set_connected_at(&self, connected_at: Option<time::Instant>) {
        let mut state = self.state.lock().unwrap().clone();
        state.connected_at = connected_at;
    }

    pub fn set_controller_state(&self, controller_state: ControllerState) {
        let mut state = self.state.lock().unwrap().clone();
        state.controller_state = controller_state;
    }

    pub fn modify_controller_state(&self, mut f: impl FnMut(&mut ControllerState)) {
        let mut write_lock = self.state.lock().unwrap();
        f(&mut write_lock.controller_state)
    }
}

#[derive(Debug, Default)]
pub struct ProtocolConfig {
    controller: ControllerType,
    controller_state: ControllerState,
}

#[derive(Debug)]
pub struct Protocol {
    state: Shared,
    controller: ControllerType,
    notify_data_received: Notify,
    notify_writer_wake: Notify,
    ready_for_write_tx: watch::Sender<bool>,
    paused_tx: watch::Sender<bool>,
    event_sub_tx: mpsc::Sender<SubscriptionReq>,
    msg_tx: mpsc::UnboundedSender<Event>,
}

impl Protocol {
    pub fn new(config: ProtocolConfig) -> Result<Self> {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (event_sub_tx, event_sub_rx) = mpsc::channel(1);
        Event::handle_events(msg_rx, event_sub_rx)?;
        Ok(Self {
            state: Shared::new(config.controller_state),
            controller: config.controller,
            notify_data_received: Notify::new(),
            notify_writer_wake: Notify::new(),
            ready_for_write_tx: watch::channel(false).0,
            paused_tx: watch::channel(false).0,
            event_sub_tx,
            msg_tx,
        })
    }

    // Marks a certain point when the connection is established.
    pub fn establish_connection(&self) {
        self.state.set_connected_at(Some(time::Instant::now()));
    }

    // Updates the controller state by replacing the current one.
    pub async fn set_controller_state(&self, controller_state: ControllerState) {
        self.wait_for_continue().await;
        self.state.set_controller_state(controller_state);
    }

    // Modifies the controller state in-place.
    pub async fn modify_controller_state(&self, f: impl FnMut(&mut ControllerState)) {
        self.wait_for_continue().await;
        self.state.modify_controller_state(f);
    }

    // Resolved when the first response is received by the reader.
    pub async fn wait_for_response(&self) {
        self.notify_data_received.notified().await;
    }

    // Runs reader operation using the given transport.
    pub async fn process_read(&self, transport: &impl TransportCombined) -> Result<()> {
        // FIXME: receive addr
        self.notify_data_received.notify_waiters();
        let buf = transport.read().await?;
        let output_report = match OutputReport::with_raw(buf) {
            Ok(output_report) => output_report,
            Err(err) => {
                let err = Error::with_message(
                    ErrorKind::ProtocolOutputReportParsingFailed,
                    "Failed to parse output report, ignoring.".to_owned(),
                );
                self.dispatch_event(Event::Error(err));
                return Ok(());
            }
        };
        let Some(output_report_id) = output_report.output_report_id() else {
            let err = Error::with_message(
                ErrorKind::ProtocolOutputReportParsingFailed,
                "Failed to parse `id` from output report (maybe unknown?), ignoring.".to_owned(),
            );
            self.dispatch_event(Event::Error(err));
            return Ok(());
        };
        match output_report_id {
            OutputReportId::SubCommand => {
                self.reply_to_subcommand(transport, &output_report).await?;
            }
            OutputReportId::RumbleOnly => {
                // noop: Rumble
            }
            OutputReportId::RequestIrNfcMcu => {
                let err = Error::with_message(
                    ErrorKind::NotImplemented,
                    "Attempting to request subcommand: RequestIrNfcMcu, which is not implemented, ignoring.".to_owned(),
                );
                self.dispatch_event(Event::Error(err));
            }
        }
        Ok(())
    }

    // Runs writer operation using the given transport.
    pub async fn process_write(
        &self,
        transport: &impl TransportWrite,
        // NOTE: Write hook may be used to notify controller state updater loop to continue.
        write_hook: impl Future<Output = ()>,
    ) -> Result<()> {
        self.wait_for_continue().await;
        let now = time::Instant::now();
        let input_report = self.generate_input_report(None)?;
        self.handle_write(transport, &input_report).await?;
        write_hook.await;
        let state = self.state.get();
        if state.send_delay == f64::INFINITY {
            self.notify_writer_wake.notified().await
        } else {
            let send_delay = Duration::from_secs_f64(state.send_delay);
            let elapsed = time::Instant::now() - now;
            let next_delay = match send_delay.checked_sub(elapsed) {
                Some(delay) => delay,
                None => {
                    let slow_duration = elapsed - send_delay;
                    let err = Error::with_message(
                ErrorKind::ProtocolWriteTooSlow(slow_duration),
                format!(
                    "Write operation is taking longer than usual to complete: {:?}, skipping the write.",
                    slow_duration
                ),
            );
                    self.dispatch_event(Event::Error(err));
                    return Ok(());
                }
            };
            let _ = time::timeout(next_delay, self.notify_writer_wake.notified()).await;
        }
        Ok(())
    }

    fn set_report_mode(&self, mode: Option<u8>, is_pairing: Option<bool>) {
        if let Some(mode) = mode {
            if mode == 0x21 {
                let err = Error::with_message(
                    ErrorKind::Invariant,
                    "Unexpectedly setting report mode for standard input reports.".to_owned(),
                );
                self.dispatch_event(Event::Error(err));
            }
        }
        self.state.modify(|state| {
            state.report_mode = mode;
            let is_pairing = match is_pairing {
                Some(flag) => flag,
                None => state.is_pairing,
            };
            if is_pairing {
                state.send_delay = 1.0 / 15.0;
            } else {
                let delay = SendDelay::new(mode).to_byte();
                match delay {
                    Some(delay) => state.send_delay = delay,
                    None => {
                        let err = Error::with_message(
                            ErrorKind::Invariant,
                            format!(
                                "Unknown delay for report mode {:?}, assuming it as 1/15.",
                                mode
                            ),
                        );
                        self.dispatch_event(Event::Error(err));
                        state.send_delay = 1.0 / 15.0;
                    }
                };
            }
        });
        // TODO: Revisit: Should send set_ready_for_write() and start writer thread?
        // if let Some(mode) = mode {
        //     match mode {
        //         0x30 | 0x31 | 0x32 | 0x33 => {}
        //         _ => {}
        //     }
        // }
        self.notify_writer_wake.notify_waiters();
    }

    async fn handle_write(
        &self,
        transport_w: &impl TransportWrite,
        input_report: &InputReport,
    ) -> Result<()> {
        let mut pairing_bytes: [u8; 4] = [0x00; 4];
        pairing_bytes[1..4].copy_from_slice(&input_report.data()[4..7]);
        let close_pairing_mask = self.controller.close_pairing_masks();
        let state = self.state.get();
        if state.is_pairing && (u32::from_be_bytes(pairing_bytes) & close_pairing_mask) != 0 {
            self.dispatch_event(Event::Log(LogType::PairingSuccessful));
            self.set_report_mode(state.report_mode, Some(false));
        }
        if self.is_paused() {
            self.dispatch_event(Event::Log(LogType::WriteWhilePaused));
        }
        transport_w.write(input_report.data()).await?;
        Ok(())
    }

    fn generate_input_report(&self, mode: Option<u8>) -> Result<InputReport> {
        let state = self.state.get();
        let mode = match mode {
            Some(_) => mode,
            None => state.report_mode,
        };
        if self.controller != state.controller_state.controller() {
            return Err(Error::with_message(
                ErrorKind::Invariant,
                "Supplied controller type in ControllerState does not match with one passed on Protocol init."
                    .to_owned(),
            ));
        }
        let Some(mode) = mode else {
            return Err(Error::with_message(
                ErrorKind::ProtocolInputReportCreationFailed,
                "No input report mode is supplied.".to_owned()
            ));
        };
        let mut input_report = InputReport::new();
        let Some(id) = InputReportId::from_byte(mode) else {
            return Err(Error::with_message(
                ErrorKind::ProtocolInputReportCreationFailed,
                "Unknown report mode is used for generating input report.".to_owned()
            ));
        };
        input_report.set_input_report_id(id);
        match id {
            InputReportId::Default => input_report.fill_default_report(self.controller),
            _ => {
                let timer: u64 = match state.connected_at {
                    Some(connected_at) => {
                        let cal = time::Instant::now() - connected_at;
                        (cal.as_secs_f64() / 0.005).round() as u64
                    }
                    None => 0,
                };
                input_report.set_timer(timer);
                input_report.set_misc();
                input_report.set_button(state.controller_state.button_state().data());
                input_report.set_analog_stick(
                    Some(state.controller_state.l_stick_state().data()),
                    Some(state.controller_state.r_stick_state().data()),
                );
                input_report.set_vibrator_input();
                // NOTE: Subcommand is set from caller
                match id {
                    InputReportId::NfcIrMcu => {
                        input_report.set_6axis_data();
                        // INFO: Sets empty data for now.
                        input_report.set_ir_nfc_data(&[0xFF; 313])?;
                    }
                    InputReportId::Imu | InputReportId::Unknown1 | InputReportId::Unknown2 => {
                        input_report.set_6axis_data();
                    }
                    _ => {}
                }
            }
        };
        Ok(input_report)
    }

    async fn reply_to_subcommand(
        &self,
        transport_w: &impl TransportWrite,
        output_report: &OutputReport,
    ) -> Result<()> {
        let Some(subcommand) = output_report.subcommand() else {
            let err = Error::with_message(
                ErrorKind::NotImplemented,
                "Unknown subcommand received.".to_owned()
            );
            self.dispatch_event(Event::Error(err));
            return Ok(())
        };
        if let Subcommand::Empty = subcommand {
            let err = Error::with_message(
                ErrorKind::Invariant,
                "Received output report does not contain a subcommand".to_owned(),
            );
            self.dispatch_event(Event::Error(err));
        }
        self.dispatch_event(Event::Log(LogType::SubcommandReceived(subcommand)));
        let sub_command_data = output_report.subcommand_data()?;
        let mut response_report = self.generate_input_report(Some(0x21))?;
        match subcommand {
            Subcommand::RequestDeviceInfo => {
                self.command_request_device_info(&mut response_report)?;
            }
            Subcommand::SetInputReportMode => {
                self.command_set_input_report_mode(&mut response_report, &sub_command_data);
            }
            Subcommand::TriggerButtonsElapsedTime => {
                self.command_trigger_buttons_elapsed_time(&mut response_report)?;
            }
            Subcommand::SetShipmentState => {
                self.command_set_shipment_state(&mut response_report);
            }
            Subcommand::SpiFlashRead => {
                self.command_spi_flash_read(&mut response_report, &sub_command_data)?;
            }
            Subcommand::SetNfcIrMcuConfig => {
                self.command_set_nfc_ir_mcu_config(&mut response_report);
            }
            Subcommand::SetNfcIrMcuState => {
                self.command_set_nfc_ir_mcu_state(&mut response_report, &sub_command_data);
            }
            Subcommand::SetPlayerLights => {
                self.command_set_player_lights(&mut response_report);
            }
            Subcommand::Enable6AxisSensor => {
                self.command_enable_6axis_sensor(&mut response_report);
            }
            Subcommand::EnableVibration => {
                self.command_enable_vibration(&mut response_report);
            }
            unsupported_subcommand => {
                let err = Error::with_message(
                    ErrorKind::NotImplemented,
                    format!(
                        "Unsupported subcommand: {}, ignoring.",
                        unsupported_subcommand
                    ),
                );
                self.dispatch_event(Event::Error(err));
                return Ok(());
            }
        }
        self.handle_write(transport_w, &response_report).await?;
        Ok(())
    }

    fn command_request_device_info(&self, input_report: &mut InputReport) -> Result<()> {
        // FIXME: implement
        // address = self.transport.get_extra_info('sockname')
        // assert address is not None
        // bd_address = list(map(lambda x: int(x, 16), address[0].split(':')))
        input_report.set_ack(0x82);
        // FIXME: update VVV
        input_report.sub_0x02_device_info([0xFFu8; 6], None, self.controller)?;
        Ok(())
    }

    fn command_set_shipment_state(&self, input_report: &mut InputReport) {
        input_report.set_ack(0x80);
        input_report.set_reply_to_subcommand_id(Subcommand::SetShipmentState);
    }

    fn command_spi_flash_read(
        &self,
        input_report: &mut InputReport,
        subcommand_reply_data: &[u8],
    ) -> Result<()> {
        input_report.set_ack(0x90);
        let mut offset: u32 = 0;
        let mut place: u32 = 1;
        for i in 0..4 {
            offset += subcommand_reply_data[i] as u32 * place;
            place *= 0x100;
        }
        let size = subcommand_reply_data[4];
        let state = self.state.get();
        match state.spi_flash {
            Some(spi_flash) => {
                let spi_flash_data =
                    &spi_flash.data()[(offset as usize)..(offset + size as u32) as usize];
                input_report.sub_0x10_spi_flash_read(offset, size, spi_flash_data)?;
            }
            None => {
                let spi_flash_data = vec![0x00; size as usize];
                input_report.sub_0x10_spi_flash_read(offset, size, spi_flash_data.as_ref())?;
            }
        }
        Ok(())
    }

    fn command_set_input_report_mode(
        &self,
        input_report: &mut InputReport,
        subcommand_reply_data: &[u8],
    ) {
        let state = self.state.get();
        let command = subcommand_reply_data[0];
        if let Some(report_mode) = state.report_mode {
            if report_mode == command {
                self.dispatch_event(Event::Log(LogType::RepetitiveSetOfReportMode));
            }
        }
        self.set_report_mode(Some(command), None);
        input_report.set_ack(0x80);
        input_report.set_reply_to_subcommand_id(Subcommand::SetInputReportMode);
    }

    fn command_trigger_buttons_elapsed_time(&self, input_report: &mut InputReport) -> Result<()> {
        input_report.set_ack(0x83);
        input_report.set_reply_to_subcommand_id(Subcommand::TriggerButtonsElapsedTime);
        // HACK: We assume this command is only used during pairing, sets values
        // and let the Switch to assign a player number.
        match self.controller {
            // INFO: Currently we don't support a combined JoyCon.
            ControllerType::JoyConL | ControllerType::JoyConR => input_report
                .sub_0x04_trigger_buttons_elapsed_time(&[
                    TriggerButtonsElapsedTimeCommand::SLeftTrigger(3000),
                    TriggerButtonsElapsedTimeCommand::SRightTrigger(3000),
                ])?,
            ControllerType::ProController => {
                input_report.sub_0x04_trigger_buttons_elapsed_time(&[
                    TriggerButtonsElapsedTimeCommand::LeftTrigger(3000),
                    TriggerButtonsElapsedTimeCommand::RightTrigger(3000),
                ])?
            }
        }
        Ok(())
    }

    fn command_enable_6axis_sensor(&self, input_report: &mut InputReport) {
        input_report.set_ack(0x80);
        input_report.set_reply_to_subcommand_id(Subcommand::Enable6AxisSensor);
    }

    fn command_enable_vibration(&self, input_report: &mut InputReport) {
        input_report.set_ack(0x80);
        input_report.set_reply_to_subcommand_id(Subcommand::EnableVibration);
    }

    fn command_set_nfc_ir_mcu_config(&self, input_report: &mut InputReport) {
        input_report.set_ack(0xA0);
        input_report.set_reply_to_subcommand_id(Subcommand::SetNfcIrMcuConfig);
        input_report.as_mut()[16..50].copy_from_slice(&[
            0x01, 0x00, 0xFF, 0x00, 0x08, 0x00, 0x1B, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0xC8,
        ]);
    }

    fn command_set_nfc_ir_mcu_state(
        &self,
        input_report: &mut InputReport,
        subcommand_reply_data: &[u8],
    ) {
        let command = subcommand_reply_data[0];
        match command {
            // Resume + Suspend
            0x01 | 0x00 => {
                input_report.set_ack(0x80);
                input_report.set_reply_to_subcommand_id(Subcommand::SetNfcIrMcuState);
            }
            _ => {
                let err = Error::with_message(
                    ErrorKind::NotImplemented,
                    format!(
                        "Command {} for Subcommand NFC IR is not implemented.",
                        command
                    ),
                );
                self.dispatch_event(Event::Error(err));
            }
        }
    }

    fn command_set_player_lights(&self, input_report: &mut InputReport) {
        input_report.set_ack(0x80);
        input_report.set_reply_to_subcommand_id(Subcommand::SetPlayerLights);
        self.set_ready_for_write();
    }

    pub async fn ready_for_write(&self) {
        let mut rx = self.ready_for_write_tx.subscribe();
        while !*rx.borrow() {
            rx.changed().await.unwrap();
        }
    }

    // Mark the protocol in paused state.
    pub fn pause(&self) {
        let _ = self.paused_tx.send_replace(false);
    }

    // Mark the protocol in unpaused state.
    pub fn unpause(&self) {
        let _ = self.paused_tx.send_replace(true);
    }

    fn is_paused(&self) -> bool {
        *self.paused_tx.borrow()
    }

    async fn wait_for_continue(&self) {
        let mut rx = self.paused_tx.subscribe();
        while *rx.borrow() {
            rx.changed().await.unwrap();
        }
    }

    fn set_ready_for_write(&self) {
        let _ = self.ready_for_write_tx.send_replace(true);
    }

    // Listen for the protocol events.
    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<Event>> {
        Event::subscribe(&mut self.event_sub_tx.clone()).await
    }

    fn dispatch_event(&self, event: Event) {
        let _ = self.msg_tx.send(event);
    }
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum LogType {
    PairingSuccessful,
    WriteWhilePaused,
    RepetitiveSetOfReportMode,
    SubcommandReceived(Subcommand),
}

#[derive(Debug, Clone)]
pub enum Event {
    Log(LogType),
    Error(Error),
}

#[derive(Debug)]
pub struct SubscriptionReq {
    tx: mpsc::UnboundedSender<Event>,
    ready_tx: oneshot::Sender<()>,
}

impl Event {
    pub fn handle_events(
        mut msg_rx: mpsc::UnboundedReceiver<Event>,
        mut sub_rx: mpsc::Receiver<SubscriptionReq>,
    ) -> Result<()> {
        tokio::spawn(async move {
            struct Subscription {
                tx: mpsc::UnboundedSender<Event>,
            }
            let mut subs: Vec<Subscription> = vec![];
            loop {
                tokio::select! {
                    msg = msg_rx.recv(), if subs.len() > 0 => {
                        match msg {
                            Some(evt) => {
                                subs.retain(|sub| sub.tx.send(evt.clone()).is_ok());
                            }
                            None => break,
                        }
                    },
                    sub_opts = sub_rx.recv() => {
                        match sub_opts {
                            Some(SubscriptionReq { tx, ready_tx }) => {
                                let _ = ready_tx.send(());
                                subs.push(Subscription { tx });
                            }
                            None => break,
                        };
                    },
                }
            }
        });
        Ok(())
    }

    pub async fn subscribe(
        sub_tx: &mut mpsc::Sender<SubscriptionReq>,
    ) -> Result<mpsc::UnboundedReceiver<Event>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = oneshot::channel();
        sub_tx
            .send(SubscriptionReq { tx, ready_tx })
            .await
            .map_err(|_| {
                Error::new(ErrorKind::Internal(
                    InternalErrorKind::EventSubscriptionFailed,
                ))
            })?;
        ready_rx.await.map_err(|_| {
            Error::new(ErrorKind::Internal(
                InternalErrorKind::EventSubscriptionFailed,
            ))
        })?;
        Ok(rx)
    }
}
