use std::sync::Arc;

use crate::{rest::api::ClientRestApi, session::SessionManagement};

#[derive(Clone)]
pub struct Platform {
    session_manager: Arc<dyn SessionManagement + Send + Sync>,
    api: ClientRestApi,
    operations: Arc<dyn shared::operations::Operations + Send + Sync>,
}

impl Platform {
    pub fn new() -> Self {
        let session_manager = crate::session::new_session_manager();
        let api = ClientRestApi::new("https://127.0.0.1:43910", false);
        let operations = shared::operations::new_operations();

        Self {
            session_manager,
            api,
            operations,
        }
    }

    pub fn session_manager(&self) -> Arc<dyn SessionManagement + Send + Sync> {
        self.session_manager.clone()
    }

    pub fn api(&self) -> ClientRestApi {
        self.api.clone()
    }

    pub fn operations(&self) -> Arc<dyn shared::operations::Operations + Send + Sync> {
        self.operations.clone()
    }

    // Only for tests
    #[cfg(test)]
    pub fn new_with_params(
        session_manager: Option<Arc<dyn SessionManagement + Send + Sync>>,
        api: Option<ClientRestApi>,
        operations: Option<Arc<dyn shared::operations::Operations + Send + Sync>>,
    ) -> Self {
        let session_manager =
            session_manager.unwrap_or_else(|| crate::session::new_session_manager());
        let api = api.unwrap_or_else(|| ClientRestApi::new("https://127.0.0.1:43910", false));
        let operations = operations.unwrap_or_else(|| shared::operations::new_operations());

        Self {
            session_manager,
            api,
            operations,
        }
    }
}

impl Default for Platform {
    fn default() -> Self {
        Self::new()
    }
}
