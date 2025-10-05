use crate::ws::{
    server::OutboundMsg,
    types::{RpcEnvelope, RpcMessage},
};
use axum::{Json, http::StatusCode};
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::{Notify, broadcast};

pub mod client;
pub mod server;

pub mod rcptraits;
pub mod request_tracker;
pub mod types;

pub async fn wait_response<T>(
    rx: oneshot::Receiver<RpcMessage>,
    stop: Option<Arc<Notify>>,
) -> Result<Json<T>, StatusCode>
where
    T: TryFrom<RpcMessage>,
{
    tokio::select! {
        // Cancelación externa
        _ = async {
            if let Some(stop) = &stop {
                stop.notified().await;
            }
        }, if stop.is_some() => {
            Err(StatusCode::REQUEST_TIMEOUT)
        }

        // Respuesta normal
        res = rx => {
            match res {
                Ok(msg) => match T::try_from(msg) {
                    Ok(val) => Ok(Json(val)),
                    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                },
                Err(_) => Err(StatusCode::GATEWAY_TIMEOUT), // canal roto
            }
        }
    }
}

/// Wait until receiving a `RpcEnvelope<T>` from the channel.
/// Cancels if the `stop` is triggered.
pub async fn wait_for_request<T>(
    mut rx: broadcast::Receiver<OutboundMsg>,
    stop: Option<Arc<Notify>>,
) -> Option<RpcEnvelope<T>>
where
    T: TryFrom<RpcMessage> + Clone,
{
    crate::log::debug!("Waiting for request...");
    loop {
        tokio::select! {
            _ = async {
                if let Some(stop) = &stop {
                    stop.notified().await;
                }
            }, if stop.is_some() => {
                return None;
            }

            msg = rx.recv() => {
                crate::log::debug!("Received outbound message: {:?}", msg);
                if let Ok(OutboundMsg::Json(val)) = msg
                   && let Ok(env) = serde_json::from_value::<RpcEnvelope<RpcMessage>>(val)
                   && let Ok(inner) = T::try_from(env.msg.clone()) {
                    return Some(RpcEnvelope {
                        id: env.id,
                        msg: inner,
                    });
                }
            }
        }
    }
}
