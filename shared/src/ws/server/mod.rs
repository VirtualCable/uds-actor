use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
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
use futures_util::{SinkExt, StreamExt};
use socket2::{Domain, Socket, Type};
use tokio::{
    sync::{broadcast, mpsc},
    try_join,
};

use crate::{
    log,
    sync::OnceSignal,
    tls::{CertificateInfo, certool},
    ws::{
        request_tracker::RequestTracker,
        types::{Close, Ping, RpcEnvelope, RpcMessage},
    },
};

mod routes;

#[derive(Clone)]
pub struct ServerContext {
    pub to_ws: mpsc::Sender<RpcEnvelope<RpcMessage>>,
    pub from_ws: broadcast::Sender<RpcEnvelope<RpcMessage>>,
    pub tracker: RequestTracker,
}

#[derive(Clone)]
struct ServerStartInfo {
    pub cert_info: CertificateInfo,
    pub port: u16,
    pub workers_to_wsclient: Arc<tokio::sync::Mutex<mpsc::Receiver<RpcEnvelope<RpcMessage>>>>, // unique receiver
    pub wsclient_to_workers: broadcast::Sender<RpcEnvelope<RpcMessage>>, // WS client → workers
    pub tracker: RequestTracker,
    pub stop: OnceSignal,
    pub secret: String,
}

#[derive(Clone)]
pub struct ServerState {
    pub workers_to_wsclient: Arc<tokio::sync::Mutex<mpsc::Receiver<RpcEnvelope<RpcMessage>>>>,
    pub wsclient_to_workers: broadcast::Sender<RpcEnvelope<RpcMessage>>,
    pub tracker: RequestTracker,
    pub stop: OnceSignal,
    pub secret: String,
    pub is_ws_active: Arc<AtomicBool>,
}

