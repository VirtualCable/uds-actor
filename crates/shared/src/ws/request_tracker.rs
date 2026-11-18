use anyhow::Result;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
use tokio::sync::{Mutex, oneshot};

use crate::{
    log,
    ws::types::{RequestId, RpcError, RpcMessage},
};

// Ids issued by next_id() are correlation handles for the internal RPC tracker:
// a peer connected to /ws that can *guess* a pending id could resolve somebody
// else's request (e.g. forge a ScreenshotResponse for an id the broker waits
// on). A deterministic per-process sequence makes that trivial, so the base is
// drawn once from the CSPRNG; ids stay monotonically increasing (collision-free
// within a process) but are not predictable from outside.
static ID_BASE: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

fn next_id() -> u64 {
    let base = *ID_BASE.get_or_init(|| rand::random::<u64>() | 1);
    base.wrapping_add(NEXT_ID.fetch_add(1, Ordering::Relaxed))
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
            log::debug!("Request id {} found, sending response", id);
            let _ = p.tx.send(message);
            Ok(())
        } else {
            // Not found, may be ok if it's an external req
            Err(anyhow::anyhow!("Request id {} not found", id))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws::types::RpcError;

    fn ok_msg(tag: &str) -> RpcMessage {
        RpcMessage::Error(RpcError {
            code: 200,
            message: tag.to_string(),
        })
    }

    #[tokio::test]
    async fn ids_are_unique_and_not_a_small_sequence() {
        // Regression for ws-rpc-tracker-id-only-correlation: ids must not be a
        // predictable 1,2,3... sequence (a local /ws peer guessing pending ids
        // could resolve other sessions' requests).
        let mut ids = Vec::new();
        for _ in 0..8 {
            let (_rx, id) = RequestTracker::new().register().await;
            ids.push(id);
        }
        let mut uniq = ids.clone();
        uniq.sort_unstable();
        uniq.dedup();
        assert_eq!(uniq.len(), ids.len(), "ids must be unique");
        // Sequentially collected ids must be strictly increasing (the global
        // counter only ever moves forward). Do NOT assert a delta of 1: the
        // counter is process-global and concurrent tests consume ids too.
        assert!(
            ids.windows(2).all(|w| w[1] > w[0]),
            "ids must increase monotonically: {ids:?}"
        );
        // With overwhelming probability (64-bit CSPRNG base) the first id is
        // far from the process start; a sequential implementation would give 1.
        assert_ne!(
            ids[0], 1,
            "id sequence must not start at a known deterministic base"
        );
    }

    #[tokio::test]
    async fn resolve_ok_delivers_and_removes_entry() {
        let tracker = RequestTracker::new();
        let (rx, id) = tracker.register().await;
        tracker.resolve_ok(id, ok_msg("payload")).await.unwrap();
        match rx.await.unwrap() {
            RpcMessage::Error(e) => assert_eq!(e.message, "payload"),
            other => panic!("expected the resolved payload, got {other:?}"),
        }
        // Second resolve of the same id must fail (entry was removed).
        assert!(tracker.resolve_ok(id, ok_msg("again")).await.is_err());
    }

    #[tokio::test]
    async fn cleanup_expires_pending_with_timeout_error() {
        let tracker = RequestTracker::new().with_timeout(std::time::Duration::from_millis(10));
        let (rx, _id) = tracker.register().await;
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        tracker.cleanup().await;
        match rx.await {
            Ok(RpcMessage::Error(e)) => {
                assert_eq!(e.code, 408, "expired requests must resolve with 408");
                assert_eq!(e.message, "timeout");
            }
            other => panic!("expected 408 timeout error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn deregister_removes_entry() {
        let tracker = RequestTracker::new();
        let (_rx, id) = tracker.register().await;
        tracker.deregister(id).await;
        assert!(tracker.resolve_ok(id, ok_msg("late")).await.is_err());
    }
}

/// Background task that periodically calls cleanup, so timed-out requests do
/// not accumulate in the pending map for the process lifetime. It exits when
/// the server's stop signal fires, keeping the task lifecycle tied to the WS
/// server that owns the tracker.
pub fn spawn_cleanup_task(tracker: RequestTracker, stop: crate::sync::OnceSignal) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = stop.wait() => break,
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
            }
            tracker.cleanup().await;
        }
    });
}
