use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
};

use anyhow::Result;
use axum::{
    Extension, Router,
    body::Body,
    extract::{
        ConnectInfo,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use axum_server::tls_rustls::RustlsConfig;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use socket2::{Domain, Socket, Type};
use tokio::{
    sync::{Notify, broadcast, mpsc},
    try_join,
};

use crate::{
    log,
    ws::{
        request_tracker::RequestTracker,
        types::{Close, Ping, RpcEnvelope, RpcMessage},
    },
};

mod routes;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WsFrame {
    Json(serde_json::Value),
}

#[derive(Clone)]
pub struct ServerInfo {
    pub cert_pem: Vec<u8>,
    pub key_pem: Vec<u8>,
    pub port: u16,
    pub workers_tx: mpsc::Sender<WsFrame>, // workers → WS client
    pub workers_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<WsFrame>>>, // unique receiver
    pub wsclient_to_workers: broadcast::Sender<WsFrame>,
    pub tracker: RequestTracker,
    pub stop: Arc<Notify>,
    pub secret: String,
}

#[derive(Clone)]
pub struct ServerState {
    pub workers_tx: mpsc::Sender<WsFrame>,
    pub workers_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<WsFrame>>>,
    pub wsclient_to_workers: broadcast::Sender<WsFrame>,
    pub tracker: RequestTracker,
    pub stop: Arc<Notify>,
    pub secret: String,
}

impl From<&ServerInfo> for ServerState {
    fn from(info: &ServerInfo) -> Self {
        ServerState {
            workers_tx: info.workers_tx.clone(),
            workers_rx: info.workers_rx.clone(),
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
    match segments.first() {
        Some(&"") => {} // Root path, allow
        Some(&"actor") => {
            if segments.get(1).map(|s| s == &state.secret) != Some(true) {
                log::warn!("Invalid or missing secret in actor request");
                return Err(StatusCode::FORBIDDEN);
            }
        }
        Some(&"ws") if addr.ip().is_loopback() => {
            // Allow /ws from anywhere
        }
        Some(_) => {
            log::warn!("Invalid path: {:?}", segments);
            return Err(StatusCode::NOT_FOUND);
        }
        None => unreachable!("split() always yields at least one element"),
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
    // Clonamos lo que necesitamos del estado
    let wsclient_to_workers = state.wsclient_to_workers.clone();
    let stop = state.stop.clone();

    // El Receiver único lo tienes guardado en tu ServerState,
    // probablemente envuelto en Arc<Mutex<Receiver<WsFrame>>>
    let workers_rx = state.workers_rx.clone();

    let handle_socket = move |socket: WebSocket| {
        websocket_loop(
            socket,
            workers_rx,          // único Receiver de workers → WS client
            wsclient_to_workers, // broadcast Sender de WS client → workers
            stop,
        )
    };

    ws.on_upgrade(handle_socket)
}

pub async fn websocket_loop(
    socket: WebSocket,
    workers_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<WsFrame>>>, // unique receiver
    wsclient_to_workers: broadcast::Sender<WsFrame>,
    stop: Arc<Notify>,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Task A: WS client → workers
    let mut tx_task = {
        let wsclient_to_workers = wsclient_to_workers.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_receiver.next().await {
                let frame = match msg {
                    Message::Text(txt) => match serde_json::from_str::<serde_json::Value>(&txt) {
                        Ok(val) => WsFrame::Json(val),
                        Err(e) => {
                            log::warn!("Invalid WS JSON: {e}");
                            continue;
                        }
                    },
                    Message::Ping(data) => {
                        let envelope = RpcEnvelope {
                            id: None,
                            msg: RpcMessage::Ping(Ping(data.to_vec())),
                        };
                        WsFrame::Json(serde_json::to_value(&envelope).unwrap())
                    }
                    Message::Close(_) => {
                        let envelope = RpcEnvelope {
                            id: None,
                            msg: RpcMessage::Close(Close),
                        };
                        WsFrame::Json(serde_json::to_value(&envelope).unwrap())
                    }
                    Message::Binary(_) => {
                        log::warn!("Unexpected binary WS message");
                        continue;
                    }
                    _ => continue,
                };

                if let Err(e) = wsclient_to_workers.send(frame) {
                    log::warn!("Failed to broadcast WS->workers: {e}");
                    break;
                }
            }
        })
    };

    // Task B: workers → WS client
    let mut rx_task = {
        let workers_rx = workers_rx.clone();
        tokio::spawn(async move {
            loop {
                let msg_opt = { workers_rx.lock().await.recv().await };
                let Some(WsFrame::Json(val)) = msg_opt else {
                    break;
                };

                match serde_json::to_string(&val) {
                    Ok(txt) => {
                        if ws_sender.send(Message::Text(txt.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => log::warn!("Failed to serialize JSON: {e}"),
                }
            }
        })
    };

    tokio::select! {
        _ = &mut tx_task => { rx_task.abort(); }
        _ = &mut rx_task => { tx_task.abort(); }
        _ = stop.notified() => {
            log::info!("Stopping WebSocket loop");
            tx_task.abort();
            rx_task.abort();
        }
    }
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
