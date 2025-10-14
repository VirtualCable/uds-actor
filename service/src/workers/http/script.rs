use anyhow::Result;

use shared::{
    log,
    ws::{server::ServerInfo, types::ScriptExecRequest, wait_for_request},
};

use crate::platform;

// Owned ServerInfo and Platform
pub async fn worker(server_info: ServerInfo, platform: platform::Platform) -> Result<()> {
    // Note that logoff is a simple notification. No response expected (in fact, will return "ok" immediately)
    let mut rx = server_info.wsclient_to_workers.subscribe();
    if let Some(env) = wait_for_request::<ScriptExecRequest>(&mut rx, Some(platform.get_stop())).await {
        log::debug!("Received ScriptExecRequest");
        // Send logoff to wsclient
        let envelope = shared::ws::types::RpcEnvelope {
            id: None,
            msg: shared::ws::types::RpcMessage::ScriptExecRequest(ScriptExecRequest { script_type: env.msg.script_type, script: env.msg.script }),
        };
        if let Err(e) = server_info.workers_to_wsclient.send(envelope).await {
            log::error!("Failed to send ScriptExecRequest to wsclient: {}", e);
        } else {
            log::info!("Sent ScriptExecRequest to wsclient");
        }
    }
    Ok(())
}
