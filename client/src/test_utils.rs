use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::{fs, path::PathBuf};

use mockito::{ServerGuard};

#[allow(dead_code)]
pub struct MockServer {
    pub server: ServerGuard,
    pub launch: mockito::Mock,
    pub log: mockito::Mock,
    pub stop: mockito::Mock,
    pub temp_file: PathBuf,
    pub ticket_id: &'static str,
}

impl Drop for MockServer {
    fn drop(&mut self) {
        // Cleanup: Remove the temp file if it exists
        let _ = fs::remove_file(&self.temp_file);
    }
}

#[allow(dead_code)]
pub fn run_with_timeout<F, R>(timeout: Duration, func: F) -> Result<R, &'static str>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let (tx, rx) = mpsc::channel();

    // Main thread to run the function
    let tx_clone = tx.clone();
    thread::spawn(move || {
        let result = func();
        let _ = tx_clone.send(Some(result));
    });

    // timeout thread
    thread::spawn(move || {
        thread::sleep(timeout);
        let _ = tx.send(None); // si gana el timeout, enviamos None
    });

    match rx.recv() {
        Ok(Some(result)) => Ok(result),
        Ok(None) => Err("Timeout reached"),
        Err(_) => Err("Error receiving result"),
    }
}
