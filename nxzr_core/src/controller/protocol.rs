use super::{
    delay::SendDelay,
    report::{
        input::{InputReport, InputReportId, TriggerButtonsElapsedTimeCommand},
        output::{OutputReport, OutputReportId},
        subcommand::Subcommand,
        ReportError,
    },
    spi_flash::SpiFlash,
    state::{ControllerState, StateError},
    ControllerType,
};
use crate::event::{setup_event, EventError};
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use std::{future::Future, sync::Mutex, time::Duration};
use strum::{Display, IntoStaticStr};
use thiserror::Error;
use tokio::{
    sync::{mpsc, oneshot, watch, Notify},
    time,
};

#[derive(Clone, Error, Debug)]
pub enum ControllerProtocolError {
    #[error("failed to parse output report from raw buffer, ignoring")]
    OutputReportParseFailed,
    #[error("failed to parse `id` from output report (maybe unknown?), ignoring")]
    OutputReportIdParseFailed,
    #[error("no input report mode is supplied")]
    NoInputReportModeSupplied,
    #[error("failed to create input report from given data")]
    InputReportCreationFailed,
    #[error("unknown report mode is used for generating input report")]
    UnknownInputReportMode,
    #[error("write operation is slower than usual: {0:?}, ignoring")]
    LaggedWrites(Duration),
    #[error("not implemented: {0}")]
    NotImplemented(String),
    #[error("invariant violation: {0}")]
    Invariant(String),
    #[error("internal error: {0}")]
    Internal(ControllerProtocolInternalError),
}

#[derive(Clone, Error, Debug)]
pub enum ControllerProtocolInternalError {
    #[error("io: {kind} {message}")]
    Io {
        kind: std::io::ErrorKind,
        message: String,
    },
    #[error("event: {0}")]
    Event(EventError),
    #[error("report: {0}")]
    Report(ReportError),
    #[error("state: {0}")]
    State(StateError),
}

impl From<std::io::Error> for ControllerProtocolError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(ControllerProtocolInternalError::Io {
            kind: err.kind(),
            message: err.to_string(),
        })
    }
}

impl From<EventError> for ControllerProtocolError {
    fn from(err: EventError) -> Self {
        Self::Internal(ControllerProtocolInternalError::Event(err))
    }
}

impl From<ReportError> for ControllerProtocolError {
    fn from(err: ReportError) -> Self {
        Self::Internal(ControllerProtocolInternalError::Report(err))
    }
}

impl From<StateError> for ControllerProtocolError {
    fn from(err: StateError) -> Self {
        Self::Internal(ControllerProtocolInternalError::State(err))
    }
}

#[async_trait]
pub trait TransportRead {
    async fn read(&self) -> std::io::Result<BytesMut>;
}

#[async_trait]
pub trait TransportWrite {
    async fn write(&self, buf: Bytes) -> std::io::Result<()>;
}

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
    // Internally we allow `spi_flash` to be `None`.
    // For public api, however, we don't expose these things at the moment.
    pub spi_flash: Option<SpiFlash>,
}

impl Shared {
    pub fn new(controller_state: ControllerState, spi_flash: Option<SpiFlash>) -> Self {
        Self {
            state: Mutex::new(State {
                is_pairing: false,
                send_delay: 1.0 / 15.0,
                report_mode: None,
                connected_at: None,
                controller_state,
                spi_flash,
            }),
        }
    }
    pub fn get(&self) -> State {
        self.state.lock().unwrap().clone()
    }

    pub fn modify<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
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

    pub fn modify_controller_state(&self, f: impl FnOnce(&mut ControllerState)) {
        let mut write_lock = self.state.lock().unwrap();
        f(&mut write_lock.controller_state)
    }
}

#[derive(Debug, Default)]
pub struct ControllerProtocolConfig {
    pub controller: ControllerType,
}

