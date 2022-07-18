use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Notify;

pub struct Flag {
    state: AtomicBool,
    notify: Notify,
}

impl Flag {
    pub fn new(initial: bool) -> Flag {
        Flag {
            state: AtomicBool::new(initial),
            notify: Notify::new(),
        }
    }

    pub fn set(&self, new: bool) {
        let old = !new;
        let ord = Ordering::SeqCst;
        if self.state.compare_exchange_weak(old, new, ord, ord).is_ok() {
            self.notify.notify_waiters();
        }
    }

    pub async fn wait(&self, val: bool) {
        loop {
            if self.state.load(Ordering::SeqCst) == val {
                break;
            }

            self.notify.notified().await;
        }
    }
}
