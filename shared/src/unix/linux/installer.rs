use crate::log;

pub fn register(name: &str, display_name: &str, description: &str) -> windows::core::Result<()> {
    log::info!("Registration not implemented");
    Ok(())
}

pub fn unregister(name: &str) -> windows::core::Result<()> {
    log::info!("Unregistration not implemented");
    Ok(())
}
