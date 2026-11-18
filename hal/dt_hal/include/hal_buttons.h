/*
 * The four front-panel buttons (hal_pins.h): active low with internal
 * pull-ups. A background task samples them every 10 ms, debounces, and
 * queues events: PRESS on release after a short press, LONG_PRESS once a
 * button has been held for 1 s (no PRESS follows it).
 *
 * Thread safety: hal_buttons_wait from one consumer thread; the rest from
 * any thread.
 */
#ifndef DT_HAL_BUTTONS_H
#define DT_HAL_BUTTONS_H

#include <stdint.h>

#include "hal_err.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    HAL_BTN_PRESS = 0,
    HAL_BTN_LONG_PRESS = 1,
} hal_button_action_t;

typedef struct {
    uint8_t button; /* 1 to 4 */
    hal_button_action_t action;
} hal_button_event_t;

/* Configure the pins and start the sampling task. Safe to call again. */
hal_err_t hal_buttons_init(void);

/* Wait up to `timeout_ms` for the next event; HAL_ERR_TIMEOUT if none. */
hal_err_t hal_buttons_wait(hal_button_event_t *out, uint32_t timeout_ms);

/* Buttons held down right now, as a bit mask (bit 0 = button 1). */
hal_err_t hal_buttons_held(uint8_t *mask);

#ifdef __cplusplus
}
#endif

#endif /* DT_HAL_BUTTONS_H */
