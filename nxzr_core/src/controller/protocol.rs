use super::{
    helper::SendDelay,
    report::{
        input::{self, InputReport, InputReportId, TriggerButtonsElapsedTimeCommand},
        subcommand::Subcommand,
    },
    state::ControllerState,
    ControllerType,
};
use crate::{Error, ErrorKind, InternalErrorKind, Result};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use strum::{Display, IntoStaticStr};
use tokio::sync::{mpsc, oneshot, watch, Notify};

#[async_trait]
pub trait ProtocolTransport {
    async fn read(&self) -> std::io::Result<&[u8]>;
    async fn write(&self, buf: &[u8]) -> std::io::Result<()>;
    fn pause();
    fn resume();
}

pub struct ProtocolControl<T>
where
    T: ProtocolTransport,
{
    inner: ControllerProtocol<T>,
}

impl<T> ProtocolControl<T> where T: ProtocolTransport {}

#[derive(Debug)]
struct Shared {
    state: Mutex<State>,
}

#[derive(Debug, Clone)]
struct State {
    pub is_pairing: bool,
    pub send_delay: f64,
    pub report_mode: Option<u8>,
    pub controller_state: ControllerState,
}

impl Shared {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(State {
                is_pairing: false,
                send_delay: 1.0 / 15.0,
                report_mode: None,
                // FIXME: revisit to accept controller, spi_flash
                controller_state: ControllerState::new(),
            }),
        }
    }

    pub(crate) fn get(&self) -> State {
        self.state.lock().unwrap().clone()
    }

    pub(crate) fn replace(&self, state: &State) {
        let mut state = self.state.lock().unwrap();
        *state = state.clone();
    }

    pub fn set<R>(&self, mut f: impl FnMut(&mut State) -> R) -> R {
        let mut write_lock = self.state.lock().unwrap();
        f(&mut write_lock)
    }

    pub(crate) fn set_is_pairing(&self, flag: bool) {
        let mut state = self.state.lock().unwrap().clone();
        state.is_pairing = flag;
    }

    pub(crate) fn set_send_delay(&self, delay: f64) {
        let mut state = self.state.lock().unwrap().clone();
        state.send_delay = delay;
    }

    pub(crate) fn set_report_mode(&self, mode: Option<u8>) {
        let mut state = self.state.lock().unwrap().clone();
        state.report_mode = mode;
    }
}

pub struct ControllerProtocol<T>
where
    T: ProtocolTransport,
{
    shared: Shared,
    transport: Arc<T>,
    controller_type: ControllerType,
    notify_input_report_wake: Notify,
    notify_controller_state_send: Notify,
    paused_tx: watch::Sender<bool>,
    msg_tx: mpsc::UnboundedSender<Event>,
    event_sub_tx: mpsc::Sender<SubscriptionReq>,
}