#[derive(Debug)]
pub struct ControllerProtocol {
    state: Shared,
    controller: ControllerType,
    notify_data_received: Notify,
    notify_writer_wake: Notify,
    writer_ready_tx: watch::Sender<bool>,
    paused_tx: watch::Sender<bool>,
    event_sub_tx: mpsc::Sender<SubscriptionReq>,
    msg_tx: mpsc::UnboundedSender<Event>,
}

impl ControllerProtocol {
    pub fn new(config: ControllerProtocolConfig) -> Result<Self, ControllerProtocolError> {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (event_sub_tx, event_sub_rx) = mpsc::channel(1);
        Event::handle_events(msg_rx, event_sub_rx)?;
        let spi_flash = SpiFlash::new();
        let controller_state = ControllerState::with_config(super::state::ControllerStateConfig {
            controller: config.controller,
            spi_flash: Some(spi_flash.clone()),
        })?;
        Ok(Self {
            state: Shared::new(controller_state, Some(spi_flash)),
            controller: config.controller,
            notify_data_received: Notify::new(),
            notify_writer_wake: Notify::new(),
            writer_ready_tx: watch::channel(false).0,
            paused_tx: watch::channel(false).0,
            event_sub_tx,
            msg_tx,
        })
    }

    // Mark a certain point when the connection is established.
    pub fn establish_connection(&self) {
        self.state.set_connected_at(Some(time::Instant::now()));
    }

    // Update the controller state by replacing the current one.
    pub async fn set_controller_state(&self, controller_state: ControllerState) {
        self.unpaused().await;
        self.state.set_controller_state(controller_state);
    }

    // Modify the controller state in-place.
    pub async fn modify_controller_state(&self, f: impl FnOnce(&mut ControllerState)) {
        self.unpaused().await;
        self.state.modify_controller_state(f);
    }

    // Resolved when the first response is received by the reader.
    pub async fn wait_for_connection(&self) {
        self.notify_data_received.notified().await;
    }

    // Send empty input reports to the host.
    pub async fn send_empty_input_report(
        &self,
        transport_write: &impl TransportWrite,
    ) -> Result<(), ControllerProtocolError> {
        self.handle_write(transport_write, InputReport::new())
            .await?;
        Ok(())
    }

