//! Errors returned by the C HAL, and their mapping to [`AppError`].

use core::fmt;

use dt_core::{AppError, ErrorCode};
use esp_idf_sys::dt_hal as ffi;

/// A `hal_err_t` other than `HAL_OK`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HalError {
    InvalidArg,
    NotInit,
    NoMem,
    Timeout,
    Io,
    WifiAuth,
    WifiNoAp,
    WifiDisconnected,
    Dns,
    Connect,
    Tls,
    HttpTooLarge,
    NotFound,
    Internal,
    /// A code this crate does not know: the C side is newer than the Rust side.
    Unknown(u32),
}

impl HalError {
    /// `Ok(())` for `HAL_OK`, otherwise the matching error.
    pub fn check(rc: ffi::hal_err_t) -> Result<(), HalError> {
        Self::from_code(rc).map_or(Ok(()), Err)
    }

    /// `None` for `HAL_OK`.
    pub fn from_code(rc: ffi::hal_err_t) -> Option<HalError> {
        use HalError::*;
        Some(match rc {
            ffi::hal_err_t_HAL_OK => return None,
            ffi::hal_err_t_HAL_ERR_INVALID_ARG => InvalidArg,
            ffi::hal_err_t_HAL_ERR_NOT_INIT => NotInit,
            ffi::hal_err_t_HAL_ERR_NO_MEM => NoMem,
            ffi::hal_err_t_HAL_ERR_TIMEOUT => Timeout,
            ffi::hal_err_t_HAL_ERR_IO => Io,
            ffi::hal_err_t_HAL_ERR_WIFI_AUTH => WifiAuth,
            ffi::hal_err_t_HAL_ERR_WIFI_NO_AP => WifiNoAp,
            ffi::hal_err_t_HAL_ERR_WIFI_DISCONNECTED => WifiDisconnected,
            ffi::hal_err_t_HAL_ERR_DNS => Dns,
            ffi::hal_err_t_HAL_ERR_CONNECT => Connect,
            ffi::hal_err_t_HAL_ERR_TLS => Tls,
            ffi::hal_err_t_HAL_ERR_HTTP_TOO_LARGE => HttpTooLarge,
            ffi::hal_err_t_HAL_ERR_NOT_FOUND => NotFound,
            ffi::hal_err_t_HAL_ERR_INTERNAL => Internal,
            other => Unknown(other),
        })
    }

    /// The C name of the code, e.g. `HAL_ERR_TIMEOUT`.
    pub fn name(self) -> &'static str {
        use HalError::*;
        match self {
            InvalidArg => "HAL_ERR_INVALID_ARG",
            NotInit => "HAL_ERR_NOT_INIT",
            NoMem => "HAL_ERR_NO_MEM",
            Timeout => "HAL_ERR_TIMEOUT",
            Io => "HAL_ERR_IO",
            WifiAuth => "HAL_ERR_WIFI_AUTH",
            WifiNoAp => "HAL_ERR_WIFI_NO_AP",
            WifiDisconnected => "HAL_ERR_WIFI_DISCONNECTED",
            Dns => "HAL_ERR_DNS",
            Connect => "HAL_ERR_CONNECT",
            Tls => "HAL_ERR_TLS",
            HttpTooLarge => "HAL_ERR_HTTP_TOO_LARGE",
            NotFound => "HAL_ERR_NOT_FOUND",
            Internal => "HAL_ERR_INTERNAL",
            Unknown(_) => "HAL_ERR_UNKNOWN",
        }
    }

    /// Default application error. Wrappers whose subsystem gives a code a
    /// more specific meaning (a display timeout, say) map it themselves.
    pub fn into_app_error(self, operation: &str) -> AppError {
        use HalError::*;
        let code = match self {
            WifiAuth => ErrorCode::WifiAuth,
            WifiNoAp => ErrorCode::WifiNoAp,
            WifiDisconnected => ErrorCode::WifiDisconnected,
            Dns => ErrorCode::NetDns,
            Connect => ErrorCode::NetConnect,
            Tls => ErrorCode::NetTls,
            Timeout => ErrorCode::NetTimeout,
            NoMem => ErrorCode::SysLowMemory,
            HttpTooLarge => ErrorCode::ApiResponseTooLarge,
            InvalidArg | NotInit | Io | NotFound | Internal | Unknown(_) => ErrorCode::SysInternal,
        };
        AppError::with_message(code, format!("{operation} failed: {self}."))
    }
}

impl fmt::Display for HalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HalError::Unknown(code) => write!(f, "HAL_ERR_UNKNOWN({code})"),
            other => f.write_str(other.name()),
        }
    }
}

impl std::error::Error for HalError {}

/// Check that every code in `hal_err.h` maps to a known [`HalError`] whose
/// name matches the C side's `hal_err_str`. Run by the on-device self-test,
/// since this crate's tests cannot run on the host.
pub fn verify_codes_match_c() -> Result<(), String> {
    /// `HAL_ERR_INTERNAL`, the highest code in hal_err.h.
    const LAST: ffi::hal_err_t = ffi::hal_err_t_HAL_ERR_INTERNAL;
    if HalError::from_code(ffi::hal_err_t_HAL_OK).is_some() {
        return Err("HAL_OK maps to an error".into());
    }
    for rc in 1..=LAST {
        let e = HalError::from_code(rc).ok_or_else(|| format!("code {rc} maps to OK"))?;
        if let HalError::Unknown(_) = e {
            return Err(format!("code {rc} is not mapped"));
        }
        // SAFETY: hal_err_str accepts any value and returns a pointer to a
        // static NUL-terminated string; it never returns NULL.
        let c_name = unsafe { core::ffi::CStr::from_ptr(ffi::hal_err_str(rc)) };
        if c_name.to_str() != Ok(e.name()) {
            return Err(format!("code {rc}: C says {c_name:?}, Rust says {}", e.name()));
        }
    }
    Ok(())
}
