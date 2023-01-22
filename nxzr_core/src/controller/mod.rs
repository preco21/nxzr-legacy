use strum::{Display, EnumString};

pub mod kind;
pub mod protocol;
pub mod report;
pub mod spi_flash;
pub mod state;

#[derive(
    Clone, Copy, Default, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Display, EnumString,
)]
pub enum ControllerType {
    JoyConL,
    JoyConR,
    #[default]
    ProController,
    Unknown,
}

impl ControllerType {
    pub fn from_id(id: u8) -> Self {
        match id {
            0x01 => Self::JoyConL,
            0x02 => Self::JoyConR,
            0x03 => Self::ProController,
            _ => Self::Unknown,
        }
    }

    pub fn id(&self) -> u8 {
        match self {
            Self::JoyConL => 0x01,
            Self::JoyConR => 0x02,
            Self::ProController => 0x03,
            _ => panic!("Unknown controller type cannot refer its id."),
        }
    }

    pub fn try_id(&self) -> Option<u8> {
        if let Self::Unknown = self {
            return None;
        }
        Some(self.id())
    }

    pub fn name(&self) -> String {
        match self {
            Self::JoyConL => "Joy-Con (L)".to_owned(),
            Self::JoyConR => "Joy-Con (R)".to_owned(),
            Self::ProController => "Pro Controller".to_owned(),
            _ => panic!("Unknown controller type cannot refer its name"),
        }
    }

    pub fn try_name(&self) -> Option<String> {
        if let Self::Unknown = self {
            return None;
        }
        Some(self.name())
    }

    pub fn connection_info(&self) -> u8 {
        match self {
            Self::JoyConL => 0x0E,
            Self::JoyConR => 0x0E,
            Self::ProController => 0x00,
            _ => panic!("Unknown controller type cannot refer its connection info."),
        }
    }

    pub fn try_connection_info(&self) -> Option<u8> {
        if let Self::Unknown = self {
            return None;
        }
        Some(self.connection_info())
    }
}
