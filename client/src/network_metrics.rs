use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct NetMetrics {
    pub tx_bytes: Arc<AtomicUsize>,
    pub rx_bytes: Arc<AtomicUsize>,
    prev_tx: usize,
    prev_rx: usize,
}

impl NetMetrics {
    pub fn new() -> Self {
        Self {
            tx_bytes: Arc::new(AtomicUsize::new(0)),
            rx_bytes: Arc::new(AtomicUsize::new(0)),
            prev_tx: 0,
            prev_rx: 0,
        }
    }

    pub fn update(&mut self) -> (f32, f32) {
        let total_tx = self.tx_bytes.load(Ordering::Relaxed);
        let total_rx = self.rx_bytes.load(Ordering::Relaxed);

        let up_speed = total_tx - self.prev_tx;
        let down_speed = total_rx - self.prev_rx;
        self.prev_tx = total_tx;
        self.prev_rx = total_rx;

        (up_speed as f32 / 1024.0, down_speed as f32 / 1024.0)
    }
}
