//! System services, implementing [`dt_core::platform::System`].

use core::time::Duration;

use dt_core::platform::{HeapStats, PlatformResult, ResetReason, System};
use esp_idf_sys::dt_hal as ffi;

use crate::HalError;

/// Uptime, memory, reset reason, restart, watchdog and retained memory.
#[derive(Debug, Default, Clone, Copy)]
pub struct HalSystem;

impl HalSystem {
    /// Milliseconds since boot.
    pub fn uptime_ms(&self) -> u64 {
        // SAFETY: no arguments; reads a hardware timer.
        unsafe { ffi::hal_sys_uptime_ms() }
    }
}

impl System for HalSystem {
    fn heap(&self) -> HeapStats {
        let (mut free, mut min) = (0u32, 0u32);
        // SAFETY: both pointers refer to live, writable locals for the call.
        let rc = unsafe { ffi::hal_sys_heap(&mut free, &mut min) };
        if HalError::check(rc).is_err() {
            return HeapStats {
                free_bytes: 0,
                min_free_bytes: 0,
            };
        }
        HeapStats {
            free_bytes: free,
            min_free_bytes: min,
        }
    }

    fn reset_reason(&self) -> ResetReason {
        // SAFETY: no arguments; reads a chip register.
        match unsafe { ffi::hal_sys_reset_reason() } {
            ffi::hal_reset_reason_t_HAL_RESET_POWER_ON => ResetReason::PowerOn,
            ffi::hal_reset_reason_t_HAL_RESET_SOFTWARE => ResetReason::Software,
            ffi::hal_reset_reason_t_HAL_RESET_PANIC => ResetReason::Panic,
            ffi::hal_reset_reason_t_HAL_RESET_WATCHDOG => ResetReason::Watchdog,
            ffi::hal_reset_reason_t_HAL_RESET_BROWNOUT => ResetReason::Brownout,
            ffi::hal_reset_reason_t_HAL_RESET_EXTERNAL => ResetReason::External,
            ffi::hal_reset_reason_t_HAL_RESET_DEEP_SLEEP => ResetReason::DeepSleep,
            _ => ResetReason::Unknown,
        }
    }

    fn restart(&self) {
        // SAFETY: no arguments; does not return.
        unsafe { ffi::hal_sys_restart() }
    }

    fn watchdog_register(&self, timeout: Duration) -> PlatformResult<()> {
        let ms = timeout.as_millis().clamp(1, u128::from(u32::MAX)) as u32;
        // SAFETY: plain value argument; acts on the calling thread.
        HalError::check(unsafe { ffi::hal_sys_wdt_register(ms) }).map_err(|e| e.into_app_error("Watchdog register"))
    }

    fn watchdog_feed(&self) -> PlatformResult<()> {
        // SAFETY: no arguments; acts on the calling thread.
        HalError::check(unsafe { ffi::hal_sys_wdt_feed() }).map_err(|e| e.into_app_error("Watchdog feed"))
    }

    fn retained_read(&self, buf: &mut [u8]) -> PlatformResult<usize> {
        let mut len: usize = 0;
        // SAFETY: `buf` is valid for writes of `buf.len()` bytes and `len` is
        // a live local; the C side copies at most `cap` bytes.
        let rc = unsafe { ffi::hal_sys_retained_read(buf.as_mut_ptr(), buf.len(), &mut len) };
        HalError::check(rc).map_err(|e| e.into_app_error("Retained memory read"))?;
        Ok(len.min(buf.len()))
    }

    fn retained_write(&self, data: &[u8]) -> PlatformResult<()> {
        // SAFETY: `data` is valid for reads of `data.len()` bytes; the C side
        // copies it and keeps no pointer.
        let rc = unsafe { ffi::hal_sys_retained_write(data.as_ptr(), data.len()) };
        HalError::check(rc).map_err(|e| e.into_app_error("Retained memory write"))
    }
}
