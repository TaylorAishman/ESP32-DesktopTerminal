//! Safe wrappers around the C hardware abstraction layer (`hal/dt_hal`).
//!
//! This is the only crate in the project allowed to contain `unsafe` code.
//! Every `unsafe` block carries a `SAFETY:` comment explaining why it is sound;
//! clippy enforces that.

#![deny(clippy::undocumented_unsafe_blocks)]

use core::ffi::CStr;

use esp_idf_sys::dt_hal as ffi;

pub mod buttons;
pub mod clock;
pub mod display;
pub mod error;
pub mod http;
pub mod store;
pub mod system;
pub mod wifi;

pub use buttons::HalButtons;
pub use clock::HalClock;
pub use display::EinkDisplay;
pub use error::HalError;
pub use http::HalHttp;
pub use store::HalStore;
pub use system::HalSystem;
pub use wifi::HalWifi;

/// Version string reported by the C HAL.
pub fn version() -> &'static str {
    // SAFETY: `hal_version` takes no arguments and returns a pointer to a
    // NUL-terminated string literal with static storage duration; it is never
    // NULL and never freed.
    let raw = unsafe { CStr::from_ptr(ffi::hal_version()) };
    raw.to_str().unwrap_or("<invalid>")
}
