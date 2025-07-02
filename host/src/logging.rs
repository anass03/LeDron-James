use crossbeam_channel::Sender;
use crate::Host;
use tracing::{info, warn, error, debug, trace};
use crate::network::Network;

pub trait LogControl {
    fn send_logging(&self, log_type: LogType, log_module: LogModule, message: String);
}
impl LogControl for Host {
    fn send_logging(&self, log_type: LogType, log_module: LogModule, message: String) {
        // Messaggio strutturato
        let role = self.type_host;
        let log_msg = format!("[{:?}] ID {} - {}", role, self.id, &message);

        // Logging con tracing
        match log_type {
            LogType::INFO => info!(module = %log_module, id = self.id, role = ?role, "{}", message),
            LogType::WARN => warn!(module = %log_module, id = self.id, role = ?role, "{}", message),
            LogType::ERROR => error!(module = %log_module, id = self.id, role = ?role, "{}", message),
            LogType::DEBUG => debug!(module = %log_module, id = self.id, role = ?role, "{}", message),
            LogType::FATAL => error!(target: "fatal", module = %log_module, id = self.id, role = ?role, "{}", message),
        }

        // LogEntry per il Simulation Controller
        let log_entry = LogEntry {
            log_type,
            module: log_module,
            message: log_msg,
        };

        if let Err(err) = self.log_sender.send(log_entry) {
            eprintln!("Failed to forward logging to controller: {}", err);
        }
    }
}
impl LogControl for Sender<LogEntry> {
    fn send_logging(&self, log_type: LogType, log_module: LogModule, message: String) {
        // we don’t need the tracing here–just forward to the controller
        let log_entry = LogEntry { log_type, module: log_module, message };
        let _ = self.send(log_entry);
    }
}
pub enum LogType{
    WARN, ERROR, INFO, DEBUG, FATAL
}
#[derive(Debug)]
pub enum LogModule{
    FLOOD, NETWORK, SESSION, PACKET, FRAGMENT, ACK, NACK, CHAT, MEDIA, FILE,
}
pub struct LogEntry{
    pub log_type: LogType,
    pub module: LogModule,
    pub message: String,
}
impl std::fmt::Display for LogModule {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}