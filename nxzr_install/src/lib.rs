use thiserror::Error;

#[derive(Clone, thiserror::Error, Debug)]
pub enum Error {
    #[error("foobar")]
    Foobar,
}

// One for windows
pub async fn prepare_system_requirements() -> Result<(), Error> {
    // 1. check wsl installed
    // 2. check if system can run wsl -> vm requirements (이건 그냥 체크되나? 1에서?)
    // 3. wsl version check if it's v2
    // 4. check usbipd installed -> maybe just include the binary
    // ㄴ https://github.com/dorssel/usbipd-win/wiki/WSL-support
    // 5. check wsl config is ready -> otherwise, install one
    // ㄴ check /etc/wsl.conf is ready -> otherwise, set one and restart vm (wait 8 sec)
    // 6. disable windows bt
    Ok(())
}

// One for linux
pub async fn ensure_system_requirements() -> Result<(), SysCheckError> {
    // sudo systemctl daemon-reload
    // sudo systemctl restart bluetooth
}

#[cfg(target_os = "linux")]
pub mod device;
