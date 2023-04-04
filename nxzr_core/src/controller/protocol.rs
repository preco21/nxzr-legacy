use crate::{Error, ErrorKind, InternalErrorKind, Result};
use tokio::sync::{mpsc, oneshot, Notify};

pub struct ControllerProtocol {
    is_pairing: bool,
    send_delay: f64,
    report_mode: Option<u8>,
    sig_input_ready: Notify,
    sig_input_report_wakeup: Notify,
    msg_tx: mpsc::UnboundedSender<Event>,
    event_sub_tx: mpsc::Sender<SubscriptionReq>,
}

impl ControllerProtocol {
    pub fn new() -> Result<Self> {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (event_sub_tx, event_sub_rx) = mpsc::channel(1);
        Event::handle_events(msg_rx, event_sub_rx)?;
        Ok(Self {
            send_delay: 1.0 / 15.0,
            // FIXME: revisit
            report_mode: None,
            is_pairing: false,
            sig_input_ready: Notify::new(),
            sig_input_report_wakeup: Notify::new(),
            event_sub_tx,
            msg_tx,
        })
    }

    fn set_report_mode(&mut self, mode: u8) {
        if mode == 0x21 {
            let err = Error::new(ErrorKind::Internal(InternalErrorKind::ProtocolError));
            err.message =
                "Standard input report is not meant to go through subcommand mode.".to_owned();
            self.msg_tx.send(Event::Error(err));
        }
        self.input_report_mode = input_report_id;
        if self.is_pairing {
            self.send_delay = 1.0 / 15.0;
        } else {
            let delay = input_report_id.send_delay();
            self.send_delay = delay;
        }
    }

    fn write() {}
    fn generate_input_report() {}
    fn run_writer_loop() {}
    fn reply_to_subcommand() {}

    fn set_connection() {}
    fn lost_connection() {}
    fn receive_report() {}

    fn send_controller_state() {}
    fn wait_for_output_report() {}
    fn pause() {}
    fn unpause() {}
    fn controller_state() {}

    fn command_request_device_info() {}
    fn command_set_shipment_state() {}
    fn command_spi_flash_read() {}
    fn command_set_input_report_mode() {}
    fn command_trigger_buttons_elapsed_time() {}
    fn command_enable_6axis_sensor() {}
    fn command_enable_vibration() {}
    fn command_set_nfc_ir_mcu_config() {}
    fn command_set_nfc_ir_mcu_state() {}
    fn command_set_player_lights() {}

    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<Event>> {
        Event::subscribe(&mut self.event_sub_tx.clone()).await
    }
}

#[derive(Debug, Clone)]
pub enum Event {
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
