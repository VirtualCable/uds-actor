use std::sync::Arc;
use tokio::sync::Notify;

use anyhow::Result;

// Run service is platform dependent
// Will invoke back this "run" function,
#[cfg(target_os = "windows")]
pub use crate::windows::service::run_service;

pub trait AsyncLauncherTrait: Send + Sync + 'static {
    fn run(&self, stop: Arc<Notify>);
}

pub struct AsyncLauncher {
    // Add async fn to call as main_async
    main_async: fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>,
}

impl AsyncLauncher {
    pub fn new(
        main_async: fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>,
    ) -> Self {
        Self { main_async }
    }
    #[cfg(target_os = "windows")]
    pub fn run_service(self) -> Result<()> {
        run_service(self)
    }

    #[cfg(not(target_os = "windows"))]
    pub fn run_service(self) -> Result<()> {
        // On other, just run directly
        // Notify is a dummy here
        let notify = Arc::new(Notify::new());
        self.run(notify);
    }
}

impl AsyncLauncherTrait for AsyncLauncher {
    fn run(&self, stop: Arc<Notify>) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all() // Enable timers, I/O, etc.
            .build()
            .unwrap();

        rt.block_on(async move {
            tokio::select! {
                _ = (self.main_async)() => {},
                _ = stop.notified() => {
                    // shutdown logic
                    println!("Stop received...");
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;
    use tokio::time::timeout;

    async fn async_main() {
        // main logic
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            println!("Working...");
        }
    }

    #[tokio::test]
    async fn test_run_stops_on_notify() {
        let stop = Arc::new(Notify::new());
        let stop_clone = stop.clone();
        let stopped = Arc::new(AtomicBool::new(false));
        let stopped_clone = stopped.clone();

        let launcher = AsyncLauncher::new(|| Box::pin(async_main()));
        let handle = std::thread::spawn(move || {
            launcher.run(stop_clone);
            stopped_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        // Let it run a bit
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(!stopped.load(std::sync::atomic::Ordering::SeqCst));

        // Notify to stop
        stop.notify_one();
        // Wait for thread to join, with timeout
        let res = timeout(Duration::from_secs(5), async {
            handle.join().unwrap();
        })
        .await;
        assert!(res.is_ok(), "Thread did not stop in time");
        assert!(stopped.load(std::sync::atomic::Ordering::SeqCst));
    }
}
