#[macro_export]
macro_rules! spawn_workers((
    $platform:expr,
    [ $( ($name1:literal, $func1:path) ),* $(,)? ],
) => {{
    // Common workers
    $(
        {
            log::info!("{} worker created", $name1);
            let p = $platform.clone();
            tokio::spawn(async move {
                if let Err(e) = $func1(p).await {
                    log::error!("{} worker error: {:?}", $name1, e);
                }
            });
        }
    )*
}});
