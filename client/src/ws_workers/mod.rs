use crate::platform;
use shared::log;

mod macros;

mod logoff;
mod screenshot;
mod alive;
mod pong;

use crate::spawn_workers;

#[allow(dead_code)]
pub async fn setup_workers(platform: platform::Platform) {
    spawn_workers!(
        platform,
        [
            ("Logoff", logoff::worker),
            ("Screenshot", screenshot::worker),
            ("Alive", alive::worker),
            ("Pong", pong::worker)
        ],
    );
}
