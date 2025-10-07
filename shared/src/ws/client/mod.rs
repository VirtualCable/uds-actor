use futures_util::StreamExt;
use tokio::sync::broadcast;
use tokio_tungstenite::{Connector, tungstenite::protocol::Message};

use crate::{
    log,
    ws::types::{Close, RpcEnvelope, RpcMessage},
};

/// Connects to a local WebSocket server over TLS and spawns a reader task.
/// Every incoming message is parsed into a typed RpcMessage and forwarded into a broadcast channel.
///
/// # Arguments
/// * `port` - Local port where the WebSocket server is listening.
/// * `capacity` - Maximum buffer size of the broadcast channel (e.g. 32 or 64).
///
/// # Returns
/// A `broadcast::Sender<RpcEnvelope<RpcMessage>>` that can be cloned and subscribed to by other modules.
pub async fn websocket_task(port: u16, capacity: usize) -> broadcast::Sender<RpcEnvelope<RpcMessage>> {
    let (ws_to_client, _rx) = broadcast::channel::<RpcEnvelope<RpcMessage>>(capacity);

    let connector = Connector::Rustls(crate::tls::noverify::client_config());
    let url = format!("wss://localhost:{}/ws", port);

    let (mut ws, _) =
        tokio_tungstenite::connect_async_tls_with_config(url, None, true, Some(connector))
            .await
            .expect("WS connect failed");

    let tx_clone = ws_to_client.clone();
    tokio::spawn(async move {
        while let Some(msg) = ws.next().await {
            let env = match msg {
                Ok(Message::Text(txt)) => {
                    if let Ok(env) = serde_json::from_str::<RpcEnvelope<RpcMessage>>(&txt) {
                        env
                    } else {
                        log::warn!("Invalid WS JSON: {txt}");
                        continue;
                    }
                }
                Ok(Message::Binary(_bin)) => {
                    // Not supported, log and skip
                    log::warn!("Binary frame received, attempting to parse as UTF-8");
                    continue;
                }
                Ok(Message::Close(_)) => RpcEnvelope { id: None, msg: RpcMessage::Close(Close) },
                Ok(Message::Ping(data)) => RpcEnvelope { id: None, msg: RpcMessage::Ping(crate::ws::types::Ping(data.to_vec())) },
                _ => continue,
            };
            if let Err(e) = tx_clone.send(env) {
                log::warn!("Failed to broadcast WS message: {e}");
                break;
            }
        }
    });

    ws_to_client
}
