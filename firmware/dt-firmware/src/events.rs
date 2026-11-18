//! The UI event queue and the threads that feed it.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use dt_core::platform::{ButtonEvent, Buttons};
use log::{error, info};

/// Something the UI thread must react to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Button(ButtonEvent),
}

/// The UI event queue. Unbounded, so a sender never blocks (REQ-API-06).
pub fn channel() -> (Sender<Event>, Receiver<Event>) {
    mpsc::channel()
}

/// How long one wait on the HAL lasts before the thread loops round.
const WAIT: Duration = Duration::from_secs(1);
const STACK_SIZE: usize = 4096;

/// Start the buttons thread: HAL button events become `Event::Button`.
/// The thread ends if the queue's receiver is dropped.
pub fn spawn_buttons(tx: Sender<Event>) -> std::io::Result<()> {
    thread::Builder::new()
        .name("buttons".into())
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let mut buttons = match dt_hal::HalButtons::new() {
                Ok(b) => b,
                Err(e) => {
                    error!("{e}");
                    return;
                }
            };
            info!("buttons thread started");
            loop {
                match buttons.wait(WAIT) {
                    Ok(Some(ev)) => {
                        if tx.send(Event::Button(ev)).is_err() {
                            return;
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        error!("{e}");
                        thread::sleep(WAIT);
                    }
                }
            }
        })
        .map(|_| ())
}
