//! Safe wrappers around the C hardware abstraction layer (`hal/dt_hal`).
//!
//! This is the only crate in the project allowed to contain `unsafe` code.
//! Every `unsafe` block carries a `SAFETY:` comment explaining why it is sound.

use core::ffi::CStr;

use esp_idf_sys::dt_hal as ffi;

/// Version string reported by the C HAL.
pub fn version() -> &'static str {
    // SAFETY: `hal_version` takes no arguments and returns a pointer to a
    // NUL-terminated string literal with static storage duration; it is never
    // NULL and never freed.
    let raw = unsafe { CStr::from_ptr(ffi::hal_version()) };
    raw.to_str().unwrap_or("<invalid>")
}
