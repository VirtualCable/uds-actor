use axum::Json;
use tokio::sync::oneshot;

pub mod client;
pub mod server;

pub mod request_tracker;
pub mod types;
mod rcptraits;

pub async fn wait_response<T>(rx: oneshot::Receiver<types::RpcMessage>) -> Result<Json<T>, axum::http::StatusCode>
where
    T: TryFrom<types::RpcMessage>,
{
    match rx.await {
        Ok(msg) => match T::try_from(msg) {
            Ok(val) => Ok(Json(val)),
            Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        },
        Err(_) => Err(axum::http::StatusCode::GATEWAY_TIMEOUT), // Broken channel
    }
}
