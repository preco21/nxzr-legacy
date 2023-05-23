use run_script::{run_script, IoOptions, ScriptError, ScriptOptions};
use thiserror::Error;

const INSTALL_SERVER_SCRIPT: &str = include_str!("scripts/install-server.sh");
const SETUP_CONFIG_SCRIPT: &str = include_str!("scripts/setup-config.sh");

#[derive(Error, Debug)]
pub enum ExternalScriptError {
    #[error("failed to run external script")]
    RunFailed(ScriptError),
    #[error("failed to install server: {0}")]
    ServerInstallFailed(String),
    #[error("failed to setup config: {0}")]
    ConfigSetupFailed(String),
}

impl From<ScriptError> for ExternalScriptError {
    fn from(err: ScriptError) -> Self {
        Self::RunFailed(err)
    }
}

#[tracing::instrument(target = "external_scripts")]
pub fn run_server_install() -> Result<(), ExternalScriptError> {
    let mut options = ScriptOptions::new();
    options.print_commands = true;
    options.output_redirection = IoOptions::Inherit;
    let (code, stdout, stderr) = run_script!(INSTALL_SERVER_SCRIPT, &options)?;
    if code != 0 {
        return Err(ExternalScriptError::ServerInstallFailed(format!(
            "{stdout}; {stderr}"
        )));
    }
    Ok(())
}

#[tracing::instrument(target = "external_scripts")]
pub fn run_setup_config() -> Result<(), ExternalScriptError> {
    let mut options = ScriptOptions::new();
    options.print_commands = true;
    options.output_redirection = IoOptions::Inherit;
    let (code, stdout, stderr) = run_script!(SETUP_CONFIG_SCRIPT, &options)?;
    if code != 0 {
        return Err(ExternalScriptError::ConfigSetupFailed(format!(
            "{stdout}; {stderr}"
        )));
    }
    Ok(())
}
