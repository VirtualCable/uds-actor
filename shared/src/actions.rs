#[cfg(target_os = "windows")]
pub use crate::windows::actions::*;

#[cfg(unix)]
pub use crate::unix::actions::*;