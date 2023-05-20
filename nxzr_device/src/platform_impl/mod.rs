// #[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform;
// #[cfg(target_os = "windows")]
// #[path = "windows/mod.rs"]
// mod platform;

pub use self::platform::*;

#[cfg(all(not(target_os = "linux"), not(target_os = "windows")))]
compile_error!("The platform you're compiling for is not supported by nxzr_device");
