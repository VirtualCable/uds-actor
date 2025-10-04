use tokio::sync::broadcast;
use futures_util::StreamExt;
use tokio_tungstenite::{
    tungstenite::protocol::Message,
    Connector,
};

use crate::log;

pub mod request_state;
pub mod resolver;
pub mod types;

/// Generic frame pushed into the internal broadcast bus.
/// You can later replace `raw` with a typed enum if needed.
#[derive(Debug, Clone)]
pub struct Frame {
    pub raw: String,
}

/// Connects to a local WebSocket server over TLS and spawns a reader task.
/// Every incoming message is forwarded into a broadcast channel.
///
/// # Arguments
/// * `port` - Local port where the WebSocket server is listening.
/// * `capacity` - Maximum buffer size of the broadcast channel (e.g. 32 or 64).
///
/// # Returns
/// A `broadcast::Sender<Frame>` that can be cloned and subscribed to by other modules.
pub async fn ws_processor(
    port: u16,
    capacity: usize,
) -> broadcast::Sender<Frame> {
    let (tx, _rx) = broadcast::channel::<Frame>(capacity);

    let connector = Connector::Rustls(crate::tls::noverify::client_config());
    let url = format!("wss://localhost:{}/ws", port);

    let (mut ws, _) = tokio_tungstenite::connect_async_tls_with_config(
        url,
        None,
        true,
        Some(connector),
    )
    .await
    .expect("WS connect failed");

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        while let Some(msg) = ws.next().await {
            match msg {
                Ok(Message::Text(txt)) => {
                    let _ = tx_clone.send(Frame { raw: txt.to_string() });
                }
                Ok(Message::Binary(bin)) => {
                    let s = String::from_utf8_lossy(&bin).to_string();
                    let _ = tx_clone.send(Frame { raw: s });
                }
                Ok(Message::Close(_)) => break,
                Err(e) => {
                    log::error!("WebSocket error: {e}");
                    break;
                }
                _ => {}
            }
        }
    });

    tx
}
