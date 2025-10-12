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
pub async fn handle_log(server_info: ServerInfo, platform: platform::Platform) -> Result<()> {
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
