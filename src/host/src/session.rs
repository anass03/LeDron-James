// File: src/session.rs

use std::collections::HashSet;
use std::time::Instant;
use rand::prelude::ThreadRng;
use rand::Rng;
use wg_2024::network::NodeId;
use wg_2024::packet::Fragment;

pub struct RecSessionData {
    pub buffer: Vec<Option<Fragment>>,
    pub last_activity: Instant,
    pub pending: usize,
}

pub struct SentSessionData {
    pub buffer: Vec<Option<Fragment>>,
    pub last_activity: Instant,
    pub destination: NodeId,
    pub route: Vec<NodeId>,
    pub pending: usize,
    pub(crate) banned_nodes: HashSet<NodeId>,
}

pub fn generate_unique_session_id() -> u64 {
    let mut rng: ThreadRng = rand::rng();
    rng.gen()
}
