use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

#[derive(Clone, Error, Debug)]
pub enum EventError {
    #[error("failed to subscribe events")]
    SubscriptionFailed,
}

pub struct SubscriptionReq<T> {
    tx: mpsc::UnboundedSender<T>,
    ready_tx: oneshot::Sender<()>,
}

#[async_trait::async_trait]
pub trait Event {
    type EventItem: Clone + Send + Sync + 'static;

    fn handle_events(
        mut msg_rx: mpsc::Receiver<Self::EventItem>,
        mut sub_rx: mpsc::Receiver<SubscriptionReq<Self::EventItem>>,
    ) -> Result<(), EventError> {
        tokio::spawn(async move {
            struct Subscription<T> {
                tx: mpsc::UnboundedSender<T>,
            }
            let mut subs: Vec<Subscription<Self::EventItem>> = vec![];
            loop {
                tokio::select! {
                    msg = msg_rx.recv(), if !subs.is_empty() => {
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

    async fn subscribe(
        sub_tx: &mut mpsc::Sender<SubscriptionReq<Self::EventItem>>,
    ) -> std::result::Result<mpsc::UnboundedReceiver<Self::EventItem>, EventError> {
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
}

#[macro_export]
macro_rules! setup_event {
    ($event_type:ident) => {
        pub fn handle_events(
            mut msg_rx: mpsc::Receiver<$event_type>,
            mut sub_rx: mpsc::Receiver<nxzr_shared::event::SubscriptionReq<$event_type>>,
        ) -> std::result::Result<(), nxzr_shared::event::EventError> {
            tokio::spawn(async move {
                struct Subscription {
                    tx: mpsc::UnboundedSender<$event_type>,
                }
                let mut subs: Vec<Subscription> = vec![];
                loop {
                    tokio::select! {
                        msg = msg_rx.recv(), if !subs.is_empty() => {
                            match msg {
                                Some(evt) => {
                                    subs.retain(|sub| sub.tx.send(evt.clone()).is_ok());
                                }
                                None => break,
                            }
                        },
                        sub_opts = sub_rx.recv() => {
                            match sub_opts {
                                Some(nxzr_shared::event::SubscriptionReq { tx, ready_tx }) => {
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
            sub_tx: &mut mpsc::Sender<nxzr_shared::event::SubscriptionReq<$event_type>>,
        ) -> std::result::Result<mpsc::UnboundedReceiver<$event_type>, nxzr_shared::event::EventError> {
            let (tx, rx) = mpsc::unbounded_channel();
            let (ready_tx, ready_rx) = oneshot::channel();
            sub_tx
                .send(nxzr_shared::event::SubscriptionReq { tx, ready_tx })
                .await
                .map_err(|_| nxzr_shared::event::EventError::SubscriptionFailed)?;
            ready_rx
                .await
                .map_err(|_| nxzr_shared::event::EventError::SubscriptionFailed)?;
            Ok(rx)
        }
    };
}

pub use setup_event;
