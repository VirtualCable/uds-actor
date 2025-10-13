#[macro_export]
macro_rules! spawn_workers {
    ( $server_info:expr, $platform:expr, [ $( ($name:literal, $func:path) ),* $(,)? ] ) => {
        $(
            {
                let s = $server_info.clone();
                let p = $platform.clone();
                log::info!("{} worker created", $name);
                tokio::spawn(async move {
                    if let Err(e) = $func(s, p).await {
                        log::error!("{} worker error: {:?}", $name, e);
                    }
                });
            }
        )*
    };
}