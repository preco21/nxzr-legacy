use super::{
    interval::SendInterval,
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
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use nxzr_shared::{
    addr::Address,
    event::{setup_event, EventError, SubscriptionReq},
};
use std::{future::Future, sync::Mutex};
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
    LaggedWrites(time::Duration),
    #[error("write operation is triggered while paused, ignoring")]
    WriteWhilePaused,
    #[error("a report mode has been set, which is identical to previous one")]
    DuplicatedReportModeSet,
    #[error("transport error: {message}")]
    Transport {
        kind: std::io::ErrorKind,
        message: String,
    },
    #[error("not implemented: {0}")]
    NotImplemented(String),
    #[error("invariant violation: {0}")]
    Invariant(String),
    #[error("internal error: {0}")]
    Internal(ControllerProtocolInternalError),
}

#[derive(Clone, Error, Debug)]
pub enum ControllerProtocolInternalError {
    #[error("report: {0}")]
    Report(ReportError),
    #[error("state: {0}")]
    State(StateError),
    #[error("event: {0}")]
    Event(EventError),
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

impl From<EventError> for ControllerProtocolError {
    fn from(err: EventError) -> Self {
        Self::Internal(ControllerProtocolInternalError::Event(err))
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
    pub send_interval: f64,
    pub report_mode: Option<u8>,
    pub connected_at: Option<time::Instant>,
    pub controller_state: ControllerState,
    // Internally we allow `spi_flash` to be `None`.
    // For public api, however, we don't expose these things at the moment.
    pub spi_flash: Option<SpiFlash>,
}

impl Shared {
    pub fn new(
        controller_state: ControllerState,
        spi_flash: Option<SpiFlash>,
        reconnect: bool,
    ) -> Self {
        Self {
            state: Mutex::new(State {
                is_pairing: !reconnect,
                send_interval: if reconnect {
                    SendInterval::new(None).to_byte().unwrap()
                } else {
                    SendInterval::default_byte()
                },
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
        let mut state = self.state.lock().unwrap();
        state.connected_at = connected_at;
    }

    pub fn set_controller_state(&self, controller_state: ControllerState) {
        let mut state = self.state.lock().unwrap();
        state.controller_state = controller_state;
    }

    pub fn modify_controller_state(&self, f: impl FnOnce(&mut ControllerState)) {
        let mut write_lock = self.state.lock().unwrap();
        f(&mut write_lock.controller_state)
    }
}

#[derive(Debug, Default)]
pub struct ControllerProtocolConfig {
    pub controller_type: ControllerType,
    pub dev_address: Address,
    pub reconnect: bool,
}

#[derive(Debug)]
pub struct ControllerProtocol {
    state: Shared,
    controller_type: ControllerType,
    dev_addr: Address,
    notify_data_received: Notify,
    notify_writer_wake: Notify,
    writer_ready_tx: watch::Sender<bool>,
    paused_tx: watch::Sender<bool>,
    event_sub_tx: mpsc::Sender<SubscriptionReq<Event>>,
    msg_tx: mpsc::Sender<Event>,
}

impl ControllerProtocol {
    pub fn new(config: ControllerProtocolConfig) -> Result<Self, ControllerProtocolError> {
        let (msg_tx, msg_rx) = mpsc::channel(256);
        let (event_sub_tx, event_sub_rx) = mpsc::channel(1);
        Event::handle_events(msg_rx, event_sub_rx)?;
        let spi_flash = SpiFlash::new();
        let controller_state = ControllerState::with_config(super::state::ControllerStateConfig {
            controller: config.controller_type,
            spi_flash: Some(spi_flash.clone()),
        })?;
        Ok(Self {
            state: Shared::new(controller_state, Some(spi_flash), config.reconnect),
            controller_type: config.controller_type,
            dev_addr: config.dev_address,
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

    // Send blank input reports to the host.
    pub async fn send_blank_input_report(
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
        self.notify_data_received.notify_waiters();
        let buf = transport
            .read()
            .await
            .map_err(|err| ControllerProtocolError::Transport {
                kind: err.kind(),
                message: err.to_string(),
            })?;
        let output_report = match OutputReport::with_raw(buf) {
            Ok(output_report) => output_report,
            Err(_) => {
                self.emit_event(Event::Warning(
                    ControllerProtocolError::OutputReportParseFailed,
                ));
                // Continues silently after error logging.
                return Ok(());
            }
        };
        let Some(output_report_id) = output_report.output_report_id() else {
            self.emit_event(Event::Warning(ControllerProtocolError::OutputReportIdParseFailed));
            return Ok(());
        };
        match output_report_id {
            OutputReportId::SubCommand => {
                self.reply_to_subcommand(transport, &output_report).await?;
            }
            OutputReportId::RumbleOnly => {
                // Rumble: noop
            }
            OutputReportId::RequestIrNfcMcu => {
                self.emit_event(Event::Warning(
                    ControllerProtocolError::NotImplemented("attempting to request subcommand: RequestIrNfcMcu, which is not implemented, ignoring.".into()
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
        if state.send_interval == f64::INFINITY {
            self.notify_writer_wake.notified().await
        } else {
            let send_interval = time::Duration::from_secs_f64(state.send_interval);
            let elapsed = now.elapsed();
            let shim_delay = match send_interval.checked_sub(elapsed) {
                Some(delay) => delay,
                None => {
                    let slow_duration = elapsed - send_interval;
                    self.emit_event(Event::Warning(ControllerProtocolError::LaggedWrites(
                        slow_duration,
                    )));
                    return Ok(());
                }
            };
            let _ = time::timeout(shim_delay, self.notify_writer_wake.notified()).await;
        }
        Ok(())
    }

    fn set_report_mode(&self, mode: Option<u8>) {
        match mode {
            Some(0x21) => {
                self.emit_event(Event::Warning(ControllerProtocolError::Invariant(
                    "unexpectedly setting report mode for standard input reports.".into(),
                )));
            }
            _ => {}
        }
        let send_interval = self.state.modify(|state| {
            let mode = match mode {
                Some(_) => mode,
                None => state.report_mode,
            };
            state.report_mode = mode;
            // In pairing mode, you must write reports at 15hz pace; exceeding
            // this limit will result in disconnection from the host.
            //
            // After exiting the pairing mode, this routine will be called again
            // with `is_pairing` set to `false`, and be setting appropriate
            // send interval for the current report mode.
            if state.is_pairing {
                Some(SendInterval::default_byte())
            } else {
                None
            }
        });
        self.set_send_interval(send_interval);
        // TODO: Revisit: Should send set_writer_ready() and start writer thread?
        // if let Some(mode) = mode {
        //     match mode {
        //         0x30 | 0x31 | 0x32 | 0x33 => {}
        //         _ => {}
        //     }
        // }
        self.notify_writer_wake.notify_waiters();
    }

    fn set_send_interval(&self, interval: Option<f64>) {
        self.state.modify(|state| match interval {
            Some(interval) => {
                state.send_interval = interval;
            }
            // If `None` is specified, try extracting it from the report mode.
            None => {
                let interval = SendInterval::new(state.report_mode).to_byte();
                match interval {
                    Some(interval) => state.send_interval = interval,
                    None => {
                        self.emit_event(Event::Warning(ControllerProtocolError::Invariant(
                            format!(
                                "unknown interval for report mode \"{:?}\", assuming it as 15hz.",
                                state.report_mode
                            ),
                        )));
                        state.send_interval = SendInterval::default_byte();
                    }
                };
            }
        })
    }

    async fn handle_write(
        &self,
        transport_write: &impl TransportWrite,
        input_report: InputReport,
    ) -> Result<(), ControllerProtocolError> {
        // FIXME: this is fragile, need to revisit after testing if the vibration subcommand method works properly.
        let mut pairing_bytes: [u8; 4] = [0; 4];
        pairing_bytes[1..4].copy_from_slice(&input_report.as_buf()[4..7]);
        let close_pairing_mask = self.controller_type.close_pairing_masks();
        let is_pairing_ended = self.state.modify(|state| {
            if state.is_pairing && (u32::from_be_bytes(pairing_bytes) & close_pairing_mask) != 0 {
                state.is_pairing = false;
                true
            } else {
                false
            }
        });
        if is_pairing_ended {
            self.set_report_mode(None);
            self.emit_event(Event::Log(LogType::PairingEnded));
        }
        if self.is_paused() {
            self.emit_event(Event::Warning(ControllerProtocolError::WriteWhilePaused));
        }
        transport_write
            .write(Bytes::copy_from_slice(input_report.as_buf()))
            .await
            .map_err(|err| ControllerProtocolError::Transport {
                kind: err.kind(),
                message: err.to_string(),
            })?;
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
        if self.controller_type != state.controller_state.controller() {
            return Err(
                ControllerProtocolError::Invariant("supplied controller type in `ControllerState` does not match with one that's passed on `Protocol` init.".into())
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
            InputReportId::Default => input_report.fill_default_report(self.controller_type),
            _ => {
                let timer: u64 = match state.connected_at {
                    Some(connected_at) => {
                        let elapsed = connected_at.elapsed();
                        (elapsed.as_secs_f64() / 0.005).round() as u64
                    }
                    None => 0,
                };
                input_report.set_timer(timer);
                input_report.set_misc();
                input_report.set_button(state.controller_state.button_state().as_bytes());
                input_report.set_analog_stick(
                    Some(state.controller_state.l_stick_state().to_buf()),
                    Some(state.controller_state.r_stick_state().to_buf()),
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
        let subcommand = match output_report.subcommand() {
            Ok(subcommand) => subcommand,
            Err(err) => {
                self.emit_event(Event::Warning(ControllerProtocolError::from(err)));
                // Silently continues the process after error logging.
                return Ok(());
            }
        };
        self.emit_event(Event::Log(LogType::SubcommandReceived(subcommand)));
        let sub_command_data = output_report.subcommand_data()?;
        let mut res_input_report = self.generate_input_report(Some(0x21))?;
        match subcommand {
            Subcommand::RequestDeviceInfo => {
                self.command_request_device_info(&mut res_input_report)?;
            }
            Subcommand::SetInputReportMode => {
                self.command_set_input_report_mode(&mut res_input_report, &sub_command_data)?;
            }
            Subcommand::TriggerButtonsElapsedTime => {
                self.command_trigger_buttons_elapsed_time(&mut res_input_report)?;
            }
            Subcommand::SetShipmentState => {
                self.command_set_shipment_state(&mut res_input_report)?;
            }
            Subcommand::SpiFlashRead => {
                self.command_spi_flash_read(&mut res_input_report, &sub_command_data)?;
            }
            Subcommand::SetNfcIrMcuConfig => {
                self.command_set_nfc_ir_mcu_config(&mut res_input_report)?;
            }
            Subcommand::SetNfcIrMcuState => {
                self.command_set_nfc_ir_mcu_state(&mut res_input_report, &sub_command_data)?;
            }
            Subcommand::SetPlayerLights => {
                self.command_set_player_lights(&mut res_input_report)?;
            }
            Subcommand::Enable6AxisSensor => {
                self.command_enable_6axis_sensor(&mut res_input_report)?;
            }
            Subcommand::EnableVibration => {
                self.command_enable_vibration(&mut res_input_report, &sub_command_data)?;
            }
            unsupported_subcommand => {
                self.emit_event(Event::Warning(ControllerProtocolError::NotImplemented(
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
        input_report.set_ack(0x82);
        input_report.sub_0x02_device_info(*self.dev_addr, None, self.controller_type)?;
        Ok(())
    }

    fn command_set_shipment_state(
        &self,
        input_report: &mut InputReport,
    ) -> Result<(), ControllerProtocolError> {
        input_report.set_ack(0x80);
        input_report.set_response_subcommand(Subcommand::SetShipmentState)?;
        Ok(())
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
                let spi_flash_data = &spi_flash[(offset as usize)..(offset + size as u64) as usize];
                input_report.sub_0x10_spi_flash_read(offset, size, spi_flash_data)?;
            }
            None => {
                let zeroed_spi_flash_data: Vec<u8> = vec![0; size as usize];
                input_report.sub_0x10_spi_flash_read(offset, size, &zeroed_spi_flash_data)?;
            }
        }
        Ok(())
    }

    fn command_set_input_report_mode(
        &self,
        input_report: &mut InputReport,
        subcommand_reply_data: &[u8],
    ) -> Result<(), ControllerProtocolError> {
        let state = self.state.get();
        let mode = subcommand_reply_data[0];
        if let Some(report_mode) = state.report_mode {
            if report_mode == mode {
                self.emit_event(Event::Warning(
                    ControllerProtocolError::DuplicatedReportModeSet,
                ));
            }
        }
        self.set_report_mode(Some(mode));
        input_report.set_ack(0x80);
        input_report.set_response_subcommand(Subcommand::SetInputReportMode)?;
        Ok(())
    }

    fn command_trigger_buttons_elapsed_time(
        &self,
        input_report: &mut InputReport,
    ) -> Result<(), ControllerProtocolError> {
        input_report.set_ack(0x83);
        input_report.set_response_subcommand(Subcommand::TriggerButtonsElapsedTime)?;
        // HACK: We assume this command is only used during pairing, sets values
        // and let the Switch to assign a player number.
        match self.controller_type {
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

    fn command_enable_6axis_sensor(
        &self,
        input_report: &mut InputReport,
    ) -> Result<(), ControllerProtocolError> {
        input_report.set_ack(0x80);
        input_report.set_response_subcommand(Subcommand::Enable6AxisSensor)?;
        Ok(())
    }

    fn command_enable_vibration(
        &self,
        input_report: &mut InputReport,
        subcommand_reply_data: &[u8],
    ) -> Result<(), ControllerProtocolError> {
        let command = subcommand_reply_data[0];
        match command {
            // If it's 0x01, then we shell slow down the send frequency.
            0x01 => self.set_send_interval(Some(SendInterval::default_byte())),
            // Otherwise, we can release it to match with the pace of `report_mode`.
            //
            // Also, we toggle `is_pairing` flag to `true` if not toggled previously.
            _ => {
                // FIXME: still fragile...
                // let pairing_toggled = self.state.modify(|state| {
                //     if state.is_pairing {
                //         state.is_pairing = false;
                //         true
                //     } else {
                //         false
                //     }
                // });
                // if pairing_toggled {
                //     self.set_report_mode(None);
                //     self.emit_event(Event::Log(LogType::PairingSuccess));
                // } else {
                self.set_send_interval(None)
                // }
            }
        }
        input_report.set_ack(0x80);
        input_report.set_response_subcommand(Subcommand::EnableVibration)?;
        Ok(())
    }

    fn command_set_nfc_ir_mcu_config(
        &self,
        input_report: &mut InputReport,
    ) -> Result<(), ControllerProtocolError> {
        input_report.set_ack(0xA0);
        input_report.set_response_subcommand(Subcommand::SetNfcIrMcuConfig)?;
        input_report.as_mut()[16..50].copy_from_slice(&[
            0x01, 0x00, 0xFF, 0x00, 0x08, 0x00, 0x1B, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0xC8,
        ]);
        Ok(())
    }

    fn command_set_nfc_ir_mcu_state(
        &self,
        input_report: &mut InputReport,
        subcommand_reply_data: &[u8],
    ) -> Result<(), ControllerProtocolError> {
        let command = subcommand_reply_data[0];
        match command {
            // Resume + Suspend
            0x01 | 0x00 => {
                input_report.set_ack(0x80);
                input_report.set_response_subcommand(Subcommand::SetNfcIrMcuState)?;
            }
            _ => {
                self.emit_event(Event::Warning(ControllerProtocolError::NotImplemented(
                    format!("command \"{command}\" for Subcommand NFC IR is not implemented.",),
                )));
            }
        }
        Ok(())
    }

    fn command_set_player_lights(
        &self,
        input_report: &mut InputReport,
    ) -> Result<(), ControllerProtocolError> {
        input_report.set_ack(0x80);
        input_report.set_response_subcommand(Subcommand::SetPlayerLights)?;
        self.set_writer_ready();
        Ok(())
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

    fn emit_event(&self, event: Event) {
        let _ = self.msg_tx.try_send(event);
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Log(LogType),
    Warning(ControllerProtocolError),
}

#[derive(Clone, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum LogType {
    PairingEnded,
    SubcommandReceived(Subcommand),
}

impl Event {
    setup_event!(Event);
}
