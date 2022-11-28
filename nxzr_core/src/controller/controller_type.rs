#[derive(Debug)]
pub enum ControllerType {
    JoyConL,
    JoyConR,
    ProController,
}

impl Default for ControllerType {
    fn default() -> Self {
        Self::ProController
    }
}
