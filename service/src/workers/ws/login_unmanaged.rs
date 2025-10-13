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

// Login on unmanaged actor is a bit different.
// On VM Start, we could not "identify" the machine with an user service, because not user service is created yet for this.
// So, we just wait for a LoginRequest from the wsclient, and then notify correctly to the broker.
// So we need to call "initialize" prior to login, to get the machine attached to the userservice.
// Also, we will no "save" the token, store it on memoy only, as unmanaged actors are not supposed to be long-lived.
pub async fn worker(server_info: ServerInfo, platform: platform::Platform) -> Result<()> {
    let mut rx = server_info.wsclient_to_workers.subscribe();
    while let Some(env) = wait_for_request::<LoginRequest>(&mut rx, None).await {
        log::debug!("Received LoginRequest with id {:?}", env.id);
        let broker_api = platform.broker_api();

        let interfaces = platform.operations().get_network_info()?;
        // On unmanaged, on login, we will force the use of master_token
        // Initialize
        let master_token = platform
            .config()
            .read()
            .await
            .master_token
            .clone()
            .unwrap_or_default();
        // Ensure we use the correct token for this
        broker_api.write().await.set_token(&master_token);
        if let Ok(response) = broker_api
            .write()
            .await
            .initialize(interfaces.as_slice())
            .await
        {
            // If token on response is none, this is not a managed host,continue until next request
            if response.token.is_none() {
                log::error!(
                    "Unmanaged actor initialization did not return a token, cannot continue login"
                );
                continue;
            }

            let cfg = platform.config();  // Avoid drop while writing
            let mut cfg = cfg.write().await;

            // If master token is present on response, and is different of current, update it
            if let Some(master_token) = response.master_token
                && cfg.master_token.as_ref() != Some(&master_token)
            {
                log::info!("Master token updated from broker");
                cfg.master_token = Some(master_token);
                // We need to save here, rest of config is "volatile" form unmanaged
                let mut saver = platform.config_storage();
                if let Err(e) = saver.save_config(&cfg) {
                    log::error!("Failed to save updated config with new master_token: {}", e);
                }
            }
            cfg.own_token = response.token;
            cfg.config.unique_id = response.unique_id;
            cfg.config.os = response.os;

            // Note, we do not save

            // Now, set back the broker_api token to the new own_token
            if let Some(own_token) = cfg.own_token.clone() {
                broker_api.write().await.set_token(&own_token);
            }

            log::debug!("Unmanaged actor initialized, proceeding with login");
        } else {
            log::error!("Failed to initialize unmanaged actor prior to login");
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
