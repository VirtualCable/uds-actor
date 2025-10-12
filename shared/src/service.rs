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
use std::{sync::Arc, time::Duration, pin::Pin};
use tokio::sync::Notify;

use anyhow::Result;

// Run service is platform dependent
// Will invoke back this "run" function,
#[cfg(target_os = "windows")]
pub use crate::windows::service::run_service;

pub trait AsyncServiceTrait: Send + Sync + 'static {
    fn run(&self, stop: Arc<Notify>);

    fn get_stop_notify(&self) -> Arc<Notify>;
}

// Type alias for the main async function signature
type MainAsyncFn = fn(Arc<Notify>) -> Pin<Box<dyn Future<Output = ()> + Send>>;

pub struct AsyncService {
    // Add async fn to call as main_async
    main_async: MainAsyncFn,
    stop: Arc<Notify>,
}

impl AsyncService {
    pub fn new(main_async: MainAsyncFn) -> Self {
        Self {
            main_async,
            stop: Arc::new(Notify::new()),
        }
    }
    #[cfg(target_os = "windows")]
    pub fn run_service(self) -> Result<()> {
        run_service(self)
    }

    #[cfg(not(target_os = "windows"))]
    pub fn run_service(self) -> Result<()> {
        // On other, just run directly
        // Stop is a dummy here
        self.run(self.stop.clone());
        Ok(())
    }

    async fn signals(stop: Arc<Notify>) {
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
                _ = stop.notified() => {
                    crate::log::info!("Stop notified");
                    return;
                }
            }
            // Notify to stop
            stop.notify_waiters();
        }

        #[cfg(windows)]
        {
            // On windows, we don't have signals, just wait forever
            // The service control handler will notify us to stop
            stop.notified().await;
        }
    }
}

impl AsyncServiceTrait for AsyncService {
    fn run(&self, stop: Arc<Notify>) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all() // Enable timers, I/O, etc.
            .build()
            .unwrap();

        rt.block_on(async move {
            let mut main_task = tokio::spawn((self.main_async)(stop.clone()));
            let signals_task = tokio::spawn(AsyncService::signals(stop.clone()));
            tokio::select! {
                res = &mut main_task => {
                    match res {
                        Ok(_) => {
                            crate::log::info!("Main async task completed");
                        },
                        Err(e) => {
                            crate::log::error!("Main async task failed: {}", e);
                        }
                    }
                    stop.notify_waiters();
                    signals_task.abort();  // This can be safely aborted
                },
                // Stop from SCM (on windows) or signal (on unix)
                _ = stop.notified() => {
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

    fn get_stop_notify(&self) -> Arc<Notify> {
        self.stop.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;
    use tokio::time::timeout;

    fn async_main(stop: Arc<Notify>) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async move {
            // main logic
            stop.notified().await;
            println!("Stop received");
        })
    }

    #[tokio::test]
    async fn test_run_stops_on_notify() {
        let stopped = Arc::new(AtomicBool::new(false));
        let stopped_clone = stopped.clone();

        let service = AsyncService::new(async_main);
        let stop = service.get_stop_notify();
        let stop_clone = stop.clone();
        let handle = std::thread::spawn(move || {
            service.run(stop_clone);
            stopped_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        // Let it run a bit
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(!stopped.load(std::sync::atomic::Ordering::SeqCst));

        // Notify to stop
        stop.notify_waiters();
        // Wait for thread to join, with timeout
        let res = timeout(Duration::from_secs(5), async {
            handle.join().unwrap();
        })
        .await;
        assert!(res.is_ok(), "Thread did not stop in time");
        assert!(stopped.load(std::sync::atomic::Ordering::SeqCst));
    }
}
