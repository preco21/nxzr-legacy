use std::sync::Arc;

mod agent;
pub use agent::*;
mod logging;
pub use logging::*;
mod wsl;
pub use wsl::*;

pub struct AppState {
    pub wsl_manager: Arc<WslManager>,
    pub agent_manager: Arc<AgentManager>,
    pub logging_manager: Arc<LoggingManager>,
}