impl From<&ServerStartInfo> for ServerState {
    fn from(info: &ServerStartInfo) -> Self {
        ServerState {
            workers_to_wsclient: info.workers_to_wsclient.clone(),
            wsclient_to_workers: info.wsclient_to_workers.clone(),
            tracker: info.tracker.clone(),
            stop: info.stop.clone(),
            secret: info.secret.clone(),
            is_ws_active: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Middleware for verifying the secret in the path
async fn check_secret_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(state): Extension<ServerState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, axum::http::StatusCode> {
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
    response
        .headers_mut()
        .insert("Actor-Version", HeaderValue::from_static("1.0"));
    Ok(response)
}

async fn ws_handler(ws: WebSocketUpgrade, Extension(state): Extension<ServerState>) -> Response {
    let ws_active = state.is_ws_active.clone();
    if ws_active.swap(true, Ordering::SeqCst) {
        // ya estaba true → hay cliente activo
        log::warn!("WebSocket connection attempt while another is active");
        return (StatusCode::CONFLICT, "Another WebSocket client is active").into_response();
    }

    let wsclient_to_workers = state.wsclient_to_workers.clone();
    let stop = state.stop.clone();

    ws.on_upgrade(move |socket| {
        websocket_loop(
            socket,
            state.workers_to_wsclient,
            wsclient_to_workers,
            stop,
            ws_active,
            state.tracker.clone(),
        )
    })
}

pub async fn websocket_loop(
    socket: WebSocket,
    to_ws: Arc<tokio::sync::Mutex<mpsc::Receiver<RpcEnvelope<RpcMessage>>>>,
    from_ws: broadcast::Sender<RpcEnvelope<RpcMessage>>,
    stop: OnceSignal,
    ws_active: Arc<AtomicBool>,
    tracker: RequestTracker,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Task A: WS client → workers
    let mut tx_task = {
        let wsclient_to_workers = from_ws.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_receiver.next().await {
                log::debug!("WS client sent message: {:?}", msg);
                let env = match msg {
                    Message::Text(txt) => {
                        if let Ok(env) = serde_json::from_str::<RpcEnvelope<RpcMessage>>(&txt) {
                            env
                        } else {
                            log::warn!("Invalid WS JSON: {txt}");
                            continue;
                        }
                    }
                    Message::Ping(data) => RpcEnvelope {
                        id: None,
                        msg: RpcMessage::Ping(Ping(data.to_vec())),
                    },
                    Message::Close(_) => RpcEnvelope {
                        id: None,
                        msg: RpcMessage::Close(Close),
                    },
                    Message::Binary(_) => {
                        log::warn!("Unexpected binary");
                        continue;
                    }
                    _ => continue,
                };

                if let Some(id) = env.id {
                    tracker.resolve_ok(id, env.msg.clone()).await;
                    continue;
                }

                if let Err(e) = wsclient_to_workers.send(env) {
                    log::warn!("Failed to broadcast WS->workers: {e}");
                    break;
                }
            }
        })
    };

    // Task B: workers → WS client
    let mut rx_task = {
        let workers_rx = to_ws.clone();
        tokio::spawn(async move {
            loop {
                let msg_opt = { workers_rx.lock().await.recv().await };
                let Some(env) = msg_opt else { break };

                match serde_json::to_string(&env) {
                    Ok(txt) => {
                        if ws_sender.send(Message::Text(txt.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => log::warn!("Failed to serialize RpcEnvelope: {e}"),
                }
            }
        })
    };

    tokio::select! {
        _ = &mut tx_task => { rx_task.abort(); }
        _ = &mut rx_task => { tx_task.abort(); }
        _ = stop.wait() => {
            log::info!("Stopping WebSocket loop");
            tx_task.abort();
            rx_task.abort();
        }
    }
    ws_active.store(false, Ordering::SeqCst);
}

/// Main server function
async fn server(config: &ServerStartInfo) -> Result<()> {
    log::debug!("Initializing server {}", config.port);
    let state = ServerState::from(config);

    let tls_config = certool::rustls_config_from_pem(config.cert_info.clone())?;
    log::debug!("TLS configuration loaded");

    let handle = axum_server::Handle::new();
    let handle_stop = config.stop.clone();

    let app = Router::new()
        .merge(routes::routes())
        .route("/ws", get(ws_handler))
        .route_layer(middleware::from_fn(check_secret_middleware));

    // TODO: Remove this testing code
    #[cfg(debug_assertions)]
    use tower_http::trace::TraceLayer;

    #[cfg(debug_assertions)]
    let app = app.layer(
        TraceLayer::new_for_http()
            .on_request(|req: &Request<_>, _span: &tracing::Span| {
                log::info!("--> {} {}", req.method(), req.uri());
            })
            .on_response(
                |res: &Response<_>, latency: std::time::Duration, _span: &tracing::Span| {
                    log::info!("<-- {} (took {:?})", res.status(), latency);
                },
            )
            .on_failure(
                |err: _, latency: std::time::Duration, _span: &tracing::Span| {
                    log::error!("!! error={:?} latency={:?}", err, latency);
                },
            ),
    );
    // TODO until here, join app also :)

    let app = app.layer(Extension(state));

    tokio::spawn({
        let handle = handle.clone();
        async move {
            handle_stop.wait().await;
            log::info!("Stop signal received, shutting down server...");
            handle.graceful_shutdown(None);
        }
    });

    fn bind_ipv6_only(port: u16) -> std::io::Result<std::net::TcpListener> {
        let socket = Socket::new(Domain::IPV6, Type::STREAM, None)?;
        socket.set_only_v6(true)?;
        socket.set_reuse_address(true)?;
        socket.bind(&SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port).into())?;
        socket.listen(128)?;
        Ok(socket.into())
    }

    let listener_v4: std::net::TcpListener =
        std::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, config.port))?;
    listener_v4.set_nonblocking(true)?;

    let listener_v6 = bind_ipv6_only(config.port)?;
    listener_v6.set_nonblocking(true)?;

    let svc = app.into_make_service_with_connect_info::<SocketAddr>();

    let server_v4 = axum_server::from_tcp_rustls(listener_v4, tls_config.clone())
        .handle(handle.clone())
        .serve(svc.clone());

    let server_v6 = axum_server::from_tcp_rustls(listener_v6, tls_config)
        .handle(handle)
        .serve(svc);

    try_join!(server_v4, server_v6)?;
    Ok(())
}

// Creates and starts the server, returning a handle to interact with it
//
pub async fn start_server(
    cert_info: CertificateInfo,
    stop: OnceSignal,
    secret: String,
    port: Option<u16>,
) -> Result<(ServerContext, tokio::task::JoinHandle<()>)> {
    // Create channels
    let (to_ws, from_workers) = mpsc::channel::<RpcEnvelope<RpcMessage>>(128);
    let (from_ws, _) = broadcast::channel::<RpcEnvelope<RpcMessage>>(128);
    let tracker = RequestTracker::new();

    // Armar ServerInfo
    let info = ServerStartInfo {
        cert_info,
        port: port.unwrap_or(crate::consts::UDS_PORT),
        workers_to_wsclient: Arc::new(tokio::sync::Mutex::new(from_workers)),
        wsclient_to_workers: from_ws.clone(),
        tracker: tracker.clone(),
        stop: stop.clone(),
        secret,
    };

    // Launch the server task
    let handle = tokio::spawn(async move {
        if let Err(e) = server(&info).await {
            log::error!("Server failed: {e}");
            // Signal stop to main task, because we cannot continue without the server
            stop.set();
        }
    });

    Ok((
        ServerContext {
            to_ws,
            from_ws,
            tracker,
        },
        handle,
    ))
}

#[cfg(test)]
mod tests;

// +-------------------+                       +-------------------+
// |   WebSocket       |                       |      Workers      |
// |   Client          |                       |                   |
// +---------+---------+                       +---------+---------+
//           |                                           ^
//           | WS → Workers (broadcast)                  |
//           |                                           |
//           v                                           |
// +-------------------+                       +-------------------+
// | wsclient_to_workers |  broadcast::Sender  | wsclient_to_workers |
// | (fan-out channel)   | ------------------> |   .subscribe()      |
// +-------------------+                       +-------------------+

// +-------------------+                       +-------------------+
// | workers_to_wsclient | mpsc::Sender        | workers_to_wsclient |
// | (fan-in channel)    | <------------------ |   mpsc::Receiver    |
// +-------------------+                       +-------------------+
//           ^                                           |
//           | Workers → WS (mpsc)                       |
//           |                                           v
// +---------+---------+                       +---------+---------+
// |   WebSocket       |                       |   WebSocket       |
// |   Client          |                       |   Loop (server)   |
// +-------------------+                       +-------------------+
