//! Monotonic and wall-clock time, implementing [`dt_core::platform::Clock`].

use std::ffi::CString;

use dt_core::platform::{Clock, Instant, PlatformResult, UnixTime};
use dt_core::{AppError, ErrorCode};
use esp_idf_sys::dt_hal as ffi;

use crate::HalError;

/// Time since boot, SNTP-synchronised wall-clock time and the local zone.
#[derive(Debug, Default, Clone, Copy)]
pub struct HalClock;

impl HalClock {
    /// Set the time zone and start SNTP. Call once, after Wi-Fi has started.
    pub fn start_sync(&self, posix_tz: &str) -> PlatformResult<()> {
        let tz = CString::new(posix_tz).map_err(|_| AppError::with_message(ErrorCode::ConfigInvalid, "Time zone contains a NUL character."))?;
        // SAFETY: `tz` is a valid NUL-terminated string for the call; the C
        // side copies it into the environment.
        HalError::check(unsafe { ffi::hal_time_start_sync(tz.as_ptr()) }).map_err(|e| e.into_app_error("Time sync start"))
    }
}

impl Clock for HalClock {
    fn monotonic(&self) -> Instant {
        // SAFETY: no arguments; reads a hardware timer.
        Instant::from_millis(unsafe { ffi::hal_sys_uptime_ms() })
    }

    fn now(&self) -> Option<UnixTime> {
        let mut t = 0i64;
        // SAFETY: `t` is a live, writable local.
        let rc = unsafe { ffi::hal_time_now(&mut t) };
        HalError::check(rc).ok().map(|()| UnixTime(t))
    }

    fn utc_offset(&self, at: UnixTime) -> i32 {
        let mut off = 0i32;
        // SAFETY: `off` is a live, writable local.
        let rc = unsafe { ffi::hal_time_utc_offset(at.seconds(), &mut off) };
        if HalError::check(rc).is_ok() {
            off
        } else {
            0
        }
    }
}
