use std::time::Duration;
use std::thread;

use shared::sync::event::{Event, EventLike};

#[test]
fn event_signal_and_wait() {
    let ev = Event::new();
    assert!(!ev.is_set());

    let ev2 = ev.clone();
    let handle = thread::spawn(move || {
        ev2.wait();
        42
    });

    // Señalamos después de un pequeño delay
    thread::sleep(Duration::from_millis(100));
    ev.signal();

    let result = handle.join().unwrap();
    assert_eq!(result, 42);
    assert!(ev.is_set());
}

#[test]
fn event_wait_timeout() {
    let ev = Event::new();

    // No está señalizado, debería expirar
    let signaled = ev.wait_timeout(Duration::from_millis(100));
    assert!(!signaled);

    // Ahora lo señalizamos y debería despertar
    ev.signal();
    let signaled = ev.wait_timeout(Duration::from_millis(100));
    assert!(signaled);
}

#[test]
fn event_reset() {
    let ev = Event::new();
    ev.signal();
    assert!(ev.is_set());

    ev.reset();
    assert!(!ev.is_set());
}
