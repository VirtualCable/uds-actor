use anyhow::Result;

use shared::{
    log,
    ws::{types::Ping},
};

use crate::platform;

// Owned ServerInfo and Platform
pub async fn worker(platform: platform::Platform) -> Result<()> {
    let stop = platform.get_stop();
    // Err means timeout
    while stop
        .wait_timeout(std::time::Duration::from_secs(30))
        .await
        .is_err()
    {
        // Sending ping
        let ws_client = platform.ws_client();
        ws_client
            .to_ws
            .send(shared::ws::types::RpcEnvelope {
                id: None,
                msg: shared::ws::types::RpcMessage::Ping(Ping(b"ping".to_vec())),
            })
            .await?;
        log::debug!("Sent ping request");
    }

    Ok(())
}
