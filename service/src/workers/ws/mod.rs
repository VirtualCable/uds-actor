use shared::{log, ws::server::ServerInfo};

use crate::platform;

mod logger;
mod login;
mod logout;

use crate::spawn_workers;

#[allow(dead_code)]
pub async fn create_workers(server_info: ServerInfo, platform: platform::Platform) {
    spawn_workers!(
        server_info,
        platform,
        [
            ("Log", logger::worker),
            ("Login", login::worker),
            ("Logout", logout::worker),
        ]
    );
}
