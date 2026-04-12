use crate::log;
use anyhow::Result;

pub fn register(_name: &str, _display_name: &str, _description: &str) -> Result<()> {
    log::info!("Registration not implemented");
    Ok(())
}

pub fn unregister(_name: &str) -> Result<()> {
    log::info!("Unregistration not implemented");
    Ok(())
}
