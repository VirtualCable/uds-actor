// Synchronous wrappers for the async UdsBrokerApi methods.
// These helpers build a small Tokio runtime, call the async method with
// `rt.block_on(...)` and convert errors to `anyhow::Error` for easy use
// from synchronous code (for example from installers or platform code
// that doesn't use async).

use anyhow::{anyhow, Result};
use tokio::runtime::Runtime;

use super::UdsBrokerApi;
use crate::config::ActorConfiguration;
use super::types;
use super::BrokerApi;

/// Create a small Tokio runtime (current-thread) with default options.
fn create_runtime() -> Result<Runtime> {
	let rt = tokio::runtime::Builder::new_current_thread()
		.enable_all()
		.build()
		.map_err(|e| anyhow!(format!("Failed to build tokio runtime: {}", e)))?;
	Ok(rt)
}

/// Synchronous wrapper for `UdsBrokerApi::register`.
///
/// This will construct the `UdsBrokerApi` from the provided `cfg`, create a
/// runtime, run the async `register` call and return the resulting String or
/// an anyhow::Error on failure.
pub fn register(
	cfg: ActorConfiguration,
	username: &str,
	hostname: &str,
	interface: crate::operations::NetworkInterface,
	command: types::RegisterCommandData,
	os: &str,
) -> Result<String> {
	let rt = create_runtime()?;
    let loglevel = cfg.log_level;
	let api = UdsBrokerApi::new(cfg, false, None);

	let res = rt.block_on(async { api.register(username, hostname, &interface, &command, loglevel.into(), os).await });
	res.map_err(|e| anyhow!(format!("{:?}", e)))
}

/// Synchronous wrapper for `UdsBrokerApi::test`.
pub fn test(cfg: ActorConfiguration) -> Result<String> {
	let rt = create_runtime()?;
	let api = UdsBrokerApi::new(cfg, false, None);

	let res = rt.block_on(async { api.test().await });
	res.map_err(|e| anyhow!(format!("{:?}", e)))
}

/// Synchronous wrapper for `UdsBrokerApi::enumerate_authenticators`.
pub fn enumerate_authenticators(
	cfg: ActorConfiguration,
) -> Result<Vec<types::Authenticator>> {
	let rt = create_runtime()?;
	let api = UdsBrokerApi::new(cfg, false, None);

	let res = rt.block_on(async { api.enumerate_authenticators().await });
	res.map_err(|e| anyhow!(format!("{:?}", e)))
}

