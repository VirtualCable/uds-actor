use std::sync::Arc;

use shared::{
    system,
    sync::OnceSignal,
    ws::{
        client::WsClient,
        types::{LoginRequest, LoginResponse, LogoutRequest, RpcEnvelope, RpcMessage},
        wait_message_arrival,
    },
};

#[async_trait::async_trait]
pub trait WsReqs: Send + Sync {
    async fn login(&self) -> anyhow::Result<LoginResponse>;
    async fn logout(&self, session_id: Option<&str>) -> anyhow::Result<()>;
}

pub struct WsRequester {
    operations: Arc<dyn system::System>,
    ws_client: WsClient,
    stop: OnceSignal,
}

impl WsRequester {
    pub fn new(operations: Arc<dyn system::System>, ws_client: WsClient, stop: OnceSignal) -> Self {
        Self { operations, ws_client, stop }
    }
}

#[async_trait::async_trait]
impl WsReqs for WsRequester {
    async fn login(&self) -> anyhow::Result<LoginResponse> {
        // Send login
        let username = self.operations.get_current_user()?;
        let session_type = self.operations.get_session_type()?;
        let ws_client = self.ws_client.clone();
        let stop = self.stop.clone();

        ws_client
            .to_ws
            .send(RpcEnvelope {
                id: Some(19720701), // Some arbitrary id
                msg: RpcMessage::LoginRequest(LoginRequest {
                    username: username.clone(),
                    session_type: session_type.clone(),
                }),
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send login message: {}", e))?;

        // Wait for response
        let mut rx = ws_client.from_ws.subscribe();
        let envelope = wait_message_arrival::<LoginResponse>(&mut rx, Some(stop))
            .await
            .ok_or_else(|| {
                anyhow::anyhow!("Failed to receive login response for user {}", username)
            })?;

        Ok(envelope.msg)
    }

    async fn logout(&self, session_id: Option<&str>) -> anyhow::Result<()> {
        let username = self.operations.get_current_user()?;
        let session_type = self.operations.get_session_type()?;
        let ws_client = self.ws_client.clone();

        ws_client
            .to_ws
            .send(RpcEnvelope {
                id: Some(19720701), // Some arbitrary id
                msg: RpcMessage::LogoutRequest(LogoutRequest {
                    username: username.clone(),
                    session_type: session_type.clone(),
                    session_id: session_id.unwrap_or_default().to_string(),
                }),
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send logout message: {}", e))?;

        Ok(())
    }
}
