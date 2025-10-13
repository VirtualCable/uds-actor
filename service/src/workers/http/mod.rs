use shared::{log, ws::server::ServerInfo};

use crate::platform;

pub mod logoff;
pub mod message;
pub mod preconnect;
pub mod screenshot;
pub mod script;

use crate::spawn_workers;

#[allow(dead_code)]
pub async fn create_workers(server_info: ServerInfo, platform: platform::Platform) {
    spawn_workers!(
        server_info,
        platform,
        [
            ("Logoff", logoff::worker),
            ("Message", message::worker),
            ("Script", script::worker),
            ("PreConnect", preconnect::worker),
            ("Screenshot", screenshot::worker),
        ]
    );
}
