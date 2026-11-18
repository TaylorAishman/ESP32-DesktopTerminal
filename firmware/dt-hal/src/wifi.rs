//! Wi-Fi station, implementing [`dt_core::platform::Wifi`].

use core::time::Duration;
use std::ffi::CString;

use dt_core::platform::{PlatformResult, Wifi, WifiState, WifiStatus};
use dt_core::{AppError, ErrorCode};
use esp_idf_sys::dt_hal as ffi;

use crate::HalError;

/// The ESP32's Wi-Fi in station mode. Reconnects by itself once started.
#[derive(Debug, Default, Clone, Copy)]
pub struct HalWifi;

impl Wifi for HalWifi {
    fn start(&self, ssid: &str, password: &str) -> PlatformResult<()> {
        let bad = || AppError::with_message(ErrorCode::ConfigInvalid, "Wi-Fi name or password contains a NUL character.");
        let ssid = CString::new(ssid).map_err(|_| bad())?;
        let password = CString::new(password).map_err(|_| bad())?;
        // SAFETY: both pointers are valid NUL-terminated strings for the call;
        // the C side copies them.
        let rc = unsafe { ffi::hal_wifi_start(ssid.as_ptr(), password.as_ptr()) };
        HalError::check(rc).map_err(|e| e.into_app_error("Wi-Fi start"))
    }

    fn status(&self) -> WifiStatus {
        let mut s = ffi::hal_wifi_status_t {
            state: ffi::hal_wifi_state_t_HAL_WIFI_IDLE,
            rssi: 0,
            ipv4: 0,
            last_error: ffi::hal_err_t_HAL_OK,
        };
        // SAFETY: `s` is a live, writable local of the expected type.
        if HalError::check(unsafe { ffi::hal_wifi_status(&mut s) }).is_err() {
            return WifiStatus {
                state: WifiState::Idle,
                rssi: None,
                ip: None,
                last_error: None,
            };
        }
        let state = match s.state {
            ffi::hal_wifi_state_t_HAL_WIFI_CONNECTING => WifiState::Connecting,
            ffi::hal_wifi_state_t_HAL_WIFI_CONNECTED => WifiState::Connected,
            ffi::hal_wifi_state_t_HAL_WIFI_DISCONNECTED => WifiState::Disconnected,
            _ => WifiState::Idle,
        };
        let connected = state == WifiState::Connected;
        WifiStatus {
            state,
            rssi: connected.then_some(s.rssi),
            ip: connected.then(|| s.ipv4.to_le_bytes()),
            last_error: HalError::from_code(s.last_error).map(wifi_error),
        }
    }

    fn wait_connected(&self, timeout: Duration) -> PlatformResult<()> {
        let ms = timeout.as_millis().min(u128::from(u32::MAX)) as u32;
        // SAFETY: plain value argument.
        let rc = unsafe { ffi::hal_wifi_wait_connected(ms) };
        HalError::check(rc).map_err(wifi_error)
    }
}

fn wifi_error(e: HalError) -> AppError {
    match e {
        HalError::WifiAuth => AppError::with_message(ErrorCode::WifiAuth, "The Wi-Fi network rejected the password."),
        HalError::WifiNoAp => AppError::with_message(ErrorCode::WifiNoAp, "The Wi-Fi network was not found."),
        HalError::WifiDisconnected => AppError::with_message(ErrorCode::WifiDisconnected, "Not connected to Wi-Fi."),
        HalError::Timeout => AppError::with_message(ErrorCode::NetOffline, "Still connecting to Wi-Fi."),
        other => other.into_app_error("Wi-Fi"),
    }
}
