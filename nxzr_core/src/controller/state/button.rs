use crate::controller::ControllerType;
use strum::{Display, EnumString};

use super::StateError;

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, EnumString)]
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
#[derive(Clone, Debug, Default)]
pub struct ButtonState {
    // TODO: Refactor ButtonState later on such that no controller type is
    // retained in the state and split into granular ButtonState per controller
    // type.
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

    pub fn with_controller(controller: ControllerType) -> Self {
        Self {
            controller,
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
                _ => unreachable!(),
            },
            ButtonKey::Sl => match self.controller {
                ControllerType::JoyConR => is_toggled(0, 5),
                ControllerType::JoyConL => is_toggled(2, 5),
                _ => unreachable!(),
            },
            ButtonKey::L => is_toggled(2, 6),
            ButtonKey::Zl => is_toggled(2, 7),
        }
    }

    pub fn toggle_button(&mut self, key: ButtonKey) -> Result<(), StateError> {
        self.set_button(key, !self.is_button_set(key))
    }

    pub fn set_button(&mut self, key: ButtonKey, flag: bool) -> Result<(), StateError> {
        if !ButtonKey::can_use_button(self.controller, key) {
            return Err(StateError::ButtonNotAvailable);
        }
        let mut toggle = |idx: usize, bit: usize| {
            if flag != check_bit(self.bytes[idx], bit) {
                self.bytes[idx] = flip_bit(self.bytes[idx], bit);
            }
        };
        // This mapping relies on that the controller is filtered by above
        // condition before going through the routine, which means it has no
        // guarantee for that a key being checked may not be available to the
        // controller if the condition is not accurate.
        match key {
            // Byte group 1
            ButtonKey::Y => toggle(0, 0),
            ButtonKey::X => toggle(0, 1),
            ButtonKey::B => toggle(0, 2),
            ButtonKey::A => toggle(0, 3),
            ButtonKey::R => toggle(0, 6),
            ButtonKey::Zr => toggle(0, 7),
            // Byte group 2
            ButtonKey::Minus => toggle(1, 0),
            ButtonKey::Plus => toggle(1, 1),
            ButtonKey::RStick => toggle(1, 2),
            ButtonKey::LStick => toggle(1, 3),
            ButtonKey::Home => toggle(1, 4),
            ButtonKey::Capture => toggle(1, 5),
            // Byte group 3
            ButtonKey::Down => toggle(2, 0),
            ButtonKey::Up => toggle(2, 1),
            ButtonKey::Right => toggle(2, 2),
            ButtonKey::Left => toggle(2, 3),
            ButtonKey::Sr => match self.controller {
                ControllerType::JoyConR => toggle(0, 4),
                ControllerType::JoyConL => toggle(2, 4),
                _ => unreachable!(),
            },
            ButtonKey::Sl => match self.controller {
                ControllerType::JoyConR => toggle(0, 5),
                ControllerType::JoyConL => toggle(2, 5),
                _ => unreachable!(),
            },
            ButtonKey::L => toggle(2, 6),
            ButtonKey::Zl => toggle(2, 7),
        }
        Ok(())
    }

    pub fn controller(&self) -> ControllerType {
        self.controller
    }

    pub fn available_buttons(&self) -> &'static [ButtonKey] {
        ButtonKey::available_buttons(self.controller)
    }

    pub fn clear(&mut self) {
        for byte in &mut self.bytes {
            *byte = 0;
        }
    }

    pub fn as_bytes(&self) -> &[u8; 3] {
        &self.bytes
    }
}

fn check_bit(value: u8, n: usize) -> bool {
    (value >> n & 1) != 0
}

fn flip_bit(value: u8, n: usize) -> u8 {
    value ^ (1 << n)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::ButtonKey;

    #[test]
    fn convert_string_to_button_key() {
        let button = ButtonKey::from_str("s");
        assert!(button.is_err(), "invalid string should fail conversion");
        let button = ButtonKey::from_str("Capture").unwrap();
        assert_eq!(button, ButtonKey::Capture);
    }

    #[test]
    fn test_convert_button_key_to_string() {
        let button = ButtonKey::Capture;
        println!("{}", button);
        assert_eq!(button.to_string(), "Capture");
    }
}
