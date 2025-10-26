use shared::{log, ws::server::ServerContext};

use crate::platform;

pub mod logoff;
pub mod message;
pub mod preconnect;
pub mod uniqueid;
pub mod script;
pub mod screenshot;

use crate::spawn_workers;

#[allow(dead_code)]
pub async fn create_workers(server_info: ServerContext, platform: platform::Platform) {
    spawn_workers!(
        server_info,
        platform,
        [
            ("Logoff", logoff::worker),
            ("Message", message::worker),
            ("Script", script::worker),
            ("PreConnect", preconnect::worker),
            ("Screenshot", uniqueid::worker),
            ("UniqueId", screenshot::worker),
        ],
        [],
        []
    );
}
