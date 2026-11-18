//! The e-ink panel, implementing [`dt_core::platform::Display`].

use dt_core::platform::{Display, PlatformResult};
use dt_core::{AppError, ErrorCode};
use esp_idf_sys::dt_hal as ffi;

use crate::HalError;

/// Bytes in one frame (800 × 480 at 1 bit per pixel).
pub const FRAME_BYTES: usize = ffi::HAL_DISPLAY_FRAME_BYTES as usize;

/// The Waveshare 7.5" panel behind the C HAL. The C side serialises access,
/// so any number of handles may exist; the application keeps one.
#[derive(Debug, Default)]
pub struct EinkDisplay {
    _private: (),
}

impl EinkDisplay {
    /// Initialise the panel and leave it asleep. Failing here is not fatal:
    /// every later call re-initialises the panel if needed.
    pub fn new() -> PlatformResult<Self> {
        let display = EinkDisplay { _private: () };
        // SAFETY: takes no arguments; the C side serialises access.
        map(unsafe { ffi::hal_display_init() }, "Display init")?;
        // SAFETY: as above.
        map(unsafe { ffi::hal_display_sleep() }, "Display sleep")?;
        Ok(display)
    }
}

impl Display for EinkDisplay {
    fn show(&mut self, frame: &[u8]) -> PlatformResult<()> {
        if frame.len() != FRAME_BYTES {
            return Err(AppError::with_message(
                ErrorCode::SysInternal,
                format!("Display frame is {} bytes, expected {FRAME_BYTES}.", frame.len()),
            ));
        }
        // SAFETY: `frame` is a valid, initialised slice of exactly `len` bytes
        // for the duration of the call; the C side only reads it and keeps no
        // pointer after returning.
        map(
            unsafe { ffi::hal_display_show(frame.as_ptr(), frame.len()) },
            "Display refresh",
        )
    }

    fn clear(&mut self) -> PlatformResult<()> {
        // SAFETY: takes no arguments; the C side serialises access.
        map(unsafe { ffi::hal_display_clear() }, "Display clear")
    }

    fn sleep(&mut self) -> PlatformResult<()> {
        // SAFETY: takes no arguments; the C side serialises access.
        map(unsafe { ffi::hal_display_sleep() }, "Display sleep")
    }
}

/// Display-specific error mapping: a timeout means the panel's BUSY line
/// never cleared, and I/O errors are SPI failures.
fn map(rc: ffi::hal_err_t, operation: &str) -> PlatformResult<()> {
    HalError::check(rc).map_err(|e| match e {
        HalError::Timeout => AppError::with_message(
            ErrorCode::DisplayBusyTimeout,
            format!("{operation}: panel did not respond (BUSY stuck)."),
        ),
        HalError::Io => AppError::with_message(
            ErrorCode::DisplayIo,
            format!("{operation}: SPI communication with the panel failed."),
        ),
        other => other.into_app_error(operation),
    })
}
