#[macro_export]
macro_rules! spawn_workers {
    // spawns a single worker sub-macro
    (@spawn_one $server_info:expr, $platform:expr, $name:literal, $func:path) => {{
        log::info!("{} worker created", $name);
        tokio::spawn({
            let s = $server_info.clone();
            let p = $platform.clone();
            async move {
                if let Err(e) = $func(s, p).await {
                    log::error!("{} worker error: {:?}", $name, e);
            }
        }});
    }};

    (
        $server_info:expr,
        $platform:expr,
        [ $( ($name1:literal, $func1:path) ),* $(,)? ],
        [ $( ($name2:literal, $func2:path) ),* $(,)? ],
        [ $( ($name3:literal, $func3:path) ),* $(,)? ]
    ) => {{
        // Common workers
        $(
            spawn_workers!(@spawn_one $server_info, $platform, $name1, $func1);
        )*

        // Actor type specific workers
        {
            tokio::spawn({
                async move {
                let p = $platform.clone();
                let actor_type = p.config().read().await.actor_type.clone();
                if actor_type.is_managed() {
                    $(
                        spawn_workers!(@spawn_one $server_info, $platform, $name2, $func2);
                    )*
                } else {
                    $(
                        spawn_workers!(@spawn_one $server_info, $platform, $name3, $func3);
                    )*
                }
            }});
        }
    }};
}
