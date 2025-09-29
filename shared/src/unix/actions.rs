use crate::log;
use std::sync::Arc;
use async_trait::async_trait;
use crate::actions::Actions;

pub struct UnixActions;

#[async_trait]
impl Actions for UnixActions {
    async fn logoff(&self) -> anyhow::Result<()> {
        log::info!("Logoff requested (stub)");
        // TODO: Close user session on Unix (stub)
        Ok(())
    }

    async fn screenshot(&self) -> anyhow::Result<Vec<u8>> {
        log::info!("Screenshot requested (stub)");
        // TODO: Implement screenshot on Unix. Return 1x1 transparent PNG for now.
        const PNG_1X1_TRANSPARENT: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
            0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41,
            0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
            0x42, 0x60, 0x82
        ];
        Ok(PNG_1X1_TRANSPARENT.to_vec())
    }

    async fn run_script(&self, _script: &str) -> anyhow::Result<String> {
        // TODO: Execute script on Unix (stub)
        Ok("".to_string())
    }

}

pub fn new_actions() -> Arc<impl Actions> {
    Arc::new(UnixActions)
}
