/*
 * Low-level transport to the e-ink panel controller: SPI, reset, D/C and BUSY.
 * Private to the HAL; hal_display.c builds the panel protocol on top.
 *
 * Not thread-safe on its own: callers hold the display mutex.
 */
#ifndef DT_HAL_EPD_TRANSPORT_H
#define DT_HAL_EPD_TRANSPORT_H

#include <stddef.h>
#include <stdint.h>

#include "hal_err.h"

/* Configure SPI and the control pins. Safe to call more than once. */
hal_err_t epd_transport_init(void);

/* Pulse RST low to hardware-reset the controller. */
hal_err_t epd_reset(void);

/* Send one command byte (D/C low). */
hal_err_t epd_command(uint8_t cmd);

/* Send data bytes (D/C high). Splits large buffers into SPI transactions. */
hal_err_t epd_data(const uint8_t *data, size_t len);

/* Send a command followed by its parameter bytes. */
hal_err_t epd_command_data(uint8_t cmd, const uint8_t *data, size_t len);

/* Wait until the panel reports idle, for at most timeout_ms.
 * Returns HAL_ERR_TIMEOUT if it stays busy. */
hal_err_t epd_wait_idle(uint32_t timeout_ms);

/* Raw BUSY line level, for diagnostics: 1 = idle, 0 = busy. */
int epd_busy_level(void);

#endif /* DT_HAL_EPD_TRANSPORT_H */
