//! Desktop terminal firmware.

#![forbid(unsafe_code)]

use std::{thread, time::Duration};

use dt_core::platform::{Display, System};
use dt_core::store::Store;
use dt_ui::view::Context;
use dt_ui::{render, FrameBuffer, Screen};
use log::{error, info, warn};

mod config;
mod logger;

use config::CONFIG;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    // Required once at start-up so ESP-IDF's runtime patches are linked in.
    esp_idf_sys::link_patches();
    logger::init();

    let system = dt_hal::HalSystem;
    let reset = system.reset_reason();
    info!("Desktop Terminal {VERSION} (C HAL {}), reset reason {reset:?}", dt_hal::version());
    info!(
        "config: {} topics, {} instruments, location {}",
        CONFIG.topics.len(),
        CONFIG.instruments.len(),
        CONFIG.location.name
    );
    if reset.is_abnormal() {
        warn!("previous run ended abnormally ({reset:?})");
    }

    // The frame buffer is allocated once and reused for every frame.
    let mut frame = FrameBuffer::new();
    let store = Store::new(CONFIG.topics.len(), CONFIG.instruments.len());
    let ctx = Context::from_config(&CONFIG, None, 0, None);

    // A display failure is logged, never fatal: the next refresh retries.
    let mut display = match dt_hal::EinkDisplay::new() {
        Ok(d) => Some(d),
        Err(e) => {
            error!("{e}");
            None
        }
    };
    let splash = Screen::Splash {
        version: VERSION.into(),
        status: "Starting...".into(),
    };
    render(&mut frame, &splash, &store, &CONFIG, &ctx);
    match display.as_mut().map(|d| d.show(frame.as_bytes())) {
        Some(Ok(())) => info!("splash shown"),
        Some(Err(e)) => error!("{e}"),
        None => {}
    }

    let heap = system.heap();
    info!("heap: {} B free, {} B lowest", heap.free_bytes, heap.min_free_bytes);
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
