use anyhow::Result;

use shared::{
    log,
    ws::{
        server::ServerInfo,
        types::{LoginRequest, RpcEnvelope, RpcMessage},
        wait_for_request,
    },
};

use crate::platform;

// Owned ServerInfo and Platform
pub async fn worker(server_info: ServerInfo, platform: platform::Platform) -> Result<()> {
    let mut rx = server_info.wsclient_to_workers.subscribe();
    while let Some(env) = wait_for_request::<LoginRequest>(&mut rx, Some(platform.get_stop())).await {
        log::debug!("Received LoginRequest with id {:?}", env.id);
        let broker_api = platform.broker_api();

        let interfaces = platform.operations().get_network_info()?;
        if let Ok(response) =  broker_api.write().await.login(
            interfaces.as_slice(),
            env.msg.username.as_str(),
            env.msg.session_type.as_str(),
        ).await {
            let response_env = RpcEnvelope {
                id: env.id,
                msg: RpcMessage::LoginResponse(response),
            };
            if let Err(e) = server_info
                .workers_to_wsclient
                .send(response_env)
                .await
            {
                log::error!("Failed to send LoginResponse: {}", e);
            } else {
                log::debug!("Sent LoginResponse for id {:?}", env.id);
            }
        } else {
            log::error!("Login failed for user {}", env.msg.username);
        }
    }

    Ok(())
}
