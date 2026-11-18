//! Persistent blobs, implementing [`dt_core::platform::Storage`].

use std::ffi::CString;

use dt_core::platform::{PlatformResult, Storage};
use dt_core::{AppError, ErrorCode};
use esp_idf_sys::dt_hal as ffi;

use crate::HalError;

/// Named blobs in NVS flash; survive power loss.
#[derive(Debug, Default, Clone, Copy)]
pub struct HalStore;

fn key(k: &str) -> PlatformResult<CString> {
    if k.is_empty() || k.len() > ffi::HAL_STORE_KEY_MAX as usize {
        return Err(AppError::with_message(ErrorCode::SysInternal, format!("Storage key '{k}' must be 1 to 15 characters.")));
    }
    CString::new(k).map_err(|_| AppError::with_message(ErrorCode::SysInternal, "Storage key contains a NUL character."))
}

impl Storage for HalStore {
    fn get(&self, k: &str, buf: &mut [u8]) -> PlatformResult<Option<usize>> {
        let k = key(k)?;
        let mut len = 0usize;
        // SAFETY: `k` is NUL-terminated; `buf` is valid for writes of
        // `buf.len()` bytes; `len` is a live local.
        let rc = unsafe { ffi::hal_store_get(k.as_ptr(), buf.as_mut_ptr(), buf.len(), &mut len) };
        match HalError::check(rc) {
            Ok(()) if len > 0 => Ok(Some(len.min(buf.len()))),
            Ok(()) => Ok(None),
            // Stored value larger than the buffer: treat as unreadable.
            Err(HalError::InvalidArg) => Ok(None),
            Err(e) => Err(e.into_app_error("Storage read")),
        }
    }

    fn set(&self, k: &str, data: &[u8]) -> PlatformResult<()> {
        let k = key(k)?;
        // SAFETY: `k` is NUL-terminated; `data` is valid for reads of
        // `data.len()` bytes; the C side copies it.
        let rc = unsafe { ffi::hal_store_set(k.as_ptr(), data.as_ptr(), data.len()) };
        HalError::check(rc).map_err(|e| e.into_app_error("Storage write"))
    }
}
