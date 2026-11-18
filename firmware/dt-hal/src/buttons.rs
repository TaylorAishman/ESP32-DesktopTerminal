//! Front-panel buttons, implementing [`dt_core::platform::Buttons`].

use core::time::Duration;

use dt_core::platform::{Button, ButtonAction, ButtonEvent, Buttons, PlatformResult};
use esp_idf_sys::dt_hal as ffi;

use crate::HalError;

/// The four buttons. Creating it starts the HAL's sampling task.
#[derive(Debug)]
pub struct HalButtons {
    _private: (),
}

impl HalButtons {
    pub fn new() -> PlatformResult<Self> {
        // SAFETY: no arguments; idempotent.
        HalError::check(unsafe { ffi::hal_buttons_init() }).map_err(|e| e.into_app_error("Buttons init"))?;
        Ok(HalButtons { _private: () })
    }

    /// Buttons held down right now.
    pub fn held(&self) -> Vec<Button> {
        let mut mask = 0u8;
        // SAFETY: `mask` is a live, writable local.
        if HalError::check(unsafe { ffi::hal_buttons_held(&mut mask) }).is_err() {
            return Vec::new();
        }
        Button::ALL.into_iter().filter(|b| mask & (1 << (b.number() - 1)) != 0).collect()
    }
}

impl Buttons for HalButtons {
    fn wait(&mut self, timeout: Duration) -> PlatformResult<Option<ButtonEvent>> {
        let mut e = ffi::hal_button_event_t {
            button: 0,
            action: ffi::hal_button_action_t_HAL_BTN_PRESS,
        };
        let ms = timeout.as_millis().min(u128::from(u32::MAX)) as u32;
        // SAFETY: `e` is a live, writable local of the expected type.
        match HalError::check(unsafe { ffi::hal_buttons_wait(&mut e, ms) }) {
            Ok(()) => Ok(Button::from_number(e.button).map(|button| ButtonEvent {
                button,
                action: if e.action == ffi::hal_button_action_t_HAL_BTN_LONG_PRESS {
                    ButtonAction::LongPress
                } else {
                    ButtonAction::Press
                },
            })),
            Err(HalError::Timeout) => Ok(None),
            Err(other) => Err(other.into_app_error("Buttons")),
        }
    }
}
