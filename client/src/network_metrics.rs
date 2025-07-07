use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub struct NetMetrics {
    pub tx_bytes: Arc<AtomicUsize>,
    pub rx_bytes: Arc<AtomicUsize>,
    prev_tx: usize,
    prev_rx: usize,
    prev_time: Instant,
}

impl NetMetrics {
    pub fn new() -> Self {
        Self {
            tx_bytes: Arc::new(AtomicUsize::new(0)),
            rx_bytes: Arc::new(AtomicUsize::new(0)),
            prev_tx: 0,
            prev_rx: 0,
            prev_time: Instant::now(),
        }
    }

    /// Returns (upload_kBps, download_kBps)
    pub fn update(&mut self) -> (f32, f32) {
        let total_tx = self.tx_bytes.load(Ordering::Relaxed);
        let total_rx = self.rx_bytes.load(Ordering::Relaxed);

        let now = Instant::now();
        let elapsed = now.duration_since(self.prev_time).as_secs_f32();

        let secs = if elapsed > 0.0 { elapsed } else { 1.0 };

        let delta_tx = total_tx - self.prev_tx;
        let delta_rx = total_rx - self.prev_rx;

        let up_kbps = (delta_tx as f32 / 1024.0) / secs;
        let down_kbps = (delta_rx as f32 / 1024.0) / secs;

        self.prev_tx = total_tx;
        self.prev_rx = total_rx;
        self.prev_time = now;

        (up_kbps, down_kbps)
    }
}
