use crate::Host;
use tracing::{error, warn, info, debug, trace};

/// Log levels corresponding to tracing macros
#[derive(Debug, Copy, Clone)]
pub enum LogType {
    WARN, ERROR, INFO, DEBUG
}

/// Application modules
#[derive(Debug, Copy, Clone)]
pub enum LogModule {
    FLOOD, NETWORK, SESSION, PACKET, FRAGMENT, ACK, NACK, CHAT, MEDIA, FILE,
}

impl std::fmt::Display for LogModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Use Debug so output matches the variant name
        write!(f, "{:?}", self)
    }
}

/// Trait for emitting logs
pub trait LogControl {
    /// Sends a log with the given level, module, and message.
    fn send_logging(&self, level: LogType, module: LogModule, msg: String);
}

/// Implement LogControl for Host, forwarding to tracing macros
impl LogControl for Host {
    fn send_logging(&self, level: LogType, module: LogModule, msg: String) {
        // Build the target string, e.g., "42_Client" or "10_Drone"
        let target = format!("{}_{:?}", self.id, self.type_host);
        // Format message as "[Module] message"
        let formatted = format!("[{}] {}", module, msg);

        match level {
            LogType::ERROR => error!("{} - {}", target, formatted),
            LogType::WARN  => warn!("{} - {}", target, formatted),
            LogType::INFO  => info!("{} - {}", target, formatted),
            LogType::DEBUG => debug!("{} - {}", target, formatted),
        }
    }
}
