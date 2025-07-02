// File: src/logging.rs

use crate::host::Host;
use crossbeam_channel::Sender;
pub trait LogControl {
    fn send_logging(&self, log_type: LogType, log_module: LogModule, message: String);
}

impl LogControl for Host {
    fn send_logging(&self, log_type: LogType, log_module: LogModule, message: String) {
        let log_entry = LogEntry {
            log_type,
            module: log_module,
            message,
        };
        let _ = self.log_sender.send(log_entry);
    }
}

#[derive(Clone)]
pub struct InlineLogger<'a>(pub &'a Sender<LogEntry>);

impl<'a> LogControl for InlineLogger<'a> {
    fn send_logging(&self, log_type: LogType, module: LogModule, message: String) {
        let _ = self.0.send(LogEntry {
            log_type,
            module,
            message,
        });
    }
}

#[derive(Clone, Debug)]
pub enum LogType {
    WARN,
    ERROR,
    INFO,
    DEBUG,
    FATAL,
}

#[derive(Clone, Debug)]
pub enum LogModule {
    FLOOD,
    NETWORK,
    SESSION,
    PACKET,
    FRAGMENT,
    ACK,
    NACK,
    CHAT,
    MEDIA,
    FILE,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub log_type: LogType,
    pub module: LogModule,
    pub message: String,
}
