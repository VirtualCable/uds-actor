use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_path = "/org/freedesktop/login1",
    default_service = "org.freedesktop.login1"
)]
trait LoginManager {
    // Señales que nos interesan
    #[zbus(signal)]
    fn SessionNew(id: &str, path: zbus::zvariant::ObjectPath<'_>);

    #[zbus(signal)]
    fn SessionRemoved(id: &str, path: zbus::zvariant::ObjectPath<'_>);
}

// use futures_util::StreamExt;

// #[tokio::main]
// async fn main() -> zbus::Result<()> {
//     let conn = zbus::Connection::system().await?;
//     let proxy = LoginManagerProxy::new(&conn).await?;

//     let mut new_stream = proxy.receive_session_new().await?;
//     let mut removed_stream = proxy.receive_session_removed().await?;

//     tokio::spawn(async move {
//         while let Some(signal) = new_stream.next().await {
//             let args = signal.args().unwrap();
//             println!("Nueva sesión: {} {}", args.id, args.path);
//         }
//     });

//     while let Some(signal) = removed_stream.next().await {
//         let args = signal.args().unwrap();
//         println!("Sesión terminada: {} {}", args.id, args.path);
//     }

//     Ok(())
// }
