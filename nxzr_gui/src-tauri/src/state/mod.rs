use std::sync::Arc;

mod agent;
pub use agent::*;
mod logging;
pub use logging::*;

pub struct AppState {
    pub agent_manager: Arc<AgentManager>,
    pub logging_manager: Arc<LoggingManager>,
}
