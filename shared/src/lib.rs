pub mod log;

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
