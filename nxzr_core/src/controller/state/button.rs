use super::{StateError, StateResult};
use crate::controller::ControllerType;
use strum::Display;

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ButtonKey {
    Y,
    X,
    B,
    A,
    R,
    Zr,
    Minus,
    Plus,
    RStick,
    LStick,
    Home,
    Capture,
    Down,
    Up,
    Right,
    Left,
    Sr,
    Sl,
    L,
    Zl,
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

    pub fn is_button_set(&self, key: ButtonKey) -> bool {
        if !ButtonKey::can_use_button(self.controller, key) {
            return false;
        }
        let is_toggled = |idx: usize, bit: usize| check_bit(self.bytes[idx], bit);
        // This mapping relies on that the controller is filtered by above
        // condition before going through the routine, which means it has no
        // guarantee for that a key being checked may not be available to the
        // controller if the condition is not accurate.
        match key {
            // Byte group 1
            ButtonKey::Y => is_toggled(0, 0),
            ButtonKey::X => is_toggled(0, 1),
            ButtonKey::B => is_toggled(0, 2),
            ButtonKey::A => is_toggled(0, 3),
            ButtonKey::R => is_toggled(0, 6),
            ButtonKey::Zr => is_toggled(0, 7),
            // Byte group 2
            ButtonKey::Minus => is_toggled(1, 0),
            ButtonKey::Plus => is_toggled(1, 1),
            ButtonKey::RStick => is_toggled(1, 2),
            ButtonKey::LStick => is_toggled(1, 3),
            ButtonKey::Home => is_toggled(1, 4),
            ButtonKey::Capture => is_toggled(1, 5),
            // Byte group 3
            ButtonKey::Down => is_toggled(2, 0),
            ButtonKey::Up => is_toggled(2, 1),
            ButtonKey::Right => is_toggled(2, 2),
            ButtonKey::Left => is_toggled(2, 3),
            ButtonKey::Sr => match self.controller {
                ControllerType::JoyConR => is_toggled(0, 4),
                ControllerType::JoyConL => is_toggled(2, 4),
                _ => false,
            },
            ButtonKey::Sl => match self.controller {
                ControllerType::JoyConR => is_toggled(0, 5),
                ControllerType::JoyConL => is_toggled(2, 5),
                _ => false,
            },
            ButtonKey::L => is_toggled(2, 6),
            ButtonKey::Zl => is_toggled(2, 7),
        }
    }

    pub fn set_button(&self, key: ButtonKey, flag: Option<bool>) -> StateResult<()> {
        if !ButtonKey::can_use_button(self.controller, key) {
            return Err(StateError::ButtonNotAvailable);
        }
        let toggle_button = |idx: usize, bit: usize| match flag {
            Some(flag) => {
                self.bytes[idx] = flip_bit(self.bytes[idx], bit);
            }
            None => {
                if self.is_button_set(key) {
                    self.bytes[idx] = flip_bit(self.bytes[idx], bit);
                }
            }
        };
        match key {
            ButtonKey::Y => {}
            ButtonKey::X => {}
            ButtonKey::B => {}
            ButtonKey::A => {}
            ButtonKey::R => {}
            ButtonKey::Zr => {}
            ButtonKey::Minus => {}
            ButtonKey::Plus => {}
            ButtonKey::RStick => {}
            ButtonKey::LStick => {}
            ButtonKey::Home => {}
            ButtonKey::Capture => {}
            ButtonKey::Down => {}
            ButtonKey::Up => {}
            ButtonKey::Right => {}
            ButtonKey::Left => {}
            ButtonKey::Sr => {}
            ButtonKey::Sl => {}
            ButtonKey::L => {}
            ButtonKey::Zl => {}
        }
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

fn check_bit(value: u8, n: usize) -> bool {
    (value >> n & 1) != 0
}

fn flip_bit(value: u8, n: usize) -> u8 {
    value ^ (1 << n)
}
