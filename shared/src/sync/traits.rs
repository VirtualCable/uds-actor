use std::time::Duration;

pub trait EventLike: Clone + std::fmt::Debug {
    fn wait(&self);
    fn wait_timeout(&self, timeout: Duration) -> bool;
    fn signal(&self);
    fn reset(&self);
    fn is_set(&self) -> bool;
}
