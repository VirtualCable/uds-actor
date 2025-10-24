#[cfg(target_os = "windows")]
pub use crate::windows::installer;

#[cfg(target_family = "unix")]
pub use crate::unix::installer;
