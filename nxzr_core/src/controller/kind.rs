use once_cell::sync::Lazy;
use std::collections::HashMap;
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

#[derive(Debug)]
pub struct ControllerInfo {
    pub id: u8,
    pub connection_info: u8,
}

pub static CONTROLLER_INFO_MAP: Lazy<HashMap<ControllerType, ControllerInfo>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(
        ControllerType::JoyConL,
        ControllerInfo {
            id: 0x01,
            connection_info: 0x0E,
        },
    );
    map.insert(
        ControllerType::JoyConR,
        ControllerInfo {
            id: 0x02,
            connection_info: 0x0E,
        },
    );
    map.insert(
        ControllerType::ProController,
        ControllerInfo {
            id: 0x03,
            connection_info: 0x00,
        },
    );
    map
});
