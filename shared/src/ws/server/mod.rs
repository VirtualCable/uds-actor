use std::net::{IpAddr, Ipv6Addr, SocketAddr};

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
use tokio::sync::broadcast;

use crate::log;

async fn ws_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ws: WebSocketUpgrade,
    Extension(tx): Extension<broadcast::Sender<String>>,
) -> impl IntoResponse {
    let handle_socket = move |socket: WebSocket| websocket_loop(socket, tx);
    match addr.ip() {
        IpAddr::V4(ip) if ip.is_loopback() => {
            // Permitimos solo loopback IPv4
            ws.on_upgrade(handle_socket)
        }
        IpAddr::V6(ip) if ip.is_loopback() => {
            // Permitimos loopback IPv6 (::1)
            ws.on_upgrade(handle_socket)
        }
        _ => {
            // Cualquier otra IP → 403 Forbidden
            (axum::http::StatusCode::FORBIDDEN, "Forbidden").into_response()
        }
    }
}

async fn websocket_loop(socket: WebSocket, tx: broadcast::Sender<String>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = tx.subscribe();

    // Task for broadcasting messages to the client
    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let _ = sender.send(Message::Text(msg.into())).await;
        }
    });

    // Bucle para leer mensajes del cliente
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            log::debug!("Cliente envió: {}", text);
            if text == "ping" {
                // En vez de responder directamente, lo mandamos al canal
                let _ = tx.send("pong".to_string());
            }
        }
    }
}

async fn ping() -> &'static str {
    "pong"
}

pub async fn server(
    cert_pem: &[u8],
    key_pem: &[u8],
    port: u16,
    tx: broadcast::Sender<String>,
) -> Result<()> {
    let config = RustlsConfig::from_pem(cert_pem.to_vec(), key_pem.to_vec())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create RustlsConfig: {}", e))?;

    let app = Router::new()
        .route("/ping", get(ping))
        .route("/ws", get(ws_handler))
        .layer(Extension(tx));

    let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port);
    log::info!("Starting Web server on https://{}", addr);

    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start server: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod test_certs;

#[cfg(test)]
mod tests;
