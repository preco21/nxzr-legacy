use strum::{Display, EnumString};

#[derive(
    Clone, Copy, Default, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Display, EnumString,
)]
pub enum ControllerType {
    JoyConL,
    JoyConR,
    #[default]
    ProController,
}
