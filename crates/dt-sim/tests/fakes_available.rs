//! The platform fakes are usable from outside dt-core.

use dt_core::platform::fake::{FakeClock, FakeDisplay};
use dt_core::platform::{Clock, Display};
use std::time::Duration;

#[test]
fn fakes_work_from_another_crate() {
    let clock = FakeClock::new();
    clock.advance(Duration::from_secs(1));
    assert_eq!(clock.monotonic().as_millis(), 1000);

    let handle = FakeDisplay::new();
    let mut display = handle.clone();
    display.show(&[0u8; 4]).unwrap();
    assert_eq!(handle.show_count(), 1);
}
