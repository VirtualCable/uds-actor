pub use super::traits::EventLike;

#[cfg(target_os = "windows")]
pub use crate::windows::event::WindowsEvent;

#[cfg(target_os = "windows")]
pub use WindowsEvent as Event;

#[cfg(not(target_os = "windows"))]
pub use crate::unix::event::UnixEvent;
#[cfg(not(target_os = "windows"))]
pub use UnixEvent as Event;
