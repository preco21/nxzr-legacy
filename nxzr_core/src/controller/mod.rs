use state::button::ButtonKey;
use strum::{Display, EnumString};

pub mod interval;
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
}

impl ControllerType {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0x01 => Some(Self::JoyConL),
            0x02 => Some(Self::JoyConR),
            0x03 => Some(Self::ProController),
            _ => None,
        }
    }

    pub fn id(&self) -> u8 {
        match self {
            Self::JoyConL => 0x01,
            Self::JoyConR => 0x02,
            Self::ProController => 0x03,
        }
    }

    pub fn name(&self) -> String {
        match self {
            Self::JoyConL => "Joy-Con (L)".into(),
            Self::JoyConR => "Joy-Con (R)".into(),
            Self::ProController => "Pro Controller".into(),
        }
    }

    pub fn connection_info(&self) -> u8 {
        match self {
            Self::ProController => 0x00,
            _ => 0x0E,
        }
    }

    pub fn device_info_color(&self) -> u8 {
        match self {
            Self::ProController => 0x02,
            _ => 0x01,
        }
    }

    pub fn close_pairing_masks(&self) -> u32 {
        match self {
            Self::JoyConL => u32::from_be_bytes([0x00, 0x02 | 0x08, 0x10, 0x00]),
            Self::JoyConR => u32::from_be_bytes([0x00, 0x00, 0x00, 0x01 | 0x08]),
            Self::ProController => u32::from_be_bytes([0x00, 0x04 | 0x08, 0x10, 0x00]),
        }
    }

    pub fn close_pairing_menu_seq(&self) -> &[ButtonKey] {
        match self {
            Self::JoyConL => &[ButtonKey::X, ButtonKey::A, ButtonKey::Home],
            Self::JoyConR => &[ButtonKey::Down, ButtonKey::Left],
            Self::ProController => &[ButtonKey::A, ButtonKey::B, ButtonKey::Home],
        }
    }
}
