use crossbeam_channel::Sender;
use host::{ContentID, HostEvent, ServerId};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
#[derive(Clone)]
pub struct TransferTracker {
    pub uploads: Arc<Mutex<HashMap<ContentID, (usize, Arc<AtomicUsize>)>>>,
}

impl TransferTracker {
    pub fn new() -> Self {
        Self {
            uploads: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start(&self, id: ContentID, total: usize) {
        if let Ok(mut lock) = self.uploads.lock() {
            lock.insert(id.clone(), (total, Arc::new(AtomicUsize::new(0))));
        } else {
            eprintln!(
                "[TransferTracker] Failed to acquire lock in start() for {:?}",
                id
            );
        }
    }

    pub fn get_closure(
        &self,
        id: ContentID,
        server_id: ServerId,
        ctrl: Sender<HostEvent>,
    ) -> impl FnMut(usize) + Send + 'static {
        let map = self.uploads.clone();
        move |chunk_len: usize| {
            let mut remove = false;
            let percent_opt = match map.lock() {
                Ok(mut uploads) => {
                    if let Some((total, counter)) = uploads.get_mut(&id) {
                        let current = counter.fetch_add(chunk_len, Ordering::Relaxed) + chunk_len;
                        if current >= *total {
                            remove = true;
                        }
                        Some((current, *total))
                    } else {
                        None
                    }
                }
                Err(_) => {
                    return;
                }
            };

            if let Some((current, total)) = percent_opt {
                let percent = (current as f32 / total as f32) * 100.0;
                let _ = ctrl.send(HostEvent::UploadingInfo(server_id, id.clone(), percent));
            }

            if remove {
                if let Ok(mut uploads) = map.lock() {
                    uploads.remove(&id);
                }
            }
        }
    }
}
