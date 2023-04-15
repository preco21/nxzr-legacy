use super::ProtocolTransport;
use crate::controller::{spi_flash::SpiFlash, state::ControllerState};
use std::sync::{Mutex, RwLock};

#[derive(Debug)]
pub(crate) struct Shared {
    state: Mutex<State>,
}

#[derive(Debug, Clone)]
pub(crate) struct State {
    pub is_pairing: bool,
    pub send_delay: f64,
    pub report_mode: Option<u8>,
    pub controller_state: ControllerState,
    pub spi_flash: Option<SpiFlash>,
}

impl Shared {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                is_pairing: false,
                send_delay: 1.0 / 15.0,
                report_mode: None,
                // FIXME: revisit to accept controller, spi_flash
                controller_state: ControllerState::new(),
                spi_flash: None,
            }),
        }
    }
    pub fn get(&self) -> State {
        self.state.lock().unwrap().clone()
    }

    pub fn replace(&self, state: &State) {
        let mut state = self.state.lock().unwrap();
        *state = state.clone();
    }

    pub fn set<R>(&self, mut f: impl FnMut(&mut State) -> R) -> R {
        let mut write_lock = self.state.lock().unwrap();
        f(&mut write_lock)
    }

    pub fn set_is_pairing(&self, flag: bool) {
        let mut state = self.state.lock().unwrap().clone();
        state.is_pairing = flag;
    }

    pub fn set_send_delay(&self, delay: f64) {
        let mut state = self.state.lock().unwrap().clone();
        state.send_delay = delay;
    }

    pub fn set_report_mode(&self, mode: Option<u8>) {
        let mut state = self.state.lock().unwrap().clone();
        state.report_mode = mode;
    }
}

#[derive(Debug)]
pub(crate) struct TransportShared<T>(RwLock<Option<T>>)
where
    T: ProtocolTransport + Send + Sync;

impl<T> TransportShared<T>
where
    T: ProtocolTransport + Send + Sync,
{
    pub fn new() -> Self {
        Self(RwLock::new(None))
    }
    // pub fn transport()
}
