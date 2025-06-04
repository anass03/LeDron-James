/// Used in multiple crates, needs to be pub
#[derive(Clone)]
pub enum LogType{
    WARN, ERROR, INFO, DEBUG, FATAL
}

/// Used in multiple crates, needs to be pub
#[derive(Clone)]
pub struct Logs{
    pub logs_memory: Vec<(LogType,String)>,
    pub log_scroll_offset: usize,
}

pub (crate) struct Statistics{
    pub total_packet_exchanged: usize,
    pub total_active_connections: usize,
    pub network_reliability: u8,
}

// Implementations

impl Logs{
    pub(super) fn default()->Self{
        Self{
            logs_memory: vec![],
            log_scroll_offset: 0,
        }
    }
    pub/*(crate)*/ fn push_log(&mut self, type_of_log: LogType, text: &str){
        if self.logs_memory.len()>=1000{
            self.logs_memory.remove(0);
            self.logs_memory.push((type_of_log, text.to_string()));
        }else{
            self.logs_memory.push((type_of_log, text.to_string()));
            self.log_scroll_offset += 1;
        }
    }
}
impl Statistics{
    pub fn default()->Self{
        Self{
            total_packet_exchanged: 0,
            total_active_connections: 0,
            network_reliability: 0,
        }
    }
    pub fn received_packet(&mut self){
        self.total_packet_exchanged += 1;
    }
}

