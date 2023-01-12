use strum::{Display, EnumString};

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Display, EnumString)]
pub enum ControllerType {
    JoyConL,
    JoyConR,
    ProController,
}

impl Default for ControllerType {
    fn default() -> Self {
        Self::ProController
    }
}