    // Run reader operation using the given transport.
    pub async fn process_read<T>(&self, transport: &T) -> Result<(), ControllerProtocolError>
    where
        T: TransportRead + TransportWrite,
    {
        // FIXME: receive addr for subcommand
        self.notify_data_received.notify_waiters();
        let buf = transport.read().await?;
        let output_report = match OutputReport::with_raw(buf) {
            Ok(output_report) => output_report,
            Err(_) => {
                self.dispatch_event(Event::Error(
                    ControllerProtocolError::OutputReportParseFailed,
                ));
                return Ok(());
            }
        };
        let Some(output_report_id) = output_report.output_report_id() else {
            self.dispatch_event(Event::Error(ControllerProtocolError::OutputReportIdParseFailed));
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
                self.dispatch_event(Event::Error(
                    ControllerProtocolError::NotImplemented("attempting to request subcommand: RequestIrNfcMcu, which is not implemented, ignoring".to_owned()
                )));
            }
        }
        Ok(())
    }

    // Run writer operation using the given transport.
    pub async fn process_write(
        &self,
        transport: &impl TransportWrite,
        // NOTE: Write hook may be used to notify controller state updater loop to continue.
        write_hook: Option<impl Future<Output = ()>>,
    ) -> Result<(), ControllerProtocolError> {
        self.unpaused().await;
        let now = time::Instant::now();
        let input_report = self.generate_input_report(None)?;
        self.handle_write(transport, input_report).await?;
        if let Some(write_hook) = write_hook {
            write_hook.await;
        }
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
                    self.dispatch_event(Event::Error(ControllerProtocolError::LaggedWrites(
                        slow_duration,
                    )));
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
                self.dispatch_event(Event::Error(ControllerProtocolError::Invariant(
                    "unexpectedly setting report mode for standard input reports.".to_owned(),
                )));
            }
        }
        self.state.modify(|state| {
            state.report_mode = mode;
            if is_pairing.unwrap_or(state.is_pairing) {
                state.send_delay = 1.0 / 15.0;
            } else {
                let delay = SendDelay::new(mode).to_byte();
                match delay {
                    Some(delay) => state.send_delay = delay,
                    None => {
                        self.dispatch_event(Event::Error(ControllerProtocolError::Invariant(
                            format!(
                                "unknown delay for report mode \"{mode:?}\", assuming it as 1/15.",
                            ),
                        )));
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
        transport_write: &impl TransportWrite,
        input_report: InputReport,
    ) -> Result<(), ControllerProtocolError> {
        let mut pairing_bytes: [u8; 4] = [0; 4];
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
        transport_write
            .write(Bytes::copy_from_slice(input_report.data()))
            .await?;
        Ok(())
    }

    fn generate_input_report(
        &self,
        mode: Option<u8>,
    ) -> Result<InputReport, ControllerProtocolError> {
        let state = self.state.get();
        let mode = match mode {
            Some(_) => mode,
            None => state.report_mode,
        };
        if self.controller != state.controller_state.controller() {
            return Err(
                ControllerProtocolError::Invariant("supplied controller type in `ControllerState` does not match with one that's passed on `Protocol` init.".to_owned())
            );
        }
        let Some(mode) = mode else {
            return Err(ControllerProtocolError::NoInputReportModeSupplied);
        };
        let mut input_report = InputReport::new();
        let Some(id) = InputReportId::from_byte(mode) else {
            return Err(ControllerProtocolError::UnknownInputReportMode);
        };
        input_report.set_input_report_id(id);
        match id {
            InputReportId::Default => input_report.fill_default_report(self.controller),
            _ => {
                let timer: u64 = match state.connected_at {
                    Some(connected_at) => {
                        let elapsed = time::Instant::now() - connected_at;
                        (elapsed.as_secs_f64() / 0.005).round() as u64
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
        transport_write: &impl TransportWrite,
        output_report: &OutputReport,
    ) -> Result<(), ControllerProtocolError> {
        let Some(subcommand) = output_report.subcommand() else {
            self.dispatch_event(Event::Error(
                ControllerProtocolError::NotImplemented("unknown subcommand received.".to_owned())
            ));
            return Ok(())
        };
        if let Subcommand::Empty = subcommand {
            self.dispatch_event(Event::Error(ControllerProtocolError::Invariant(
                "received output report does not contain a subcommand".to_owned(),
            )));
        }
        self.dispatch_event(Event::Log(LogType::SubcommandReceived(subcommand)));
        let sub_command_data = output_report.subcommand_data()?;
        let mut res_input_report = self.generate_input_report(Some(0x21))?;
        match subcommand {
            Subcommand::RequestDeviceInfo => {
                self.command_request_device_info(&mut res_input_report)?;
            }
            Subcommand::SetInputReportMode => {
                self.command_set_input_report_mode(&mut res_input_report, &sub_command_data);
            }
            Subcommand::TriggerButtonsElapsedTime => {
                self.command_trigger_buttons_elapsed_time(&mut res_input_report)?;
            }
            Subcommand::SetShipmentState => {
                self.command_set_shipment_state(&mut res_input_report);
            }
            Subcommand::SpiFlashRead => {
                self.command_spi_flash_read(&mut res_input_report, &sub_command_data)?;
            }
            Subcommand::SetNfcIrMcuConfig => {
                self.command_set_nfc_ir_mcu_config(&mut res_input_report);
            }
            Subcommand::SetNfcIrMcuState => {
                self.command_set_nfc_ir_mcu_state(&mut res_input_report, &sub_command_data);
            }
            Subcommand::SetPlayerLights => {
                self.command_set_player_lights(&mut res_input_report);
            }
            Subcommand::Enable6AxisSensor => {
                self.command_enable_6axis_sensor(&mut res_input_report);
            }
            Subcommand::EnableVibration => {
                self.command_enable_vibration(&mut res_input_report);
            }
            unsupported_subcommand => {
                self.dispatch_event(Event::Error(ControllerProtocolError::NotImplemented(
                    format!("unsupported subcommand: \"{unsupported_subcommand}\", ignoring.",),
                )));
                return Ok(());
            }
        }
        self.handle_write(transport_write, res_input_report).await?;
        Ok(())
    }

    fn command_request_device_info(
        &self,
        input_report: &mut InputReport,
    ) -> Result<(), ControllerProtocolError> {
        // FIXME: receive addr: implement
        // address = self.transport.get_extra_info('sockname')
        // assert address is not None
        // bd_address = list(map(lambda x: int(x, 16), address[0].split(':')))
        input_report.set_ack(0x82);
        // FIXME: receive addr: update VVV
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
    ) -> Result<(), ControllerProtocolError> {
        input_report.set_ack(0x90);
        let mut offset: u64 = 0;
        let mut place: u64 = 1;
        for i in 0..4 {
            offset += subcommand_reply_data[i] as u64 * place;
            place *= 0x100;
        }
        let size = subcommand_reply_data[4];
        let state = self.state.get();
        match state.spi_flash {
            Some(spi_flash) => {
                let spi_flash_data =
                    &spi_flash.data()[(offset as usize)..(offset + size as u64) as usize];
                input_report.sub_0x10_spi_flash_read(offset, size, spi_flash_data)?;
            }
            None => {
                let spi_flash_data: Vec<u8> = vec![0; size as usize];
                input_report.sub_0x10_spi_flash_read(offset, size, spi_flash_data.as_ref())?;
            }
        }
        println!("spi flash read {:?}", input_report.data());
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

    fn command_trigger_buttons_elapsed_time(
        &self,
        input_report: &mut InputReport,
    ) -> Result<(), ControllerProtocolError> {
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
                self.dispatch_event(Event::Error(ControllerProtocolError::NotImplemented(
                    format!("command \"{command}\" for Subcommand NFC IR is not implemented.",),
                )));
            }
        }
    }

    fn command_set_player_lights(&self, input_report: &mut InputReport) {
        input_report.set_ack(0x80);
        input_report.set_reply_to_subcommand_id(Subcommand::SetPlayerLights);
        self.set_writer_ready();
    }

    pub async fn writer_ready(&self) {
        let mut rx = self.writer_ready_tx.subscribe();
        while !*rx.borrow() {
            rx.changed().await.unwrap();
        }
    }

    fn set_writer_ready(&self) {
        let _ = self.writer_ready_tx.send_replace(true);
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

    async fn unpaused(&self) {
        let mut rx = self.paused_tx.subscribe();
        while *rx.borrow() {
            rx.changed().await.unwrap();
        }
    }

    // Listen for the protocol events.
    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<Event>, ControllerProtocolError> {
        Ok(Event::subscribe(&mut self.event_sub_tx.clone()).await?)
    }

    fn dispatch_event(&self, event: Event) {
        let _ = self.msg_tx.send(event);
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Log(LogType),
    Error(ControllerProtocolError),
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Log(log) => write!(f, "event log: {:?}", log),
            Self::Error(err) => write!(f, "event error: {}", err.to_string()),
        }
    }
}

#[derive(Clone, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum LogType {
    PairingSuccessful,
    WriteWhilePaused,
    RepetitiveSetOfReportMode,
    SubcommandReceived(Subcommand),
}

#[derive(Debug)]
pub struct SubscriptionReq {
    tx: mpsc::UnboundedSender<Event>,
    ready_tx: oneshot::Sender<()>,
}

impl Event {
    setup_event!();
}
