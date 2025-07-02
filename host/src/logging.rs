use crate::Host;

pub trait LogControl {
    fn send_logging(&self, log_type: LogType, log_module: LogModule, message: String);
}
impl LogControl for Host {
    fn send_logging(&self, log_type: LogType, log_module: LogModule, message: String) {
        let role = self.type_host;
        let log_msg = format!("[{:?}] ID {} - {}", role, self.id, message);
        let log_entry = LogEntry{
            log_type,
            module: log_module,
            message
        };
        if let Err(err) = self.log_sender.send(log_entry) {
            eprintln!("Failed to forward logging to controller: {}", err);
        }
    }
}
pub enum LogType{
    WARN, ERROR, INFO, DEBUG, FATAL
}
pub enum LogModule{
    FLOOD, NETWORK, SESSION, PACKET, FRAGMENT, ACK, NACK, CHAT, MEDIA, FILE,
}
pub struct LogEntry{
    log_type: LogType,
    module: LogModule,
    message: String,
}