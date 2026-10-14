use anyhow::Result;

use shared::{
    log,
    ws::{server::ServerContext, types::PreConnect, wait_message_arrival},
};

use crate::{computer, platform};

// Owned ServerInfo and Platform
pub async fn worker(server_info: ServerContext, platform: platform::Platform) -> Result<()> {
    let mut rx = server_info.from_ws.subscribe();
    while let Some(env) =
        wait_message_arrival::<PreConnect>(&mut rx, Some(platform.get_stop())).await
    {
        log::debug!("Received PreConnect: {:?}", env.msg);
        // Process the Preconnect. If protocol is rdp, ensure the user can rdp
        let msg = env.msg;
        if msg.protocol.to_lowercase() == "rdp" {
            if let Err(e) = platform.system().ensure_user_can_rdp(&msg.user) {
                log::error!("Failed to ensure user can RDP: {}", e);
            } else {
                log::info!("Ensured user can RDP: {}", msg.user);
            }
        }
        // Same positional parameters, order and "unknown" defaults as the 4.0 actor,
        // so existing preconnect scripts keep working.
        let args = [
            msg.user.as_str(),
            msg.protocol.as_str(),
            msg.ip.as_deref().unwrap_or("unknown"),
            msg.hostname.as_deref().unwrap_or("unknown"),
            msg.udsuser.as_deref().unwrap_or("unknown"),
        ];
        computer::process_command(&platform, computer::CommandType::PreConnect, &args).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::mock;
    use std::time::Duration;

    use shared::ws::types::{RpcEnvelope, RpcMessage};

    #[tokio::test]
    async fn test_preconnect_worker() {
        log::setup_logging("debug", shared::log::LogType::Tests);
        let server_info = mock::mock_server_info().await;
        let mocked_platform = mock::mock_platform().await;
        let platform = mocked_platform.platform.clone();
        let calls = mocked_platform.calls.clone();
        platform.config().write().await.master_token = Some("mastertoken".into());

        let wsclient_to_workers = server_info.from_ws.clone();

        let _handle = tokio::spawn(async move {
            worker(server_info, platform).await.unwrap();
        });

        // Wait to have at least one receiver
        while wsclient_to_workers.receiver_count() == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        log::info!("wsclient_to_workers has receiver");

        // Send 3 logoff requests
        for _i in 0..3 {
            let req = RpcEnvelope {
                id: None,
                msg: RpcMessage::PreConnect(PreConnect {
                    user: "testuser".into(),
                    protocol: "rdp".into(),
                    ip: Some("192.168.1.1".into()),
                    hostname: Some("testhost".into()),
                    udsuser: Some("udsuser".into()),
                }),
            };
            if let Err(e) = wsclient_to_workers.send(req) {
                log::error!("Failed to send MessageRequest: {}", e);
            }
        }
        // Wait a bit to let processing happen
        tokio::time::sleep(Duration::from_millis(200)).await;

        // No calls here, only redirects messages to wsclient
        log::info!("calls: {:?}", calls.dump());
        assert!(calls.count_calls("operations::ensure_user_can_rdp(") == 3);
    }

    // Scripts written for the 4.0 actor read these positional parameters, in this order.
    #[cfg(unix)]
    #[tokio::test]
    async fn test_preconnect_command_receives_connection_parameters() {
        log::setup_logging("debug", shared::log::LogType::Tests);
        let dir = std::env::temp_dir().join(format!("udsactor-preconnect-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("preconnect.sh");
        let output = dir.join("args.txt");
        std::fs::write(
            &script,
            format!("#!/bin/sh\necho \"$@\" > '{}'\n", output.display()),
        )
        .unwrap();

        let server_info = mock::mock_server_info().await;
        let mocked_platform = mock::mock_platform().await;
        let platform = mocked_platform.platform.clone();
        platform.config().write().await.pre_command = Some(format!("/bin/sh {}", script.display()));

        let wsclient_to_workers = server_info.from_ws.clone();
        let _handle = tokio::spawn(async move {
            worker(server_info, platform).await.unwrap();
        });
        while wsclient_to_workers.receiver_count() == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let send = |ip: Option<&str>, hostname: Option<&str>, udsuser: Option<&str>| {
            wsclient_to_workers
                .send(RpcEnvelope {
                    id: None,
                    msg: RpcMessage::PreConnect(PreConnect {
                        user: "jdoe".into(),
                        protocol: "nx".into(),
                        ip: ip.map(Into::into),
                        hostname: hostname.map(Into::into),
                        udsuser: udsuser.map(Into::into),
                    }),
                })
                .unwrap();
        };
        let read_args = || async {
            for _ in 0..40 {
                if let Ok(content) = std::fs::read_to_string(&output)
                    && !content.is_empty()
                {
                    return content.trim().to_string();
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            String::new()
        };

        send(Some("10.0.0.25"), Some("laptop-01"), Some("jdoe@corp"));
        assert_eq!(read_args().await, "jdoe nx 10.0.0.25 laptop-01 jdoe@corp");

        std::fs::remove_file(&output).unwrap();
        send(None, None, None);
        assert_eq!(read_args().await, "jdoe nx unknown unknown unknown");

        std::fs::remove_dir_all(&dir).ok();
    }
}
