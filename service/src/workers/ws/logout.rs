use anyhow::Result;

use shared::{
    log,
    ws::{server::ServerInfo, types::LogoutRequest, wait_for_request},
};

use crate::platform;

// Owned ServerInfo and Platform
pub async fn worker(server_info: ServerInfo, platform: platform::Platform) -> Result<()> {
    // Note that logout is a simple notification. No response expected (in fact, will return "ok" immediately)
    let mut rx = server_info.wsclient_to_workers.subscribe();
    if let Some(env) = wait_for_request::<LogoutRequest>(&mut rx, None).await {
        log::debug!("Received LogoutRequest with id {:?}", env.id);
        let broker_api = platform.broker_api();
        // Clone api to avoid holding the lock during await
        let interfaces = platform.operations().get_network_info()?;
        if let Err(err) = broker_api
            .write()
            .await
            .logout(
                interfaces.as_slice(),
                env.msg.username.as_str(),
                env.msg.session_type.as_str(),
                env.msg.session_id.as_str(),
            )
            .await
        {
            log::error!("Logout failed for user {}: {:?}", env.msg.username, err);
        } else {
            log::debug!("Processed LogoutRequest for user {}", env.msg.username);
        }
    }

    Ok(())
}
