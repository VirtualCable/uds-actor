/*!
Author: Andres Schumann, aschumann at virtualcable dot es
*/
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

use eframe::egui::Context;

pub const LOST: &str = "The background task stopped unexpectedly";

struct Worker<T> {
    sender: Option<Sender<T>>,
    ctx: Context,
}

impl<T> Drop for Worker<T> {
    fn drop(&mut self) {
        drop(self.sender.take());
        self.ctx.request_repaint();
    }
}

pub struct Task<T> {
    result: Receiver<T>,
}

impl<T: Send + 'static> Task<T> {
    pub fn spawn(ctx: &Context, work: impl FnOnce() -> T + Send + 'static) -> Self {
        let (sender, result) = channel();
        let mut worker = Worker {
            sender: Some(sender),
            ctx: ctx.clone(),
        };
        std::thread::spawn(move || {
            let value = work();
            if let Some(sender) = worker.sender.take() {
                let _ = sender.send(value);
            }
        });
        Self { result }
    }

    pub fn poll_or(&self, on_lost: impl FnOnce() -> T) -> Option<T> {
        match self.result.try_recv() {
            Ok(value) => Some(value),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(on_lost()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn wait_for<T: Send + 'static>(task: &Task<T>, on_lost: impl Fn() -> T) -> Option<T> {
        (0..2000).find_map(|_| {
            let polled = task.poll_or(&on_lost);
            if polled.is_none() {
                std::thread::sleep(Duration::from_millis(1));
            }
            polled
        })
    }

    #[test]
    fn poll_returns_the_result_of_the_background_work() {
        let task = Task::spawn(&Context::default(), || 42);
        assert_eq!(wait_for(&task, || -1), Some(42));
    }

    #[test]
    fn poll_reports_a_task_that_ended_without_a_result() {
        let task = Task::spawn(&Context::default(), || -> i32 { panic!("work failed") });
        assert_eq!(wait_for(&task, || -1), Some(-1));
    }
}
