use std::{collections::HashMap, sync::{atomic::{AtomicU64, Ordering}, Arc}, time::Instant};
use tokio::sync::{Mutex, oneshot};
use anyhow::Result;

use crate::{
    log,
    ws::types::{RequestId, RpcError, RpcMessage},
};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Represents a pending request waiting for a response.
/// Stores the creation time and the oneshot sender to notify the waiter.
struct Pending {
    created: Instant,
    tx: oneshot::Sender<RpcMessage>,
}

/// Internal state holding all pending requests.
struct TrackerState {
    pending: HashMap<RequestId, Pending>,
}

/// Public request manager that wraps the internal state in Arc<Mutex<...>>.
/// Provides methods to register, resolve and cleanup requests.
#[derive(Clone)]
pub struct RequestTracker {
    inner: Arc<Mutex<TrackerState>>,
    timeout: std::time::Duration,
}

impl RequestTracker {
    /// Create a new RequestState with a default timeout of 30 seconds.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TrackerState {
                pending: HashMap::new(),
            })),
            timeout: std::time::Duration::from_secs(30),
        }
    }

    /// Override the default timeout with a custom duration.
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Register a new request and return a receiver to await the response.
    /// The caller awaits on the returned oneshot::Receiver<Result<String>>.
    pub async fn register(&self) -> (oneshot::Receiver<RpcMessage>, u64) {
        let id = next_id();
        let (tx, rx) = oneshot::channel();
        let mut guard = self.inner.lock().await;
        guard.pending.insert(
            id,
            Pending {
                created: Instant::now(),
                tx,
            },
        );
        (rx, id)
    }

    /// Deregister a request by id, removing it from pending requests.
    pub async fn deregister(&self, id: RequestId) {
        let mut guard = self.inner.lock().await;
        if guard.pending.remove(&id).is_some() {
            log::debug!("Deregistered request id {}", id);
        } else {
            log::warn!("Attempted to deregister unknown request id {}", id);
        }
    }

    /// Resolve a request by id with a successful payload.
    pub async fn resolve_ok(&self, id: RequestId, message: RpcMessage) -> Result<()> {
        log::debug!("Resolving request id {} with success", id);
        let mut guard = self.inner.lock().await;
        if let Some(p) = guard.pending.remove(&id) {
            let _ = p.tx.send(message);
            Ok(())
        } else {
            // Not found, may be ok if it's an external req
            Err(anyhow::anyhow!("Request id {} not found for resolution", id))
        }
    }

    /// Resolve a request by id with an error.
    pub async fn resolve_err(&self, id: RequestId, code: u32, message: String) -> Result<()> {
        self.resolve_ok(id, RpcMessage::Error(RpcError { code, message }))
            .await
    }

    /// Remove expired requests and notify their receivers with a timeout error.
    pub async fn cleanup(&self) {
        let mut guard = self.inner.lock().await;
        let now = std::time::Instant::now();

        // Collect expired request IDs first
        let expired: Vec<_> = guard
            .pending
            .iter()
            .filter(|(_, p)| now.duration_since(p.created) > self.timeout)
            .map(|(id, _)| *id)
            .collect();

        // Remove them and send timeout error
        for id in expired {
            if let Some(p) = guard.pending.remove(&id) {
                let _ = p.tx.send(RpcMessage::Error(RpcError {
                    code: 408,
                    message: "timeout".into(),
                }));
            }
        }
    }
}

impl Default for RequestTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Example background task that periodically calls cleanup.
/// For now it runs in an infinite loop; But we will implement it elsewhere.
/// Just an example of usage.
pub fn spawn_cleanup_task(tracker: RequestTracker) {
    tokio::spawn(async move {
        loop {
            tracker.cleanup().await;
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
}
