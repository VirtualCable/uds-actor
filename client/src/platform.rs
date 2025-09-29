use std::sync::Arc;

use crate::{rest::api::{ClientRest, new_client_rest_api}, session::SessionManagement};

#[derive(Clone)]
pub struct Platform {
    session_manager: Arc<dyn SessionManagement>,
    api: Arc<tokio::sync::RwLock<dyn ClientRest>>,
    operations: Arc<dyn shared::operations::Operations>,
    actions: Arc<dyn shared::actions::Actions>,
}

impl Platform {
    pub fn new() -> Self {
        let session_manager = crate::session::new_session_manager();
        let api = new_client_rest_api();
        let operations = shared::operations::new_operations();
        let actions = shared::actions::new_actions();

        Self {
            session_manager,
            api,
            operations,
            actions,
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

    pub fn actions(&self) -> Arc<dyn shared::actions::Actions> {
        self.actions.clone()
    }

    // Only for tests
    #[cfg(test)]
    pub fn new_with_params(
        session_manager: Option<Arc<dyn SessionManagement>>,
        api: Option<Arc<tokio::sync::RwLock<dyn ClientRest>>>,
        operations: Option<Arc<dyn shared::operations::Operations>>,
        actions: Option<Arc<dyn shared::actions::Actions>>,
    ) -> Self {
        let session_manager =
            session_manager.unwrap_or_else(|| crate::session::new_session_manager());
        let api = api.unwrap_or_else(|| new_client_rest_api());
        let operations = operations.unwrap_or_else(|| shared::operations::new_operations());
        let actions = actions.unwrap_or_else(|| shared::actions::new_actions());

        Self {
            session_manager,
            api,
            operations,
            actions,
        }
    }
}

impl Default for Platform {
    fn default() -> Self {
        Self::new()
    }
}
