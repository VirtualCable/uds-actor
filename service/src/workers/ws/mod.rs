use shared::{log, ws::server::ServerContext};

use crate::platform;

mod logger;
mod login_managed;
mod login_unmanaged;
mod logout;

use crate::spawn_workers;

#[allow(dead_code)]
pub async fn create_workers(server_info: ServerContext, platform: platform::Platform) {
    spawn_workers!(
        server_info,
        platform,
        [("Log", logger::worker), ("Logout", logout::worker),],
        // Managed only workers
        [("Login", login_managed::worker),],
        // Unmanaged only workers
        [("Login", login_unmanaged::worker),]
    );
}
