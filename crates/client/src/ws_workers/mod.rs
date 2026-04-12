use crate::platform;
use shared::log;

mod macros;

mod alive;
mod close;
mod logoff;
mod pong;
mod screenshot;

use crate::spawn_workers;

#[allow(dead_code)]
pub async fn setup_workers(platform: platform::Platform) {
    spawn_workers!(
        platform,
        [
            ("Logoff", logoff::worker),
            ("Screenshot", screenshot::worker),
            ("Alive", alive::worker),
            ("Pong", pong::worker),
            ("Close", close::worker)
        ],
    );
}
