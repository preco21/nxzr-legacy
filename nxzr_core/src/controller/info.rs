use lazy_static::lazy_static;
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
    pub name: String,
}

lazy_static! {
    pub static ref CONTROLLER_INFO_MAP: HashMap<ControllerType, ControllerInfo> = {
        let mut map = HashMap::new();
        map.insert(
            ControllerType::JoyConL,
            ControllerInfo {
                id: 0x01,
                connection_info: 0x0E,
                name: "Joy-Con (L)".to_owned(),
            },
        );
        map.insert(
            ControllerType::JoyConR,
            ControllerInfo {
                id: 0x02,
                connection_info: 0x0E,
                name: "Joy-Con (R)".to_owned(),
            },
        );
        map.insert(
            ControllerType::ProController,
            ControllerInfo {
                id: 0x03,
                connection_info: 0x00,
                name: "Pro Controller".to_owned(),
            },
        );
        map
    };
}
