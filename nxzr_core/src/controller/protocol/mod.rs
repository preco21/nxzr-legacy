use self::shared::TransportShared;

use super::{
    helper::SendDelay,
    report::{
        input::{self, InputReport, InputReportId, TriggerButtonsElapsedTimeCommand},
        subcommand::Subcommand,
    },
    spi_flash::SpiFlash,
    state::ControllerState,
    ControllerType,
};
use crate::{Error, ErrorKind, InternalErrorKind, Result};
use async_trait::async_trait;
use std::{
    ops::Deref,
    sync::{Arc, Mutex, RwLock},
};
use strum::{Display, IntoStaticStr};
use tokio::sync::{mpsc, oneshot, watch, Notify};

mod shared;

#[async_trait]
pub trait ProtocolTransport {
    async fn read(&self) -> std::io::Result<&[u8]>;
    async fn write(&self, buf: &[u8]) -> std::io::Result<()>;
    fn pause(&self);
}

pub struct ProtocolTransportShared<T>
where
    T: ProtocolTransport + Send + Sync + 'static,
{
    transport: Option<T>,
}

// FIXME: ErrorKind 앞에 Protocol 때기
#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum ProtocolErrorKind {
    UnexpectedBehavior,
    ReportCreationFailed,
    NotImplemented,
    NotRegistered,
}

#[derive(Debug, Default)]
pub struct ProtocolConfig<T>
where
    T: ProtocolTransport + Send + Sync + 'static,
{
    controller: ControllerType,
    transport: Option<T>,
}

#[derive(Debug)]
pub struct Protocol<T>
where
    T: ProtocolTransport + Send + Sync + 'static,
{
    inner: Arc<ProtocolInner<T>>,
}

impl<T> Protocol<T>
where
    T: ProtocolTransport + Send + Sync + 'static,
{
    pub fn new(config: ProtocolConfig<T>) -> Result<Self> {
        // closed handle 받기?
        Ok(Self {
            inner: Arc::new(ProtocolInner::new(config)?),
        })
    }

    // 기본적으로 얘가 Err, Ok 같은거 다뤄 줘야 함 + 그럼에도, shutdown 시그널에 의해
    // spawn 여기에서 하는 것도 okz
    // handle return도 ok
    // 만약 에러 발생하면 여기서 터쳐야
    // 에러 발생했을 때는 transport pause 순서 중요하지 않음...
    // 터지면 transport pause 후 날려버려야

    // 얘는 transport만 없으면 여러번 호출될 수 있음, 근데 있으면 터짐
    pub async fn run(&self) -> Result<ProtocolHandle> {
        let (close_tx, close_rx) = mpsc::channel(1);
        // let (closed_tx, closed_rx) = mpsc::channel(1);

        let (will_close_tx, will_close_rx) = mpsc::channel(1);

        // halt -> 일단 멈춤
        // closed -> closed 상태 표기?

        // ^^^ 일단 이 상태가 휘발성 이어야 함
        // must sync with absence of transport
        // should really we store this in inner state...?

        // 애초에 루프에서 while transport.alive() 같은 느낌으로... 처리하면 될 것 같은데

        // TODO: 이 함수가 background task를 spawn도 하면서, shutdown 시그널도 받고, shutdown 할 수 있는 handle도 반환하고, shutdown 시그널을 내보낼 수도 있어야 함

        // Here we've used terminal channels for shutdown-handling because it's
        // more versatile and reliable than just polling `None` transport with
        // something like, e.g. `while let Some(t) = inner.transport() {}`.
        //
        // This is more plausible since just polling the `None` variant will not
        // break the running task handle while tasks in tokio::select! are still
        // running.
        //
        // Also, we don't directly use transport's closing signals (e.g.
        // `t.closing()`, `t.closed()`) as we need to decouple the logic in
        // different contexts and streamline shutdown signal handling.
        let mut handles = vec![];
        {
            let inner = self.inner.clone();
            handles.push(tokio::spawn(async move {
                loop {
                    tokio::select! {
                        res = inner.process_read() => {
                            // inner.close_transport() when errored
                            // this will call will_close_tx?
                        },
                        // will close? 는 여기서 처리하고 close_tx는 따로 spawn한 스레드에서 처리하는게 나을 것 같은데
                        _ = close_tx.closed() => {
                            let _ = will_close_tx.send(());
                        }
                    }
                }
            }));
        }
        // {
        //     // Handles shutdown sequence.
        //     let inner = self.inner.clone();
        //     handles.push(tokio::spawn(async move {
        //         tokio::select! {
        //             // _ = inner.transport().closed() => {

        //             // }
        //             _ = close_tx.closed() => {}

        //         }

        //         // TODO: call will close to shutdown all the handles

        //         // ㄴ No, put halt_tx in inner, call close_connection() to set it.
        //         // to streamline
        //     }));
        // }

        // ^^^ 애초에 이 spawn 로직을 new로 옮기고, started_tx로 이걸 처리할까?

        // graceful shutdown 처리
        // 1. transport.pause 처리 여기에서 해야 함 (handle 날릴때, 내부 에러 발생시 둘 다)
        // 2. 내부 에러 발생시 handle 자동으로 날리게 해야 하나 고민이네...
        // 3. 외부에다가 signal 주고 받고 하는거 너무 좀 플로우가...
        // 애초에 handle이 필요한지 모르겠어

        // check for join errors then if there's an error return Err()
        //
        Ok(ProtocolHandle {
            // will_close_rx:
            _close_rx: close_rx,
        })
    }
}

