use anyhow::Result;

use shared::{
    log,
    ws::{types::LogoffRequest, wait_message_arrival},
};

use crate::platform;

// Owned ServerInfo and Platform
pub async fn worker(platform: platform::Platform) -> Result<()> {
    let mut rx = platform.ws_client().from_ws.subscribe();
    while let Some(_env) =
        wait_message_arrival::<LogoffRequest>(&mut rx, Some(platform.get_stop())).await
    {
        log::info!("Received logoff request, performing logoff");
        platform.get_stop().set();
        platform.operations().logoff()?;
    }

    Ok(())
}
