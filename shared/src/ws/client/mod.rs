
use futures_util::{StreamExt, SinkExt};
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::{Connector, tungstenite::protocol::Message};

use crate::{
    log,
    ws::types::{Close, RpcEnvelope, RpcMessage},
};

pub struct WsClient {
    pub from_ws: broadcast::Sender<RpcEnvelope<RpcMessage>>,
    pub to_ws: mpsc::Sender<RpcEnvelope<RpcMessage>>,
}

/// Connects to a local WebSocket server over TLS and spawns a reader and a writer task.
/// Every incoming message is parsed into a typed RpcMessage and forwarded into a broadcast channel.
///
/// # Arguments
/// * `port` - Local port where the WebSocket server is listening.
/// * `capacity` - Maximum buffer size of the broadcast channel (e.g. 32 or 64).
///
/// # Returns
/// A `WsClient` instance that can be used to send and receive messages.
pub async fn websocket_client_tasks(port: u16, capacity: usize) -> WsClient {
    let (from_ws, _rx) = broadcast::channel::<RpcEnvelope<RpcMessage>>(capacity);
    let (to_ws, mut from_clients) = mpsc::channel::<RpcEnvelope<RpcMessage>>(capacity);

    let connector = Connector::Rustls(crate::tls::noverify::client_config());
    let url = format!("wss://localhost:{}/ws", port);

    let (ws_stream, _resonse) =
        tokio_tungstenite::connect_async_tls_with_config(url, None, true, Some(connector))
            .await
            .expect("WS connect failed");

    let (mut write, mut read) = ws_stream.split();        

    // Receiver task, from websocket to broadcast
    tokio::spawn({
        let from_ws = from_ws.clone();
        async move {
            while let Some(msg) = read.next().await {
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
                        log::warn!("Binary frame received, ignored.");
                        continue;
                    }
                    Ok(Message::Close(_)) => RpcEnvelope {
                        id: None,
                        msg: RpcMessage::Close(Close),
                    },
                    Ok(Message::Ping(data)) => RpcEnvelope {
                        id: None,
                        msg: RpcMessage::Ping(crate::ws::types::Ping(data.to_vec())),
                    },
                    _ => continue,
                };
                if let Err(e) = from_ws.send(env) {
                    log::warn!("Failed to broadcast WS message: {e}");
                    break;
                }
            }
        }
    });

    // Sender task, from client to websocket
    tokio::spawn({
        async move {
            while let Some(env) = from_clients.recv().await {
                let msg_text = match serde_json::to_string(&env) {
                    Ok(txt) => txt,
                    Err(e) => {
                        log::warn!("Failed to serialize WS message: {e}");
                        continue;
                    }
                };
                if let Err(e) = write.send(Message::Text(msg_text.into())).await {
                    log::warn!("Failed to send WS message: {e}");
                    break;
                }
            }
        }
    });

    WsClient {
        from_ws,
        to_ws,
    }
}
