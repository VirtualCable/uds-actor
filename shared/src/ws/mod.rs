use crate::ws::types::{RpcEnvelope, RpcMessage};
use axum::{http::StatusCode, Json};
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot, Notify};

pub mod client;
pub mod server;
pub mod rcptraits;
pub mod request_tracker;
pub mod types;

/// Wait for a response from the tracker (oneshot channel).
pub async fn wait_response<T>(
    rx: oneshot::Receiver<RpcMessage>,
    stop: Option<Arc<Notify>>,
) -> Result<Json<T>, StatusCode>
where
    T: TryFrom<RpcMessage>,
{
    tokio::select! {
        // External stop
        _ = async {
            if let Some(stop) = &stop {
                stop.notified().await;
            }
        }, if stop.is_some() => {
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
pub async fn wait_for_request<T>(
    rx: &mut broadcast::Receiver<RpcEnvelope<RpcMessage>>,
    stop: Option<Arc<Notify>>,
) -> Option<RpcEnvelope<T>>
where
    T: TryFrom<RpcMessage> + Clone,
{
    crate::log::debug!("Waiting for request...");
    loop {
        tokio::select! {
            // External stop
            _ = async {
                if let Some(stop) = &stop {
                    stop.notified().await;
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
                    Err(e) => {
                        crate::log::warn!("Broadcast receive error: {e}");
                        return None;
                    }
                }
            }
        }
    }
}
