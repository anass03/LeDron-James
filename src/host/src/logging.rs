use crossbeam_channel::Sender;
use crate::Host;
use tracing::{info, warn, error, debug, trace};
use crate::network::Network;

pub trait LogControl {
    fn send_logging(&self, log_type: LogType, log_module: LogModule, message: String);
}
impl LogControl for Host {
    fn send_logging(&self, log_type: LogType, log_module: LogModule, message: String) {
        let role = self.type_host;
        let target = format!("{}_{}", self.id, role);
        let formatted_msg = format!("[{}] {}", log_module, message);

        match log_type {
            LogType::INFO => info!(target: &target, "{}", formatted_msg),
            LogType::WARN => warn!(target: &target, "{}", formatted_msg),
            LogType::ERROR => error!(target: &target, "{}", formatted_msg),
            LogType::DEBUG => debug!(target: &target, "{}", formatted_msg),
            LogType::FATAL => error!(target: "fatal", "{}", formatted_msg),
        }
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