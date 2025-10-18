use anyhow::Result;
use std::sync::Arc;

use crate::{
    rest::api::{ClientRest, new_client_rest_api},
    session::SessionManagement,
    gui,
};

#[derive(Clone)]
pub struct Platform {
    session_manager: Arc<dyn SessionManagement>,
    api: Arc<tokio::sync::RwLock<dyn ClientRest>>,
    operations: Arc<dyn shared::operations::Operations>,
}

impl Platform {
    pub async fn new() -> Self {
        let session_manager = crate::session::new_session_manager().await;
        let api = new_client_rest_api();
        let operations = shared::operations::new_operations();

        Self {
            session_manager,
            api,
            operations,
        }
    }

    pub fn session_manager(&self) -> Arc<dyn SessionManagement> {
        self.session_manager.clone()
    }

    pub fn api(&self) -> Arc<tokio::sync::RwLock<dyn ClientRest>> {
        self.api.clone()
    }

    pub fn operations(&self) -> Arc<dyn shared::operations::Operations> {
        self.operations.clone()
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
        api: Option<Arc<tokio::sync::RwLock<dyn ClientRest>>>,
        operations: Option<Arc<dyn shared::operations::Operations>>,
    ) -> Self {
        let session_manager = if let Some(sm) = session_manager {
            sm
        } else {
            crate::session::new_session_manager().await
        };
        let api = api.unwrap_or_else(|| new_client_rest_api());
        let operations = operations.unwrap_or_else(|| shared::operations::new_operations());

        Self {
            session_manager,
            api,
            operations,
        }
    }

    pub fn shutdown(&self) {
        // self.gui.shutdown();
    }
}
