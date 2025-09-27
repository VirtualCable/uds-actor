pub mod log;
pub mod sync;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(not(target_os = "windows"))]
pub mod unix;

#[macro_export]
macro_rules! debug_dev {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            let s = format!($($arg)*);
            shared::log::info!(target: "dev", "{}", s);
        }
    };
}

#[cfg(test)]
pub mod test_utils;
