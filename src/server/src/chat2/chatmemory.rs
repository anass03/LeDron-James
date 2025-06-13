use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crossbeam_channel::{unbounded, Sender, Receiver};
use host::ServerToClient;
use wg_2024::network::NodeId;

const MAX_HISTORY: usize = 100;

/// In-memory shared chat history and broadcast to multiple subscribers.
pub struct ChatMemory {
    histories: Arc<Mutex<HashMap<NodeId, Vec<ServerToClient>>>>,
    broadcasters: Arc<Mutex<HashMap<NodeId, Vec<Sender<ServerToClient>>>>>,
}

impl Clone for ChatMemory {
    fn clone(&self) -> Self {
        ChatMemory {
            histories: Arc::clone(&self.histories),
            broadcasters: Arc::clone(&self.broadcasters),
        }
    }
}
/// SharedChatMemory: Arc wrapper
pub type SharedChatMemory = Arc<ChatMemory>;

impl ChatMemory {
    /// Crea una nuova memoria condivisa
    pub fn new() -> Self {
        ChatMemory {
            histories: Arc::new(Mutex::new(HashMap::new())),
            broadcasters: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Assicura che esista un broadcaster per la stanza `room_id`.
    pub fn ensure_broadcaster(&self, room_id: NodeId) {
        let mut bmap = self.broadcasters.lock().unwrap();
        bmap.entry(room_id).or_default();
    }

    /// Sottoscrive al live feed di `room_id`, restituisce un Receiver
    pub fn subscribe(&self, room_id: NodeId) -> Receiver<ServerToClient> {
        let (tx, rx) = unbounded();
        let mut bmap = self.broadcasters.lock().unwrap();
        bmap.entry(room_id).or_default().push(tx);
        rx
    }

    /// Restituisce lo storico corrente di `room_id` (clonato)
    pub fn get_history(&self, room_id: NodeId) -> Vec<ServerToClient> {
        let h = self.histories.lock().unwrap();
        h.get(&room_id).cloned().unwrap_or_default()
    }

    /// Aggiunge un messaggio allo storico e lo invia a tutti i subscriber live
    pub fn push_and_broadcast(&self, room_id: NodeId, msg: ServerToClient) {
        // 1) salva nello storico
        {
            let mut h = self.histories.lock().unwrap();
            let vec = h.entry(room_id).or_default();
            vec.push(msg.clone());
            if vec.len() > MAX_HISTORY {
                vec.remove(0);
            }
        }
        // 2) invia ai subscriber
        let bmap = self.broadcasters.lock().unwrap();
        if let Some(senders) = bmap.get(&room_id) {
            for sender in senders {
                let _ = sender.send(msg.clone());
            }
        }
    }
}
