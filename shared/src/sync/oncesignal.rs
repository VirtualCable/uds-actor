use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;

/// A one-shot signal that can be awaited by multiple tasks.
/// Once fired, all current and future waiters will be released immediately.
#[derive(Clone, Debug)]
pub struct OnceSignal {
    fired: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl OnceSignal {
    /// Create a new, not-yet-fired signal.
    pub fn new() -> Self {
        Self {
            fired: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Fire the signal. This will wake all current waiters.
    /// Subsequent calls are idempotent.
    pub fn set(&self) {
        // Swap ensures we only notify once
        if !self.fired.swap(true, Ordering::SeqCst) {
            self.notify.notify_waiters();
        }
    }

    pub fn is_set(&self) -> bool {
        self.fired.load(Ordering::SeqCst)
    }

    /// Wait until the signal is fired.
    /// If it has already been fired, this returns immediately.
    pub async fn wait(&self) {
        if self.fired.load(Ordering::SeqCst) {
            return;
        }
        self.notify.notified().await;
    }
}

impl Default for OnceSignal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_once_signal_wakes_all() {
        let signal = OnceSignal::new();
        let mut tasks = Vec::new();

        for i in 0..10 {
            let s = signal.clone();
            tasks.push(tokio::spawn(async move {
                s.wait().await;
                println!("Task {} woke up", i);
            }));
        }

        // Give tasks time to start waiting
        sleep(Duration::from_millis(100)).await;

        // Fire the signal
        signal.set();

        for t in tasks {
            t.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_once_signal_immediate_return() {
        let signal = OnceSignal::new();
        signal.set();

        // Because it's already fired, wait returns immediately
        signal.wait().await;
    }
}
