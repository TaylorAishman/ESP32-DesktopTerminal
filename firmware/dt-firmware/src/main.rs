//! Desktop terminal firmware.

#![forbid(unsafe_code)]

use std::{thread, time::Duration};

fn main() {
    // Required once at start-up so ESP-IDF's runtime patches are linked in.
    esp_idf_sys::link_patches();

    println!("dt-firmware: C HAL version {}", dt_hal::version());

    for second in 0u32.. {
        println!("dt-firmware alive, {second} s");
        thread::sleep(Duration::from_secs(1));
    }
}
