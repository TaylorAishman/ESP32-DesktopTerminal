//! Desktop terminal firmware.

#![forbid(unsafe_code)]

use std::{thread, time::Duration};

mod config;

use config::CONFIG;

fn main() {
    // Required once at start-up so ESP-IDF's runtime patches are linked in.
    esp_idf_sys::link_patches();

    println!("dt-firmware: C HAL version {}", dt_hal::version());
    println!(
        "config: {} topics, {} instruments, location {}",
        CONFIG.topics.len(),
        CONFIG.instruments.len(),
        CONFIG.location.name
    );

    for second in 0u32.. {
        println!("dt-firmware alive, {second} s");
        thread::sleep(Duration::from_secs(1));
    }
}
