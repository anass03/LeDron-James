use rand::prelude::ThreadRng;
use rand::Rng;
use std::collections::HashSet;
use std::time::Instant;
use wg_2024::network::NodeId;
use wg_2024::packet::Fragment;

pub struct RecSessionData {
    pub buffer: Vec<Option<Fragment>>,
    pub last_activity: Instant, // Timestamp dell'ultima attività
    pub pending: usize,         // quanti frammenti ancora da ricevere
}
pub struct SentSessionData {
    pub buffer: Vec<Option<Fragment>>, // Buffer dei frammenti inviati
    pub last_activity: Instant,        // Timestamp dell'ultima attività
    pub destination: NodeId,           // Destinatario della sessione
    pub route: Vec<NodeId>,            // percorso pre-calcolato
    pub pending: usize,                // quanti frammenti ancora in attesa di ACK
    pub banned_nodes: HashSet<NodeId>,
}
pub fn generate_unique_session_id() -> u64 {
    let mut rng: ThreadRng = rand::rng(); // messo rng al posto di thread_rng pk quest'ultimo dalla prossima versione verrà rinominato in rng! [thread_rng --> rng]
    rng.random::<u64>() // lo stesso qui, [gen  -->  random]
}
