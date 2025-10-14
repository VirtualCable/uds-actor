use anyhow::Result;

use shared::{
    log,
    ws::{server::ServerInfo, types::PreConnect, wait_for_request},
};

use crate::{common, platform};

pub async fn run_preconnect(pre_command: &str, pre: &PreConnect) -> Result<()> {
    // If empty pre_command, do nothing
    if pre_command.trim().is_empty() {
        return Ok(());
    }
    common::run_command(
        "pre_command",
        pre_command,
        &[
            &pre.user,
            &pre.protocol,
            pre.ip.as_deref().unwrap_or_default(),
            pre.hostname.as_deref().unwrap_or_default(),
            pre.udsuser.as_deref().unwrap_or_default(),
        ],
    )
    .await
}

// Owned ServerInfo and Platform
pub async fn worker(server_info: ServerInfo, platform: platform::Platform) -> Result<()> {
    // Note that logoff is a simple notification. No response expected (in fact, will return "ok" immediately)
    let mut rx = server_info.wsclient_to_workers.subscribe();
    if let Some(env) = wait_for_request::<PreConnect>(&mut rx, Some(platform.get_stop())).await {
        log::debug!("Received PreConnect: {:?}", env.msg);
        // Process the Preconnect. If protocol is rdp, use operations::
        let msg = env.msg;
        if msg.protocol.to_lowercase() == "rdp" {
            if let Err(e) = platform.operations().ensure_user_can_rdp(&msg.user) {
                log::error!("Failed to ensure user can RDP: {}", e);
            } else {
                log::info!("Ensured user can RDP: {}", msg.user);
            }
        // If the a pre command is configured, run it
        } else if let Some(cmd) = platform.config().read().await.pre_command.clone() {
            if let Err(e) = run_preconnect(&cmd, &msg).await {
                log::error!("Failed to run pre-command for user {}: {}", msg.user, e);
            } else {
                log::info!("Ran pre-command for user {}", msg.user);
            }
        } else {
            log::info!("No action for protocol: {}", msg.protocol);
        }
    }
    Ok(())
}
