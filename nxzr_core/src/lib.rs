pub mod controller;
pub(crate) mod error;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use crate::controller::info::CONTROLLER_INFO_MAP;

    use super::*;

    #[test]
    fn it_works() {
        let a = CONTROLLER_INFO_MAP.get(&controller::info::ControllerType::ProController);
        if let Some(aa) = a {
            println!("{}", aa.id);
            assert_eq!(aa.connection_info, 0x00);
        }
    }
}
