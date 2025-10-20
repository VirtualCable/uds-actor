use anyhow::Result;
use std::sync::Arc;

use shared::{
    operations,
    sync::OnceSignal,
    ws::client::{WsClient, websocket_client_tasks},
};

use crate::{gui, session::SessionManagement};

#[derive(Clone)]
pub struct Platform {
    session_manager: Arc<dyn SessionManagement>,
    operations: Arc<dyn operations::Operations>,
    ws_client: WsClient,
    stop: Arc<OnceSignal>,
}

impl Platform {
    pub async fn new(port: u16) -> Self {
        let session_manager = crate::session::new_session_manager().await;
        let operations = shared::operations::new_operations();

        let stop = Arc::new(OnceSignal::new());

        // Task for conecting session maanger and stop flag
        tokio::spawn({
            let stop = stop.clone();
            let session_manager = session_manager.clone();
            async move {
                tokio::select! {
                    _ = stop.wait() => {
                        session_manager.stop().await;
                    }
                    _ = session_manager.wait() => {
                        stop.set();
                    }
                }
            }
        });

        Self {
            session_manager,
            operations,
            ws_client: websocket_client_tasks(port, 32).await,
            stop,
        }
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

    pub fn get_stop(&self) -> Arc<OnceSignal> {
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
        port: u16,
    ) -> Self {
        let session_manager = if let Some(sm) = session_manager {
            sm
        } else {
            crate::session::new_session_manager().await
        };
        let operations = operations.unwrap_or_else(|| shared::operations::new_operations());
        let ws_client = if let Some(ws) = ws {
            ws
        } else {
            websocket_client_tasks(port, 32).await
        };

        let stop = Arc::new(OnceSignal::new());

        // Task for conecting session maanger and stop flag
        tokio::spawn({
            let stop = stop.clone();
            let session_manager = session_manager.clone();
            async move {
                tokio::select! {
                    _ = stop.wait() => {
                        session_manager.stop().await;
                    }
                    _ = session_manager.wait() => {
                        stop.set();
                    }
                }
            }
        });

        Self {
            session_manager,
            operations,
            ws_client,
            stop,
        }
    }

    pub fn shutdown(&self) {
        // self.gui.shutdown();
    }
}
