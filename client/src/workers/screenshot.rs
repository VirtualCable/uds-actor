use anyhow::Result;

use shared::{
    log,
    ws::{
        types::{ScreenshotRequest},
        wait_message_arrival,
    },
};

use crate::platform;

// Owned ServerInfo and Platform
pub async fn worker(platform: platform::Platform) -> Result<()> {
    let mut rx = platform.ws_client().from_ws.subscribe();
    while let Some(env) = wait_message_arrival::<ScreenshotRequest>(&mut rx, Some(platform.get_stop())).await
    {
        // Currently, no screenshot supported
        log::warn!("Received screenshot request, but screenshot worker is not implemented: {:?}", env);
    }

    Ok(())
}

