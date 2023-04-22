use thiserror::Error;

#[derive(Clone, Error, Debug)]
pub enum EventError {
    #[error("failed to subscribe events")]
    SubscriptionFailed,
}

macro_rules! setup_event {
    () => {
        pub fn handle_events(
            mut msg_rx: mpsc::UnboundedReceiver<Event>,
            mut sub_rx: mpsc::Receiver<SubscriptionReq>,
        ) -> std::result::Result<(), crate::event::EventError> {
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
        ) -> std::result::Result<mpsc::UnboundedReceiver<Event>, crate::event::EventError> {
            let (tx, rx) = mpsc::unbounded_channel();
            let (ready_tx, ready_rx) = oneshot::channel();
            sub_tx
                .send(SubscriptionReq { tx, ready_tx })
                .await
                .map_err(|_| crate::event::EventError::SubscriptionFailed)?;
            ready_rx
                .await
                .map_err(|_| crate::event::EventError::SubscriptionFailed)?;
            Ok(rx)
        }
    };
}

pub(crate) use setup_event;