pub struct ProtocolHandle {
    _close_rx: mpsc::Receiver<()>,
}

impl Drop for ProtocolHandle {
    fn drop(&mut self) {
        // Required for drop order
    }
}

#[derive(Debug)]
pub struct ProtocolInner<T>
where
    T: ProtocolTransport + Send + Sync + 'static,
{
    shared: shared::Shared,
    // FIXME: watch + transport로 register, unregister 가능하게, service_control_handle in bluer 참조
    // 이거 optional로 받아서 register 여부 확인
    transport_shared: TransportShared<T>,
    // transport_tx: watch::Sender<Option<T>>,
    controller_type: ControllerType,
    notify_data_received: Notify,
    notify_input_report_wake: Notify,
    notify_controller_state_send: Notify,
    paused_tx: watch::Sender<bool>,
    halt_tx: watch::Sender<bool>,
    msg_tx: mpsc::UnboundedSender<Event>,
    event_sub_tx: mpsc::Sender<SubscriptionReq>,
}

impl<T> ProtocolInner<T>
where
    T: ProtocolTransport + Send + Sync + 'static,
{
    pub fn new(config: ProtocolConfig<T>) -> Result<Self> {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (event_sub_tx, event_sub_rx) = mpsc::channel(1);
        Event::handle_events(msg_rx, event_sub_rx)?;
        Ok(Self {
            shared: shared::Shared::new(),
            transport_shared: TransportShared::new(),
            // transport_tx: watch::channel(config.transport).0,
            controller_type: config.controller,
            notify_data_received: Notify::new(),
            notify_input_report_wake: Notify::new(),
            notify_controller_state_send: Notify::new(),
            paused_tx: watch::channel(false).0,
            halt_tx: watch::channel(false).0,
            msg_tx,
            event_sub_tx,
        })
    }

    // FIXME: transport를 항상 clone 할 수밖에 없는데...
    // borrow를 하더라도 await 할 수 있는 그게 가능할지...
    // Fn 같은걸 쓰면?
    // borrow는 어차피 여러 번 되어도 무방함. read/write는 freely하게 동시에 할 수 있음
    // fn transport(&self) -> Result<T> {
    //     match *self.transport_tx.borrow() {
    //         Some(transport) => Ok(transport),
    //         None => Err(Error::new(ErrorKind::Internal(
    //             InternalErrorKind::ProtocolError(ProtocolErrorKind::NotRegistered),
    //         ))),
    //     }
    // }

    // pub fn register_transport(&self, transport: Option<T>) {
    //     let _ = self.transport_tx.send_replace(transport);
    // }

    // FIXME:
    // 1. transport unregister를 기다리게 하기 -> term_tx 대체 가능
    // 2. term_tx를 따로 두기
    // unregister_transport()를 호출하는 조건 중, transport.closed()를 기다릴 이유는 없다?
    // 애초에 이런 상황이 나올 수 있는지 궁금하긴 하네.
    // transport가 날아갔는데 이게 가능해?
    // transport가 close되면 read/write도 에러 터짐 -> 자연스럽게 unregister 호출
    // 근데 이거에 의존하는 것도 이상하긴 한데...
    // 에러가 터져도, transport closed되어도, shutdown 되어도 unregister

    // FIXME: 이걸 atomic하게 수행하는 방법은 없나?
    // RwLock을 직접 구현하는게 맞을 것 같음...
    // pub fn unregister_transport(&self) -> Result<()> {
    //     match *self.transport_tx.borrow() {
    //         Some(ref transport) => transport.pause(),
    //         None => {
    //             return Err(Error::new(ErrorKind::Internal(
    //                 InternalErrorKind::ProtocolError(ProtocolErrorKind::NotRegistered),
    //             )));
    //         }
    //     }
    //     let _ = self.transport_tx.send_replace(None);
    //     Ok(())
    // }

    // pub async fn transport_unregistered(&self) -> Result<()> {
    //     let mut rx = self.transport_tx.subscribe();
    //     while !*rx.borrow() {
    //         rx.changed().await.unwrap();
    //     }
    // }

    pub fn set_report_mode(&self, mode: Option<u8>, is_pairing: Option<bool>) {
        if let Some(mode) = mode {
            if mode == 0x21 {
                let err = Error::with_message(
                    ErrorKind::Internal(InternalErrorKind::ProtocolError(
                        ProtocolErrorKind::UnexpectedBehavior,
                    )),
                    "Unexpectedly setting report mode for standard input reports.".to_owned(),
                );
                self.dispatch_event(Event::Error(err));
            }
        }
        self.set_mode(mode, is_pairing);
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

    fn set_mode(&self, mode: Option<u8>, is_pairing: Option<bool>) {
        self.shared.set(|state| {
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
                            ErrorKind::Internal(InternalErrorKind::ProtocolError(
                                ProtocolErrorKind::UnexpectedBehavior,
                            )),
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

    pub async fn write(
        &mut self,
        transport: &impl ProtocolTransport,
        input_report: InputReport,
    ) -> Result<()> {
        let mut pairing_bytes: [u8; 4] = [0x00; 4];
        pairing_bytes[1..4].copy_from_slice(&input_report.data()[4..7]);
        let close_pairing_mask = self.controller_type.close_pairing_masks();
        let state = self.shared.get();
        if state.is_pairing && (u32::from_be_bytes(pairing_bytes) & close_pairing_mask) != 0 {
            self.dispatch_event(Event::Log(LogType::PairingSuccess));
            self.set_report_mode(state.report_mode, Some(false));
        }
        if self.is_paused() {
            self.dispatch_event(Event::Log(LogType::WriteWhilePaused));
        }
        transport.write(input_report.data()).await?;
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
            return Err(Error::with_message(
                ErrorKind::Internal(InternalErrorKind::ProtocolError(ProtocolErrorKind::ReportCreationFailed)),
                "No input report mode is supplied.".to_owned()
            ));
        };
        let mut input_report = InputReport::new();
        let Some(id) = InputReportId::from_byte(mode) else {
            return Err(Error::with_message(
                ErrorKind::Internal(InternalErrorKind::ProtocolError(ProtocolErrorKind::ReportCreationFailed)),
                "Unknown report mode is used for generating input report.".to_owned()
            ));
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

    // TODO: fn receive_report() {} ->
    pub async fn process_read(&self) {}

    pub async fn process_write(&self) {}

    fn reply_to_subcommand() {}

    // protocol이 생성될 때 처리해도 됨 -> transport가 null일 수 없음
    fn set_connection() {}

    // connection이 풀린 경우는... 애초에 에러가 발생한 경우라서...
    // 흠... read/write에서 에러를 뿜으면, main에서 -> connection lost 로그 찍고 -> transport close 하도록 처리하면 될 듯
    // controller state sender는 중간에 터지도록 설계?
    fn lost_connection() {}

    fn send_controller_state() {}

    pub async fn wait_for_response(&self) {
        self.notify_data_received.notified().await;
    }

    fn command_request_device_info(&self, input_report: &mut InputReport) -> Result<()> {
        // FIXME: implement
        // address = self.transport.get_extra_info('sockname')
        // assert address is not None
        // bd_address = list(map(lambda x: int(x, 16), address[0].split(':')))
        input_report.set_ack(0x82);
        // FIXME: update
        input_report.sub_0x02_device_info([0xFFu8; 6], None, self.controller_type)?;
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
        let state = self.shared.get();
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
        subcommand_reply_data: &mut [u8],
    ) {
        let state = self.shared.get();
        let command = subcommand_reply_data[0];
        if let Some(report_mode) = state.report_mode {
            if report_mode == command {
                self.dispatch_event(Event::Log(LogType::RedundantSetOfInputReportMode));
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
                    ErrorKind::Internal(InternalErrorKind::ProtocolError(
                        ProtocolErrorKind::NotImplemented,
                    )),
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
        let _ = self.paused_tx.send_replace(false);
    }

    pub fn unpause(&self) {
        let _ = self.paused_tx.send_replace(true);
    }

    pub fn is_halted(&self) -> bool {
        *self.halt_tx.borrow()
    }

    pub async fn halted(&self) {
        let mut rx = self.halt_tx.subscribe();
        while !*rx.borrow() {
            rx.changed().await.unwrap();
        }
    }

    pub fn halt(&self) {
        let _ = self.halt_tx.send_replace(true);
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
