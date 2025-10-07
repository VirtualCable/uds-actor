use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
};

use anyhow::Result;
use axum::{
    Extension, Router,
    body::Body,
    extract::{
        ConnectInfo, Path,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use axum_server::tls_rustls::RustlsConfig;
use futures_util::{SinkExt, StreamExt};
use socket2::{Domain, Socket, Type};
use tokio::{
    sync::{broadcast, mpsc, Notify}, try_join
};

use crate::{log, ws::request_tracker::RequestTracker};

mod routes;

#[derive(Debug, Clone)]
pub enum WsFrame {
    Json(serde_json::Value),
    Binary(Vec<u8>), // Unexpected
    Ping(Vec<u8>),
    Close,
}

pub type ClientMsg = WsFrame;
pub type ServerMsg = WsFrame;

#[derive(Clone)]
pub struct ServerInfo {
    pub cert_pem: Vec<u8>,
    pub key_pem: Vec<u8>,
    pub port: u16,
    pub workers_to_wsclient: mpsc::Sender<ServerMsg>,
    pub wsclient_to_workers: broadcast::Sender<ClientMsg>,
    pub tracker: RequestTracker,
    pub stop: Arc<Notify>,
    pub secret: String,
}

#[derive(Clone)]
pub struct ServerState {
    pub workers_to_wsclient: mpsc::Sender<ServerMsg>,
    pub wsclient_to_workers: broadcast::Sender<ClientMsg>,
    pub tracker: RequestTracker,
    pub stop: Arc<Notify>,
    pub secret: String,
}

impl From<&ServerInfo> for ServerState {
    fn from(info: &ServerInfo) -> Self {
        ServerState {
            workers_to_wsclient: info.workers_to_wsclient.clone(),
            wsclient_to_workers: info.wsclient_to_workers.clone(),
            tracker: info.tracker.clone(),
            stop: info.stop.clone(),
            secret: info.secret.clone(),
        }
    }
}

/// Middleware for verifying the secret in the path
/// and restricting access to localhost for non-actor routes
/// Also, ensures Actor Version is set in headers
async fn check_secret(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(state): Extension<ServerState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, axum::http::StatusCode> {
    // If actor is "actor"
    let path = req.uri().path();
    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if segments.first() == Some(&"actor") {
        if segments.get(1).map(|s| s == &state.secret) != Some(true) {
            log::warn!("Invalid or missing secret in actor request");
            return Err(StatusCode::FORBIDDEN);
        }
    } else if segments.first() == Some(&"ws") && !addr.ip().is_loopback() {  // Allow / from anywhere
        log::warn!("Non-localhost request without actor prefix");
        return Err(StatusCode::FORBIDDEN);
    } else if !segments.is_empty() {
        log::warn!("Invalid path: {:?}", segments);
        return Err(StatusCode::NOT_FOUND);
    }

    let mut response = next.run(req).await;

    // Añadir cabecera personalizada
    response
        .headers_mut()
        .insert("Actor-Version", HeaderValue::from_static("1.0"));
    Ok(response)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<ServerState>,
) -> impl IntoResponse {
    let handle_socket = move |socket: WebSocket| {
        websocket_loop(socket, state.workers_to_wsclient.clone(), state.wsclient_to_workers.clone())
    };
    ws.on_upgrade(handle_socket)
}

async fn websocket_loop(
    socket: WebSocket,
    workers_to_wsclient: mpsc::Sender<ClientMsg>,
    wsclient_to_workers: broadcast::Sender<ServerMsg>,
) {
    // let (mut sender, mut receiver) = socket.split();
    // let mut outbound_rx = wsclient_to_workers.subscribe();

    // // Forward outbound messages
    // let forward = tokio::spawn(async move {
    //     while let Ok(msg) = outbound_rx.recv().await {
    //         match msg {
    //             ServerMsg::Json(val) => {
    //                 if let Ok(txt) = serde_json::to_string(&val) {
    //                     let _ = sender.send(Message::Text(txt.into())).await;
    //                 }
    //             }
    //             ServerMsg::Binary(data) => {
    //                 let _ = sender.send(Message::Binary(data.into())).await;
    //             }
    //             ServerMsg::Ping(data) => {
    //                 let _ = sender.send(Message::Ping(data.into())).await;
    //             }
    //             ServerMsg::Close => {
    //                 let _ = sender.send(Message::Close(None)).await;
    //                 break;
    //             }
    //         }
    //     }
    // });

    // // Read inbound messages
    // while let Some(Ok(msg)) = receiver.next().await {
    //     match msg {
    //         Message::Text(text) => match serde_json::from_str::<serde_json::Value>(&text) {
    //             Ok(val) => {
    //                 let _ = inbound_tx.send(ClientMsg::Json(val)).await;
    //             }
    //             Err(e) => {
    //                 log::error!("Invalid JSON from client: {e}, raw: {text}");
    //             }
    //         },
    //         Message::Binary(bin) => {
    //             log::error!("Unexpected binary message: {bin:?}");
    //             // But anyway send it to processing
    //             let _ = inbound_tx.send(ClientMsg::Binary(bin.to_vec())).await;
    //         }
    //         Message::Ping(bytes) | Message::Pong(bytes) => {
    //             log::debug!("Received ping/pong: {:?}", bytes);
    //             let _ = inbound_tx.send(ClientMsg::Ping(bytes.to_vec())).await;
    //         }
    //         Message::Close(_) => {
    //             let _ = inbound_tx.send(ClientMsg::Close).await;
    //             break;
    //         }
    //     }
    // }

    // forward.abort();
}

pub async fn server(config: &ServerInfo) -> Result<()> {
    log::debug!("Initializing server {}", config.port);
    let state = ServerState::from(config);

    let tls_config = RustlsConfig::from_pem(config.cert_pem.clone(), config.key_pem.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create RustlsConfig: {}", e))?;
    log::debug!("TLS configuration loaded");

    let handle = axum_server::Handle::new();
    let handle_stop = config.stop.clone();

    let app = Router::new()
        .merge(routes::routes())
        .route("/ws", get(ws_handler))
        .route_layer(middleware::from_fn(check_secret))
        .layer(Extension(state));

    // Graceful shutdown on notify
    tokio::spawn({
        let handle = handle.clone();
        async move {
            handle_stop.notified().await;
            log::info!("Stop signal received, shutting down server...");
            handle.graceful_shutdown(None);
        }
    });

    // Helper para IPv6 only
    fn bind_ipv6_only(port: u16) -> std::io::Result<std::net::TcpListener> {
        let socket = Socket::new(Domain::IPV6, Type::STREAM, None)?;
        socket.set_only_v6(true)?; // <- no dual-stack
        socket.set_reuse_address(true)?;
        socket.bind(&SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port).into())?;
        socket.listen(128)?;
        Ok(socket.into())
    }

    // IPv4 listener
    let listener_v4: std::net::TcpListener =
        std::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, config.port))?;
    listener_v4.set_nonblocking(true)?;

    // IPv6 listener
    let listener_v6 = bind_ipv6_only(config.port)?;
    listener_v6.set_nonblocking(true)?;

    let svc = app.into_make_service_with_connect_info::<SocketAddr>();

    // Ojo: aquí usamos from_tcp_rustls en lugar de bind_rustls
    let server_v4 = axum_server::from_tcp_rustls(listener_v4, tls_config.clone())
        .handle(handle.clone())
        .serve(svc.clone());

    let server_v6 = axum_server::from_tcp_rustls(listener_v6, tls_config)
        .handle(handle)
        .serve(svc);

    try_join!(server_v4, server_v6)?;

    Ok(())
}

#[cfg(test)]
mod test_certs;

#[cfg(test)]
mod tests;
