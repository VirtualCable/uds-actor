use anyhow::Result;

use shared::{
    broker::api::BrokerApi, log, ws::{
        server::ServerInfo,
        types::{LoginRequest, RpcEnvelope, RpcMessage},
        wait_for_request,
    }
};

use crate::{common, platform};

// Login on unmanaged actor is a bit different.
// On VM Start, we could not "identify" the machine with an user service, because not user service is created yet for this.
// So, we just wait for a LoginRequest from the wsclient, and then notify correctly to the broker.
// So we need to call "initialize" prior to login, to get the machine attached to the userservice.
// Also, we will no "save" the token, store it on memoy only, as unmanaged actors are not supposed to be long-lived.
pub async fn worker(server_info: ServerInfo, platform: platform::Platform) -> Result<()> {
    let mut rx = server_info.wsclient_to_workers.subscribe();
    while let Some(env) = wait_for_request::<LoginRequest>(&mut rx, Some(platform.get_stop())).await
    {
        log::debug!("Received LoginRequest with id {:?}", env.id);
        let broker_api: std::sync::Arc<tokio::sync::RwLock<dyn BrokerApi>> = platform.broker_api();

        let interfaces = platform.operations().get_network_info()?;
        
        if let Err(e) = common::initialize(&platform).await {
            log::error!("Failed to initialize unmanaged actor prior to login: {}", e);
            continue;
        }

        if let Ok(response) = broker_api
            .write()
            .await
            .login(
                interfaces.as_slice(),
                env.msg.username.as_str(),
                env.msg.session_type.as_str(),
            )
            .await
        {
            let response_env = RpcEnvelope {
                id: env.id,
                msg: RpcMessage::LoginResponse(response),
            };
            if let Err(e) = server_info.workers_to_wsclient.send(response_env).await {
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testing::dummy;
    use std::time::Duration;

    #[tokio::test]
    async fn test_login_worker() {
        log::setup_logging("debug", shared::log::LogType::Tests);
        let server_info = dummy::create_dummy_server_info().await;
        let (platform, calls) = dummy::create_dummy_platform().await;
        platform.config().write().await.master_token = Some("mastertoken".into());

        let wsclient_to_workers = server_info.wsclient_to_workers.clone();
        let _handle = tokio::spawn(async move {
            worker(server_info, platform).await.unwrap();
        });

        // Wait to have at least one receiver
        while wsclient_to_workers.receiver_count() == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        log::info!("wsclient_to_workers has receiver");

        // Send 3 login requests
        for i in 1..=3 {
            let req = RpcEnvelope {
                id: None,
                msg: RpcMessage::LoginRequest(LoginRequest {
                    username: format!("user{}", i),
                    session_type: "test".into(),
                }),
            };
            log::info!("Sending login request for user{}", i);
            wsclient_to_workers.send(req).unwrap();
        }

        // Wait a bit to let processing happen
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Inspect dummy broker_api
        log::info!("calls: {:?}", calls.dump());
        assert!(calls.count_calls("broker_api::initialize(") == 3);
    }
}
