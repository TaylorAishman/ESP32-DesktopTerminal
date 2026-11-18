/*
 * E-ink panel: Waveshare 7.5" 800x480 black/white (UC8179 controller).
 *
 * Frame layout: 800x480, 1 bit per pixel, rows top to bottom, 100 bytes per
 * row, most significant bit = leftmost pixel, set bit = black. 48 000 bytes.
 *
 * Thread safety: all functions may be called from any thread; calls are
 * serialised internally. Each call blocks until the panel is done.
 */
#ifndef DT_HAL_DISPLAY_H
#define DT_HAL_DISPLAY_H

#include <stddef.h>
#include <stdint.h>

#include "hal_err.h"

#ifdef __cplusplus
extern "C" {
#endif

#define HAL_DISPLAY_WIDTH 800
#define HAL_DISPLAY_HEIGHT 480
#define HAL_DISPLAY_FRAME_BYTES (HAL_DISPLAY_WIDTH / 8 * HAL_DISPLAY_HEIGHT)

/* Set up SPI and pins, reset the controller, run the init sequence and power
 * the panel on. Called again after a failure, it starts from scratch. */
hal_err_t hal_display_init(void);

/* Show a full frame: wake the panel if needed, transfer, run a full refresh
 * (several seconds) and put the panel back into deep sleep. len must be
 * HAL_DISPLAY_FRAME_BYTES. */
hal_err_t hal_display_show(const uint8_t *frame, size_t len);

/* Clear the panel to white with a full refresh, then sleep. */
hal_err_t hal_display_clear(void);

/* Power the panel off and put the controller into deep sleep. The next
 * hal_display_init() wakes it with a hardware reset. */
hal_err_t hal_display_sleep(void);

#ifdef __cplusplus
}
#endif

#endif /* DT_HAL_DISPLAY_H */
