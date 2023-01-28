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
        ButtonKey::available_buttons_for(self.controller)
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

// move this to main mod
// put available keys in map?
pub enum ButtonKey {
    Y,
    X,
    B,
    A,
    Sr,
    Sl,
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
    L,
    Zl,
}

impl ButtonKey {
    pub fn available_buttons_for(controller: ControllerType) -> &'static [ButtonKey] {
        match controller {
            ControllerType::JoyConL => &[ButtonKey::A],
            ControllerType::JoyConR => &[ButtonKey::B],
            ControllerType::ProController => &[ButtonKey::B],
            _ => panic!("Unable to get available buttons for unknown controller type."),
        }
    }
}
