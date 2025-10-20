use axum::{Json, http::StatusCode};
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot};

use crate::{
    log, sync::OnceSignal, ws::types::{RpcEnvelope, RpcMessage}
};

pub mod client;
pub mod rcptraits;
pub mod request_tracker;
pub mod server;
pub mod types;

/// Wait for a response from the tracker (oneshot channel).
pub async fn wait_response<T>(
    rx: oneshot::Receiver<RpcMessage>,
    stop: Option<Arc<OnceSignal>>,
    timeout: Option<std::time::Duration>,
) -> Result<Json<T>, StatusCode>
where
    T: TryFrom<RpcMessage>,
{
    tokio::select! {
        // External stop
        _ = async {
            if let Some(stop) = &stop {
                stop.wait().await;
            }
        }, if stop.is_some() => {
            Err(StatusCode::REQUEST_TIMEOUT)
        }

        // Timeout
        _ = tokio::time::sleep(timeout.unwrap()), if timeout.is_some() => {
            Err(StatusCode::REQUEST_TIMEOUT)
        }

        // Normal response
        res = rx => {
            match res {
                Ok(msg) => match T::try_from(msg) {
                    Ok(val) => Ok(Json(val)),
                    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                },
                Err(_) => Err(StatusCode::GATEWAY_TIMEOUT), // Broken channel
            }
        }
    }
}

/// Wait until receiving a `RpcEnvelope<T>` from the broadcast channel.
/// Cancels if the `stop` is triggered.
pub async fn wait_message_arrival<T>(
    rx: &mut broadcast::Receiver<RpcEnvelope<RpcMessage>>,
    stop: Option<Arc<OnceSignal>>,
) -> Option<RpcEnvelope<T>>
where
    T: TryFrom<RpcMessage> + Clone,
{
    log::debug!("Waiting for request of type {}", std::any::type_name::<T>());
    loop {
        tokio::select! {
            // External stop
            _ = async {
                if let Some(stop) = &stop {
                    stop.wait().await;
                }
            }, if stop.is_some() => {
                return None;
            }

            // Normal reception
            msg = rx.recv() => {
                match msg {
                    Ok(env) => {
                        if let Ok(inner) = T::try_from(env.msg.clone()) {
                            return Some(RpcEnvelope {
                                id: env.id,
                                msg: inner,
                            });
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        // Skipping messages
                        log::warn!("Skipped {} messages", count);
                    }
                    Err(e) => {
                        log::warn!("Broadcast receive error: {e}");
                        return None;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::types::Ping;

    #[tokio::test]
    async fn wait_for_request_survives_lagged() {
        // Small broadcast channel to force Lagged
        let (tx, mut _rx0) = broadcast::channel::<RpcEnvelope<RpcMessage>>(2);

        // New receiver for the test
        let mut rx = tx.subscribe();

        // Send more messages than can fit in the buffer
        for _i in 0..10 {
            let msg = RpcEnvelope {
                id: None,
                msg: RpcMessage::Ping(Ping(Vec::new())),
            };
            let _ = tx.send(msg);
        }

        // No stop signal
        let stop: Option<Arc<OnceSignal>> = None;

        // Call wait_for_request it should skip Lagged messages
        // and return the first Ping it can parse
        // Note: we sent 10 messages but the buffer is only 2, so
        // it should have skipped some
        let env = wait_message_arrival::<Ping>(&mut rx, stop).await;

        assert!(env.is_some());
    }
}
