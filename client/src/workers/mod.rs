use crate::platform;
use shared::log;

mod macros;

mod logoff;
mod screenshot;

use crate::spawn_workers;

#[allow(dead_code)]
pub async fn setup_workers(platform: platform::Platform) {
    spawn_workers!(
        platform,
        [
            ("logoff", logoff::worker),
            ("screenshot", screenshot::worker),
        ],
    );
}
