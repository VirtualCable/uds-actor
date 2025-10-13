use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use shared::{
    log,
    ws::{server::ServerInfo, types::LogRequest, wait_for_request},
};

use crate::platform;

/// FloodGuard: simple rate limiter (60 logs / 60s)
pub struct FloodGuard {
    count: usize,
    window_start: Instant,
}

impl FloodGuard {
    pub fn new() -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
        }
    }

    pub fn allow(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.window_start) > Duration::from_secs(60) {
            self.count = 0;
            self.window_start = now;
        }
        if self.count < 60 {
            self.count += 1;
            true
        } else {
            false
        }
    }
}

// Owned ServerInfo and Platform
pub async fn worker(server_info: ServerInfo, platform: platform::Platform) -> Result<()> {
    let mut rx = server_info.wsclient_to_workers.subscribe();
    let flood_guard = Arc::new(Mutex::new(FloodGuard::new()));

    while let Some(env) = wait_for_request::<LogRequest>(&mut rx, None).await {
        let mut guard = flood_guard.lock().await;
        if guard.allow() {
            log::debug!(
                "Client log (id {:?}, level: {:?}, message: {})",
                env.id,
                env.msg.level,
                env.msg.message
            );
            let api = platform.broker_api();
            if let Err(e) = api
                .write()
                .await
                .log(env.msg.level, env.msg.message.as_str())
                .await
            {
                log::error!("Failed to send log to broker: {:?}", e);
            } else {
                log::debug!("Sent log to broker for id {:?}", env.id);
            }
        } else {
            log::warn!("Flood detected: dropping log from client (id {:?})", env.id);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use shared::ws::{
        request_tracker::RequestTracker,
        types::{RpcEnvelope, RpcMessage},
    };
    use tokio::sync::{broadcast, mpsc};

    use super::*;

    #[tokio::test]
    async fn flood_guard_allows_up_to_60_per_minute() {
        log::setup_logging("debug", shared::log::LogType::Tests);
        let mut guard = FloodGuard::new();

        // First 60 should be allowed
        for _ in 0..60 {
            assert!(guard.allow());
        }

        // The 61st should be rejected
        assert!(!guard.allow());

        // Advance the clock artificially (if using tokio::time::pause/advance)
        // or manipulate window_start to simulate the passage of time
        guard.window_start -= Duration::from_secs(61);

        // Now it should reset and allow again
        assert!(guard.allow());
    }

    #[tokio::test]
    async fn handle_log_respects_flood_guard() {
        log::setup_logging("debug", shared::log::LogType::Tests);
        let (workers_tx, _workers_rx) = mpsc::channel::<RpcEnvelope<RpcMessage>>(128);
        let (wsclient_to_workers, _) = broadcast::channel::<RpcEnvelope<RpcMessage>>(128);
        let tracker = RequestTracker::new();

        let handle = tokio::spawn(async move {
            // Dummy task to keep the server_info.task valid
            tokio::time::sleep(Duration::from_secs(10)).await;
        });

        let server_info = ServerInfo {
            workers_to_wsclient: workers_tx,
            wsclient_to_workers: wsclient_to_workers.clone(),
            tracker,
            task: Arc::new(handle),
        };

        let (platform, calls) = crate::testing::dummy::create_dummy_platform().await;

        // Spawn worker
        tokio::spawn(worker(server_info, platform.clone()));

        // Wait to have at least one receiver
        while wsclient_to_workers.receiver_count() == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Send 65 log messages
        for i in 0..65 {
            let req = RpcEnvelope {
                id: None,
                msg: RpcMessage::LogRequest(LogRequest {
                    level: "INFO".into(),
                    message: format!("msg {i}"),
                }),
            };
            wsclient_to_workers.send(req).unwrap();
        }

        // Wait a bit to let processing happen
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Inspecciona dummy broker_api
        log::info!("calls: {:?}", calls.dump());
    }
}
