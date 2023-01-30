use super::StateResult;
use crate::controller::ControllerType;

pub enum ButtonKey {
    Y,
    X,
    B,
    A,
    L,
    Zl,
    Sl,
    R,
    Zr,
    Sr,
    Minus,
    Plus,
    LStick,
    RStick,
    Home,
    Capture,
    Up,
    Down,
    Left,
    Right,
}

impl ButtonKey {
    pub fn available_buttons(controller: ControllerType) -> &'static [Self] {
        match controller {
            ControllerType::ProController => &[
                Self::Y,
                Self::X,
                Self::B,
                Self::A,
                Self::R,
                Self::Zr,
                Self::Minus,
                Self::Plus,
                Self::RStick,
                Self::LStick,
                Self::Home,
                Self::Capture,
                Self::Down,
                Self::Up,
                Self::Right,
                Self::Left,
                Self::L,
                Self::Zl,
            ],
            ControllerType::JoyConR => &[
                Self::Y,
                Self::X,
                Self::B,
                Self::A,
                Self::Sr,
                Self::Sl,
                Self::R,
                Self::Zr,
                Self::Plus,
                Self::RStick,
                Self::Home,
            ],
            ControllerType::JoyConL => &[
                Self::Minus,
                Self::LStick,
                Self::Capture,
                Self::Down,
                Self::Up,
                Self::Right,
                Self::Left,
                Self::Sr,
                Self::Sl,
                Self::L,
                Self::Zl,
            ],
            _ => panic!("Unable to get available buttons for unknown controller type."),
        }
    }

    pub fn can_use_button(controller: ControllerType, key: Self) -> bool {
        match controller {
            ControllerType::ProController => match key {
                Self::Y
                | Self::X
                | Self::B
                | Self::A
                | Self::R
                | Self::Zr
                | Self::Minus
                | Self::Plus
                | Self::RStick
                | Self::LStick
                | Self::Home
                | Self::Capture
                | Self::Down
                | Self::Up
                | Self::Right
                | Self::Left
                | Self::L
                | Self::Zl => true,
                _ => false,
            },
            ControllerType::JoyConR => match key {
                Self::Y
                | Self::X
                | Self::B
                | Self::A
                | Self::Sr
                | Self::Sl
                | Self::R
                | Self::Zr
                | Self::Plus
                | Self::RStick
                | Self::Home => true,
                _ => false,
            },
            ControllerType::JoyConL => match key {
                Self::Minus
                | Self::LStick
                | Self::Capture
                | Self::Down
                | Self::Up
                | Self::Right
                | Self::Left
                | Self::Sr
                | Self::Sl
                | Self::L
                | Self::Zl => true,
                _ => false,
            },
            _ => false,
        }
    }
}

/**
 * Utility struct to set buttons in the input report:
 * https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md
 * Byte 	0 	    1 	    2 	    3 	    4 	    5 	    6 	    7
 * 1   	    Y 	    X 	    B 	    A 	    SR 	    SL 	    R 	    ZR
 * 2        Minus 	Plus 	R Stick L Stick Home 	Capture
 * 3        Down 	Up 	    Right 	Left 	SR 	    SL 	    L 	    ZL
 */
#[derive(Clone, Debug)]
pub struct ButtonState {
    controller: ControllerType,
    bytes: [u8; 3],
}

impl ButtonState {
    pub fn new() -> Self {
        Self {
            controller: ControllerType::ProController,
            bytes: [0, 0, 0],
        }
    }

    pub fn is_button_set(&self, key: ButtonKey) {
        // logic
    }

    pub fn set_button(&self, key: ButtonKey) -> StateResult<()> {
        if !ButtonKey::can_use_button(self.controller, key) {
            return Err(super::StateError::ButtonNotAvailable);
        }
        // logic
        Ok(())
    }

    pub fn available_buttons(&self) -> &'static [ButtonKey] {
        ButtonKey::available_buttons(self.controller)
    }

    pub fn clear(&mut self) {
        for byte in &mut self.bytes {
            *byte = 0;
        }
    }

    pub fn data(&self) -> &[u8; 3] {
        &self.bytes
    }
}

fn bit_on(value: u8, n: usize) -> bool {
    (value >> n & 1) != 0
}

fn flip_bit(value: u8, n: usize) -> u8 {
    value ^ (1 << n)
}
