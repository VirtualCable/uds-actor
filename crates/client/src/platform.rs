use anyhow::Result;
use std::sync::Arc;

use shared::{
    operations,
    sync::OnceSignal,
    ws::client::{WsClient, websocket_client_tasks},
};

use crate::{gui, session::SessionManagement, ws_reqs::{WsReqs, WsRequester}};

#[derive(Clone)]
pub struct Platform {
    session_manager: Arc<dyn SessionManagement>,
    operations: Arc<dyn operations::Operations>,
    ws_client: WsClient,
    ws_requester: Arc<dyn WsReqs>,
    stop: OnceSignal,
}

impl Platform {
    pub async fn new(port: u16) -> Result<Self> {
        // If cannot connect, do not initialize the rest of the platform
        let ws_client = websocket_client_tasks(port, 32).await?;
        let stop = OnceSignal::new();
        let session_manager = crate::session::new_session_manager(stop.clone()).await;
        let operations = shared::operations::new_operations();
        // Requester needs a few things
        let ws_requester = Arc::new(WsRequester::new(
            operations.clone(),
            ws_client.clone(),
            stop.clone(),
        ));

        Ok(Self {
            session_manager,
            operations,
            ws_client,
            ws_requester,
            stop,
        })
    }

    pub fn session_manager(&self) -> Arc<dyn SessionManagement> {
        self.session_manager.clone()
    }

    pub fn operations(&self) -> Arc<dyn shared::operations::Operations> {
        self.operations.clone()
    }

    pub fn ws_client(&self) -> WsClient {
        self.ws_client.clone()
    }

    pub fn ws_requester(&self) -> Arc<dyn WsReqs> {
        self.ws_requester.clone()
    }

    pub fn stop(&self) -> OnceSignal {
        self.stop.clone()
    }

    pub async fn notify_user(&self, message: &str) -> Result<()> {
        let message = message.to_string();
        gui::message_dialog("uds-actor Notification", &message).await
    }

    pub async fn dismiss_user_notifications(&self) -> Result<()> {
        gui::close_all_windows().await
    }

    // Only for tests
    #[cfg(test)]
    pub async fn new_with_params(
        session_manager: Option<Arc<dyn SessionManagement>>,
        operations: Option<Arc<dyn shared::operations::Operations>>,
        ws: Option<WsClient>,
        ws_requester: Option<Arc<dyn WsReqs>>,
        stop: Option<OnceSignal>,
        port: u16,
    ) -> Result<Self> {
        let stop = stop.unwrap_or_default();

        let session_manager = if let Some(sm) = session_manager {
            sm
        } else {
            crate::session::new_session_manager(stop.clone()).await
        };
        let operations = operations.unwrap_or_else(|| shared::operations::new_operations());
        let ws_client = if let Some(ws) = ws {
            ws
        } else {
            websocket_client_tasks(port, 32).await?
        };

        let ws_requester = if let Some(wsr) = ws_requester {
            wsr
        } else {
            Arc::new(WsRequester::new(
                operations.clone(),
                ws_client.clone(),
                stop.clone(),
            ))
        };

        Ok(Self {
            session_manager,
            operations,
            ws_client,
            ws_requester,
            stop,
        })
    }

    pub fn shutdown(&self) {
        // self.gui.shutdown();
    }
}
