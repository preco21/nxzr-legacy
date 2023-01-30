use crate::controller::ControllerType;

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

    pub fn is_button_set(&self) {}

    pub fn set_button(&self) {}

    pub fn available_buttons(&self) -> &'static [ButtonKey] {
        ButtonKey::available_buttons(self.controller)
    }

    pub fn clear(&self) {}

    pub fn data() {}
}

fn bit_on(value: u8, n: usize) -> bool {
    (value >> n & 1) != 0
}

fn flip_bit(value: u8, n: usize) -> u8 {
    value ^ (1 << n)
}

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
    pub fn available_buttons(controller: ControllerType) -> &'static [ButtonKey] {
        match controller {
            ControllerType::ProController => &[
                ButtonKey::Y,
                ButtonKey::X,
                ButtonKey::B,
                ButtonKey::A,
                ButtonKey::R,
                ButtonKey::Zr,
                ButtonKey::Minus,
                ButtonKey::Plus,
                ButtonKey::RStick,
                ButtonKey::LStick,
                ButtonKey::Home,
                ButtonKey::Capture,
                ButtonKey::Down,
                ButtonKey::Up,
                ButtonKey::Right,
                ButtonKey::Left,
                ButtonKey::L,
                ButtonKey::Zl,
            ],
            ControllerType::JoyConR => &[
                ButtonKey::Y,
                ButtonKey::X,
                ButtonKey::B,
                ButtonKey::A,
                ButtonKey::Sr,
                ButtonKey::Sl,
                ButtonKey::R,
                ButtonKey::Zr,
                ButtonKey::Plus,
                ButtonKey::RStick,
                ButtonKey::Home,
            ],
            ControllerType::JoyConL => &[
                ButtonKey::Minus,
                ButtonKey::LStick,
                ButtonKey::Capture,
                ButtonKey::Down,
                ButtonKey::Up,
                ButtonKey::Right,
                ButtonKey::Left,
                ButtonKey::Sr,
                ButtonKey::Sl,
                ButtonKey::L,
                ButtonKey::Zl,
            ],
            _ => panic!("Unable to get available buttons for unknown controller type."),
        }
    }
}
