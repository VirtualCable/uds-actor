use anyhow::Result;
use std::{collections::HashMap, sync::Arc, time::Instant};
use tokio::sync::{Mutex, oneshot};

type RequestId = u64;

/// Represents a pending request waiting for a response.
/// Stores the creation time and the oneshot sender to notify the waiter.
struct Pending {
    created: Instant,
    tx: oneshot::Sender<Result<String>>,
}

/// Internal state holding all pending requests.
struct State {
    pending: HashMap<RequestId, Pending>,
}

/// Public request manager that wraps the internal state in Arc<Mutex<...>>.
/// Provides methods to register, resolve and cleanup requests.
#[derive(Clone)]
pub struct RequestState {
    inner: Arc<Mutex<State>>,
    timeout: std::time::Duration,
}

impl RequestState {
    /// Create a new RequestState with a default timeout of 30 seconds.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(State {
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
    pub async fn register(&self, id: RequestId) -> oneshot::Receiver<Result<String>> {
        let (tx, rx) = oneshot::channel();
        let mut guard = self.inner.lock().await;
        guard.pending.insert(
            id,
            Pending {
                created: Instant::now(),
                tx,
            },
        );
        rx
    }

    /// Resolve a request by id with a successful payload.
    pub async fn resolve_ok(&self, id: RequestId, payload: String) {
        let mut guard = self.inner.lock().await;
        if let Some(p) = guard.pending.remove(&id) {
            let _ = p.tx.send(Ok(payload));
        }
    }

    /// Resolve a request by id with an error.
    pub async fn resolve_err(&self, id: RequestId, err: anyhow::Error) {
        let mut guard = self.inner.lock().await;
        if let Some(p) = guard.pending.remove(&id) {
            let _ = p.tx.send(Err(err));
        }
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
                let _ = p.tx.send(Err(anyhow::anyhow!("timeout")));
            }
        }
    }
}

impl Default for RequestState {
    fn default() -> Self {
        Self::new()
    }
}

/// Example background task that periodically calls cleanup.
/// For now it runs in an infinite loop; later you can break it
/// using `session_manager.is_running()`.
pub async fn spawn_cleanup_task(state: RequestState) {
    tokio::spawn(async move {
        loop {
            state.cleanup().await;
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
}
