use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    sync::Arc,
};

use anyhow::Result;
use axum::{
    Extension, Router,
    extract::{
        ConnectInfo,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use axum_server::tls_rustls::RustlsConfig;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{Notify, broadcast, mpsc};

use crate::{log, ws::request_tracker::RequestTracker};

mod routes;

#[derive(Debug, Clone)]
pub enum InboundMsg {
    Json(serde_json::Value),
    Binary(Vec<u8>), // Unexpected
    Ping(Vec<u8>),
    Pong(Vec<u8>), // Usually not needed
    Close,
}

#[derive(Debug, Clone)]
pub enum OutboundMsg {
    Json(serde_json::Value),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close,
}

#[derive(Clone)]
pub struct ServerState {
    pub inbound_tx: mpsc::Sender<InboundMsg>,
    pub outbound_tx: broadcast::Sender<OutboundMsg>,
    pub tracker: RequestTracker,
    pub stop: Arc<Notify>,
}

async fn ws_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ws: WebSocketUpgrade,
    Extension(state): Extension<ServerState>,
) -> impl IntoResponse {
    let handle_socket = move |socket: WebSocket| {
        websocket_loop(socket, state.inbound_tx.clone(), state.outbound_tx.clone())
    };
    match addr.ip() {
        IpAddr::V4(ip) if ip.is_loopback() => ws.on_upgrade(handle_socket),
        IpAddr::V6(ip) if ip.is_loopback() => ws.on_upgrade(handle_socket),
        _ => (axum::http::StatusCode::FORBIDDEN, "Forbidden").into_response(),
    }
}

async fn websocket_loop(
    socket: WebSocket,
    inbound_tx: mpsc::Sender<InboundMsg>,
    outbound_tx: broadcast::Sender<OutboundMsg>,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut outbound_rx = outbound_tx.subscribe();

    // Forward outbound messages
    let forward = tokio::spawn(async move {
        while let Ok(msg) = outbound_rx.recv().await {
            match msg {
                OutboundMsg::Json(val) => {
                    if let Ok(txt) = serde_json::to_string(&val) {
                        let _ = sender.send(Message::Text(txt.into())).await;
                    }
                }
                OutboundMsg::Ping(data) => {
                    let _ = sender.send(Message::Ping(data.into())).await;
                }
                OutboundMsg::Pong(data) => {
                    let _ = sender.send(Message::Pong(data.into())).await;
                }
                OutboundMsg::Close => {
                    let _ = sender.send(Message::Close(None)).await;
                    break;
                }
            }
        }
    });

    // Read inbound messages
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(val) => {
                    let _ = inbound_tx.send(InboundMsg::Json(val)).await;
                }
                Err(e) => {
                    log::error!("Invalid JSON from client: {e}, raw: {text}");
                }
            },
            Message::Binary(bin) => {
                log::error!("Unexpected binary message: {bin:?}");
                // Drop, leaving commented the send
                // let _ = inbound_tx.send(InboundMsg::Binary(bin.to_vec())).await;
            }
            Message::Ping(bytes) => {
                //  Respond with Pong with same payload
                let _ = outbound_tx.send(OutboundMsg::Pong(bytes.to_vec()));
                // And process
                let _ = inbound_tx.send(InboundMsg::Ping(bytes.to_vec())).await;
            }
            Message::Pong(bytes) => {
                // optionally handle Pong, we can keep alive timer if needed
                log::debug!("Received Pong from client");
                let _ = inbound_tx.send(InboundMsg::Pong(bytes.to_vec())).await;
            }
            Message::Close(_) => {
                let _ = inbound_tx.send(InboundMsg::Close).await;
                break;
            }
        }
    }

    forward.abort();
}

pub async fn server(
    cert_pem: &[u8],
    key_pem: &[u8],
    port: u16,
    inbound_tx: mpsc::Sender<InboundMsg>,
    outbound_tx: broadcast::Sender<OutboundMsg>,
    tracker: RequestTracker,
    stop: std::sync::Arc<tokio::sync::Notify>,
) -> Result<()> {
    log::debug!("Initializing server {}", port);
    let config = RustlsConfig::from_pem(cert_pem.to_vec(), key_pem.to_vec())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create RustlsConfig: {}", e))?;
    log::debug!("TLS configuration loaded");

    let handle = axum_server::Handle::new();
    let handle_stop = stop.clone();

    let state = ServerState {
        inbound_tx,
        outbound_tx,
        tracker,
        stop,
    };

    let app = Router::new()
        .merge(routes::routes())
        .route("/ws", get(ws_handler))
        .layer(Extension(state));

    let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port);
    log::info!("Starting Web server on https://{}", addr);

    tokio::spawn({
        let handle = handle.clone();
        async move {
            handle_stop.notified().await;
            log::info!("Stop signal received, shutting down server...");
            handle.graceful_shutdown(None);
        }
    });

    axum_server::bind_rustls(addr, config)
        .handle(handle)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start server: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod test_certs;

#[cfg(test)]
mod tests;
