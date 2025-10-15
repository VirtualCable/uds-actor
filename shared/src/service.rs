// Copyright (c) 2025 Virtual Cable S.L.U.
// All rights reserved.
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
//    * Redistributions of source code must retain the above copyright notice,
//      this list of conditions and the following disclaimer.
//    * Redistributions in binary form must reproduce the above copyright notice,
//      this list of conditions and the following disclaimer in the documentation
//      and/or other materials provided with the distribution.
//    * Neither the name of Virtual Cable S.L.U. nor the names of its contributors
//      may be used to endorse or promote products derived from this software
//      without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
/*!
Author: Adolfo Gómez, dkmaster at dkmon dot com
*/
use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::Result;

use crate::sync::OnceSignal;

// Run service is platform dependent
// Will invoke back this "run" function,
#[cfg(target_os = "windows")]
pub use crate::windows::service::run_service;

pub trait AsyncServiceTrait: Send + Sync + 'static {
    fn run(&self, stop: Arc<OnceSignal>);

    fn get_stop_notify(&self) -> Arc<OnceSignal>;

    fn get_restart_flag(&self) -> Arc<AtomicBool>;

    fn should_restart(&self) -> bool;
}

// Type alias for the main async function signature
type MainAsyncFn = fn(Arc<OnceSignal>, Arc<AtomicBool>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;

pub struct AsyncService {
    // Add async fn to call as main_async
    main_async: MainAsyncFn,
    stop: Arc<OnceSignal>,
    restart_flag: Arc<AtomicBool>,
}

impl AsyncService {
    pub fn new(main_async: MainAsyncFn) -> Self {
        Self {
            main_async,
            stop: Arc::new(OnceSignal::new()),
            restart_flag: Arc::new(AtomicBool::new(false)),
        }
    }
    #[cfg(target_os = "windows")]
    pub fn run_service(self) -> Result<()> {
        run_service(self)
    }

    #[cfg(not(target_os = "windows"))]
    pub fn run_service(self) -> Result<()> {
        // On other, just run directly
        self.run(self.stop.clone());
        Ok(())
    }

    async fn signals(stop: Arc<OnceSignal>) {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};

            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            let mut sigint = signal(SignalKind::interrupt()).unwrap();

            tokio::select! {
                _ = sigterm.recv() => {
                    crate::log::info!("Received SIGTERM");
                },
                _ = sigint.recv() => {
                    crate::log::info!("Received SIGINT");
                }
                _ = stop.wait() => {
                    crate::log::info!("Stop notified");
                    return;
                }
            }
            // Notify to stop
            stop.set();
        }

        #[cfg(windows)]
        {
            // On windows, we don't have signals, just wait forever
            // The service control handler will notify us to stop
            stop.wait().await;
        }
    }
}

impl AsyncServiceTrait for AsyncService {
    fn run(&self, stop: Arc<OnceSignal>) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all() // Enable timers, I/O, etc.
            .build()
            .unwrap();

        rt.block_on(async move {
            let mut main_task = tokio::spawn((self.main_async)(self.stop.clone(), self.restart_flag.clone()));
            let signals_task = tokio::spawn(AsyncService::signals(stop.clone()));
            tokio::select! {
                res = &mut main_task => {
                    match res {
                        Ok(task_res) => {
                            crate::log::info!("Main async task completed");
                            if let Err(e) = task_res {
                                crate::log::error!("Main async task error: {}", e);
                            }
                        },
                        Err(e) => {
                            crate::log::error!("Main async task failed: {}", e);
                        }
                    }
                    stop.set();
                    signals_task.abort();  // This can be safely aborted
                },
                // Stop from SCM (on windows) or signal (on unix)
                _ = stop.wait() => {
                    crate::log::debug!("Stop received (external)");
                    // Main task may need to do some cleanup, give it some time
                    let grace = Duration::from_secs(16);
                    if tokio::time::timeout(grace, &mut main_task).await.is_err() {
                        crate::log::warn!("Main task did not stop in {grace:?}, aborting");
                        main_task.abort();
                    }
                    // Also abort signals task
                    signals_task.abort();
                }
            }
        });
    }

    fn get_stop_notify(&self) -> Arc<OnceSignal> {
        self.stop.clone()
    }

    fn get_restart_flag(&self) -> Arc<AtomicBool> {
        self.restart_flag.clone()
    }

    fn should_restart(&self) -> bool {
        self.restart_flag.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;
    use tokio::time::timeout;

    fn async_main(stop: Arc<OnceSignal>, _restart_flag: Arc<AtomicBool>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        Box::pin(async move {
            // main logic
            stop.wait().await;
            println!("Stop received");
            Ok(())
        })
    }

    fn async_main_restart(stop: Arc<OnceSignal>, restart_flag: Arc<AtomicBool>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        Box::pin(async move {
            // main logic
            restart_flag.store(true, Ordering::Relaxed);
            stop.wait().await;
            Err(anyhow::anyhow!("Simulated error"))
        })
    }

    #[tokio::test]
    async fn test_run_stops_on_notify() {
        let stopped = Arc::new(AtomicBool::new(false));
        let service = AsyncService::new(async_main);
        let restart_flag = service.get_restart_flag();

        let stop = service.get_stop_notify();
        let handle = std::thread::spawn({
            let stop = stop.clone();
            let stopped = stopped.clone();
            move || {
                service.run(stop);
                stopped.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        });

        // Let it run a bit
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(!stopped.load(std::sync::atomic::Ordering::Relaxed));

        // Notify to stop
        stop.set();
        // Wait for thread to join, with timeout
        let res = timeout(Duration::from_secs(5), async {
            handle.join().unwrap();
        })
        .await;
        assert!(res.is_ok(), "Thread did not stop in time");
        assert!(stopped.load(std::sync::atomic::Ordering::Relaxed));
        // restart_flag should be false, as default
        assert!(!restart_flag.load(std::sync::atomic::Ordering::Relaxed));
    }

    // Test that restart_flag is set if requested
    #[tokio::test]
    async fn test_run_sets_restart_on_error() {
        let stopped = Arc::new(AtomicBool::new(false));
        let service = AsyncService::new(async_main_restart);
        let restart_flag = service.get_restart_flag();

        let stop = service.get_stop_notify();
        let handle = std::thread::spawn({
            let stop = stop.clone();
            let stopped = stopped.clone();
            move || {
                service.run(stop);
                stopped.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        });

        // Let it run a bit
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(!stopped.load(std::sync::atomic::Ordering::Relaxed));

        // Notify to stop
        stop.set();
        // Wait for thread to join, with timeout
        let res = timeout(Duration::from_secs(5), async {
            handle.join().unwrap();
        })
        .await;
        assert!(res.is_ok(), "Thread did not stop in time");
        assert!(stopped.load(std::sync::atomic::Ordering::Relaxed));
        // restart_flag should be true, as requested
        assert!(restart_flag.load(std::sync::atomic::Ordering::Relaxed));
    }
}