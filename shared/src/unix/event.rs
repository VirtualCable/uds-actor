use crate::sync::traits::EventLike;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct UnixEvent(Arc<(Mutex<bool>, Condvar)>);

impl UnixEvent {
    pub fn new() -> Self {
        Self(Arc::new((Mutex::new(false), Condvar::new())))
    }
}

impl EventLike for UnixEvent {
    fn wait(&self) {
        let (lock, cvar) = &*self.0;
        let mut set = lock.lock().unwrap();
        while !*set {
            set = cvar.wait(set).unwrap();
        }
    }

    fn wait_timeout(&self, timeout: Duration) -> bool {
        let (lock, cvar) = &*self.0;
        let mut set = lock.lock().unwrap();
        let (set, result) = cvar.wait_timeout_while(set, timeout, |s| !*s).unwrap();
        *set || !result.timed_out()
    }

    fn signal(&self) {
        let (lock, cvar) = &*self.0;
        let mut set = lock.lock().unwrap();
        *set = true;
        cvar.notify_all();
    }

    fn reset(&self) {
        let (lock, _) = &*self.0;
        *lock.lock().unwrap() = false;
    }

    fn is_set(&self) -> bool {
        let (lock, _) = &*self.0;
        *lock.lock().unwrap()
    }
}