impl<T> ControllerProtocol<T>
where
    T: ProtocolTransport,
{
    pub fn new(controller: ControllerType, transport: Arc<T>) -> Result<Self> {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (event_sub_tx, event_sub_rx) = mpsc::channel(1);
        Event::handle_events(msg_rx, event_sub_rx)?;
        Ok(Self {
            shared: Shared::new(),
            transport,
            controller_type: controller,
            notify_input_report_wake: Notify::new(),
            notify_controller_state_send: Notify::new(),
            paused_tx: watch::channel(false).0,
            msg_tx,
            event_sub_tx,
        })
    }

    pub fn set_report_mode(&self, mode: Option<u8>) {
        if let Some(mode) = mode {
            if mode == 0x21 {
                let err = Error::with_message(
                    ErrorKind::Internal(InternalErrorKind::ProtocolError),
                    "Standard input report is not meant to go through subcommand mode.".to_owned(),
                );
                self.dispatch_event(Event::Error(err));
            }
        }
        self.set_mode(mode);
        // TODO: sig input ready, start writer
        // if let Some(mode) = mode {
        //     match mode {
        //         0x30 | 0x31 | 0x32 | 0x33 => {
        //         }
        //         _ => {}
        //     }
        // }
        // FIXME: Revisit
        self.notify_input_report_wake.notify_waiters();
    }

    fn set_mode(&self, mode: Option<u8>) {
        self.shared.set(|state| {
            state.report_mode = mode;
            if state.is_pairing {
                state.send_delay = 1.0 / 15.0;
            } else {
                let delay = SendDelay::new(mode).to_byte();
                match delay {
                    Some(delay) => state.send_delay = delay,
                    None => {
                        let err = Error::with_message(
                            ErrorKind::Internal(InternalErrorKind::ProtocolError),
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
        })
    }

    pub async fn write(&mut self, input_report: InputReport) -> Result<()> {
        let mut pairing_bytes: [u8; 4] = [0x00; 4];
        pairing_bytes[1..4].copy_from_slice(&input_report.data()[4..7]);
        let close_pairing_mask = self.controller_type.close_pairing_masks();
        let state = self.shared.get();
        if state.is_pairing && (u32::from_be_bytes(pairing_bytes) & close_pairing_mask) != 0 {
            self.dispatch_event(Event::Log(LogType::PairingSuccess));
            self.shared.set_is_pairing(false);
            self.set_report_mode(state.report_mode);
        }
        if self.is_paused() {
            self.dispatch_event(Event::Log(LogType::WriteWhilePaused));
        }
        self.transport.write(input_report.data()).await?;
        self.notify_controller_state_send.notify_waiters();
        Ok(())
    }

    fn generate_input_report(&self, mode: Option<u8>) -> Result<InputReport> {
        let state = self.shared.get();
        let mode = match mode {
            Some(_) => mode,
            None => state.report_mode,
        };
        let Some(mode) = mode else {
            return Err(Error::new(ErrorKind::Internal(InternalErrorKind::InputReportCreationFailed)));
        };
        let mut input_report = InputReport::new();
        let Some(id) = InputReportId::from_byte(mode) else {
            return Err(Error::new(ErrorKind::Internal(InternalErrorKind::InputReportCreationFailed)));
        };
        input_report.set_input_report_id(id);
        match id {
            InputReportId::Default => input_report.fill_default_report(self.controller_type),
            _ => {
                // FIXME:
                //     if self._input_report_timer_start:
                //     input_report.set_timer(round((time.time() - self._input_report_timer_start) / 0.005) % 0x100)
                // else:
                //     input_report.set_timer(0)
                input_report.set_misc();
                input_report.set_button(state.controller_state.button_state().data());
                input_report.set_analog_stick(
                    Some(state.controller_state.l_stick_state().data()),
                    Some(state.controller_state.r_stick_state().data()),
                );
                input_report.set_vibrator_input();
                // NOTE: Subcommand is set outside
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

    fn run_writer_loop() {}

    fn reply_to_subcommand() {}

    fn set_connection() {}

    fn lost_connection() {}

    fn receive_report() {}

    fn send_controller_state() {}

    fn wait_for_output_report() {}

    fn controller_state() {}

    fn command_request_device_info() {}

    fn command_set_shipment_state(&self, input_report: &mut InputReport) {
        input_report.set_ack(0x80);
        input_report.set_reply_to_subcommand_id(Subcommand::SetShipmentState);
    }

    fn command_spi_flash_read() {}

    fn command_set_input_report_mode(
        &self,
        input_report: &mut InputReport,
        subcommand_reply_data: &mut [u8],
    ) {
        let state = self.shared.get();
        let command = subcommand_reply_data[0];
        if let Some(report_mode) = state.report_mode {
            if report_mode == command {
                self.dispatch_event(Event::Log(LogType::RedundantSetOfInputReportMode));
            }
        }
        self.set_report_mode(Some(command));
        input_report.set_ack(0x80);
        input_report.set_reply_to_subcommand_id(Subcommand::SetInputReportMode);
    }

    fn command_trigger_buttons_elapsed_time(&self, input_report: &mut InputReport) -> Result<()> {
        input_report.set_ack(0x83);
        input_report.set_reply_to_subcommand_id(Subcommand::TriggerButtonsElapsedTime);
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
        subcommand_reply_data: &mut [u8],
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
                    ErrorKind::Internal(InternalErrorKind::NotImplemented),
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
        // FIXME: Ping to start writer thread
        // FIXME: Send sig_input_ready channel signal
    }

    // FIXME: replace paused with transport one?
    pub fn is_paused(&self) -> bool {
        *self.paused_tx.borrow()
    }

    pub async fn paused(&self) {
        let mut rx = self.paused_tx.subscribe();
        while !*rx.borrow() {
            rx.changed().await.unwrap();
        }
    }

    pub fn pause(&self) {
        let _ = self.paused_tx.send(false);
    }

    pub fn unpause(&self) {
        let _ = self.paused_tx.send(true);
    }

    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<Event>> {
        Event::subscribe(&mut self.event_sub_tx.clone()).await
    }

    fn dispatch_event(&self, event: Event) {
        let _ = self.msg_tx.send(event);
    }
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum LogType {
    PairingSuccess,
    WriteWhilePaused,
    RedundantSetOfInputReportMode,
}

#[derive(Debug, Clone)]
pub enum Event {
    Log(LogType),
    // FIXME: Create separate error type for protocol [ProtocolErrorKind]
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
            .map_err(|_| Error::new(ErrorKind::Internal(InternalErrorKind::EventSubFailed)))?;
        ready_rx
            .await
            .map_err(|_| Error::new(ErrorKind::Internal(InternalErrorKind::EventSubFailed)))?;
        Ok(rx)
    }
}
