use tokio::process::Command;

use anyhow::{Context, Result};

use shared::{
    log,
    utils::network::{network_interfaces_changed, network_interfaces_in_subnet},
};

use crate::platform;

pub async fn wait_for_readyness(platform: &platform::Platform) -> Result<()> {
    // We need some network interface to be up and have an IP address in the configured subnet (if any)
    let subnet = platform.config().read().await.restrict_net.clone();
    let stop = platform.get_stop();
    loop {
        if !network_interfaces_in_subnet(platform.operations(), subnet.as_deref())
            .await?
            .is_empty()
        {
            break;
        }

        // wait_timeout returns Err if timeout elapsed
        if let Ok(()) = stop.wait_timeout(std::time::Duration::from_secs(3)).await {
            log::info!("Stop signal received, exiting wait");
            return Ok(());
        }
    }

    // Also, wait for any installation in progress to complete
    loop {
        if !platform.operations().is_some_installation_in_progress()? {
            break;
        }
        // wait_timeout returns Err if timeout elapsed
        if let Ok(()) = stop.wait_timeout(std::time::Duration::from_secs(3)).await {
            log::info!("Stop signal received, exiting wait");
            break;
        }
    }

    Ok(())
}

// Invokes initialization and updates config accordingly
pub async fn initialize(platform: &platform::Platform) -> Result<()> {
    let cfg_guard = platform.config();

    let mut cfg_guard = cfg_guard.write().await;

    let broker_api = platform.broker_api(); // Avoid drop borrow
    let mut broker_api_guard = broker_api.write().await;
    let interfaces = platform.operations().get_network_info()?;
    // Initialize
    let master_token = cfg_guard.master_token.clone().unwrap_or_default();
    broker_api_guard.set_token(&master_token);
    log::info!("Unmanaged actor not initialized, initializing with broker");
    if let Ok(response) = broker_api_guard.initialize(interfaces.as_slice()).await {
        // If token on response is none, this is not a managed host,continue until next request
        if response.token.is_none() {
            log::error!(
                "Unmanaged actor initialization did not return a token, cannot continue login"
            );
            return Err(anyhow::anyhow!(
                "Unmanaged actor initialization did not return a token"
            ));
        }

        // If master token is present on response, and is different of current, update it
        if let Some(master_token) = response.master_token
            && cfg_guard.master_token.as_ref() != Some(&master_token)
        {
            log::info!("Master token updated from broker");
            cfg_guard.master_token = Some(master_token);
        }

        cfg_guard.own_token = response.token;
        cfg_guard.config.unique_id = response.unique_id;
        cfg_guard.config.os = response.os;

        // Update stored config.
        // Note that in fact, on unmanaged, we do not need to store own_token or unique_id,
        // because it's volatile, but we do it anyway for simplicity as it really does not harm
        let mut saver = platform.config_storage();
        if let Err(e) = saver.save_config(&cfg_guard) {
            log::error!("Failed to save updated config with new master_token: {}", e);
            // Continue anyway, we have the token in our in-memory config
        }

        // Now, set the broker_api token to the new own_token
        if let Some(own_token) = cfg_guard.own_token.clone() {
            broker_api_guard.set_token(&own_token);
        }
    }
    Ok(())
}

// Watch for interface ip changes
// On current implementation, we simply stop the service
// And the system (Windows, systemd, launchd) will restart it
// It's cleaner and simpler than trying to restart the webserver in place
pub async fn interfaces_watch_task(
    platform: &platform::Platform,
    subnet: Option<String>,
) -> Result<()> {
    // Store existing network interface, to watch for changes
    let known_interfaces =
        network_interfaces_in_subnet(platform.operations(), subnet.as_deref()).await?;

    let stop = platform.get_stop();
    loop {
        if let Ok(interfaces) = network_interfaces_changed(
            platform.operations(),
            known_interfaces.as_slice(),
            subnet.as_deref(),
        )
        .await
            && !interfaces.is_empty()
        {
            platform
                .get_restart_flag()
                .store(true, std::sync::atomic::Ordering::Relaxed);
            log::warn!(
                "Network interfaces changed (IP change, new interface, etc), stopping service to allow restart"
            );
            stop.set(); // Signal stop
            break;
        }
        // Wait for 10 seconds or stop signal
        // wait_timeout returns Ok if signaled, Err if timeout elapsed
        if stop
            .wait_timeout(std::time::Duration::from_secs(10))
            .await
            .is_ok()
        {
            break;
        }
    }
    Ok(())
}

pub async fn run_command(info_name: &str, command: &str, args: &[&str]) -> Result<()> {
    log::debug!("Running command {}: {} {:?}", info_name, command, args);
    // If empty pre_command, do nothing
    if command.trim().is_empty() {
        return Ok(());
    }
    // Use shlex to split command into command + args, and append extra args (args)
    let mut parts = shlex::split(command)
        .with_context(|| format!("failed to parse {} command line: {}", info_name, command))?;
    parts.extend(args.iter().map(|s| s.to_string()));
    if parts.is_empty() {
        return Ok(());
    }
    let status = Command::new(parts[0].as_str())
        .args(&parts[1..])
        .status()
        .await
        .with_context(|| format!("failed to execute {}: {}", info_name, command))?;

    if !status.success() {
        anyhow::bail!("{} exited with status: {:?}", info_name, status.code());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::testing::dummy;

    use super::*;

    #[tokio::test]
    async fn test_initialize() {
        log::setup_logging("debug", shared::log::LogType::Tests);
        let (platform, calls) = dummy::create_dummy_platform().await;
        platform.config().write().await.master_token = Some("mastertoken".into());
        let result = initialize(&platform).await;
        assert!(result.is_ok());
        // Inspect dummy broker_api
        log::info!("calls: {:?}", calls.dump());
    }

    #[tokio::test]
    async fn test_interfaces_watch() {
        log::setup_logging("debug", shared::log::LogType::Tests);
        let (platform, calls) = dummy::create_dummy_platform().await;
        let subnet = platform.config().read().await.restrict_net.clone();
        let stop = platform.get_stop();
        let handle = tokio::spawn(async move {
            let result = interfaces_watch_task(&platform, subnet).await;
            assert!(result.is_ok());
        });

        // Wait a bit and then signal stop
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        // Stop should not be set yet
        assert!(!stop.is_set());

        stop.set();
        let _ = handle.await;

        log::info!("calls: {:?}", calls.dump());
        // Should have called operations::get_network_info at least twice
        assert!(calls.count_calls("operations::get_network_info()") >= 2);
    }

    #[tokio::test]
    #[cfg(target_family = "unix")]
    async fn test_run_command_unix() {
        log::setup_logging("debug", shared::log::LogType::Tests);
        // Simple command
        let result = run_command("test ls", "ls", &["-la"]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[cfg(target_family = "windows")]
    async fn test_run_command_windows() {
        log::setup_logging("debug", shared::log::LogType::Tests);
        // Simple command
        let result = run_command("test dir", "cmd.exe", &["/C", "dir"]).await;
        assert!(result.is_ok());
    }
}