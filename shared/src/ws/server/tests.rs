use super::*;
use std::time::Duration;

#[tokio::test]
async fn test_websocket_ping_pong() {
    crate::log::setup_logging("debug", crate::log::LogType::Tests);

    let _ = rustls::crypto::ring::default_provider().install_default();

    let (tx, mut rx) = tokio::sync::broadcast::channel::<String>(16);
    let (cert_pem, key_pem) = test_certs::test_cert_and_key();
    let port = 3003;

    // Arrancamos el server
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        let res = server(cert_pem, key_pem, port, tx_clone).await;
        log::info!("Server finished: {:?}", res);
    });

    // Task que escucha en el canal y responde
    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if msg == "ping" {
                let _ = tx.send("pong".into());
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(300)).await;

    // WS Client
    let connector = tokio_tungstenite::Connector::Rustls(crate::tls::noverify::client_config());
    let url = format!("wss://localhost:{}/ws", port);
    let (mut ws, _) =
        tokio_tungstenite::connect_async_tls_with_config(url, None, true, Some(connector))
            .await
            .expect("WS connect failed");

    // Enviamos ping
    ws.send(tokio_tungstenite::tungstenite::protocol::Message::Text(
        "ping".into(),
    ))
    .await
    .unwrap();

    // Esperamos pong
    if let Some(Ok(tokio_tungstenite::tungstenite::protocol::Message::Text(msg))) = ws.next().await
    {
        assert_eq!(msg, "pong");
    } else {
        panic!("No recibimos pong");
    }
}
