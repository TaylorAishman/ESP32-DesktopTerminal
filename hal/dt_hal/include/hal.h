/*
 * Desktop terminal hardware abstraction layer.
 *
 * Umbrella header: includes every HAL module. It is also the input to bindgen,
 * so everything declared here is visible to Rust (firmware/dt-hal).
 *
 * Rules every module follows (Architecture §4.1):
 *  1. Every function that can fail returns hal_err_t; results come back
 *     through out-parameters.
 *  2. Calls block, with an explicit timeout wherever they can wait.
 *  3. The caller allocates every buffer. No heap pointer crosses the boundary.
 *  4. No callbacks into Rust.
 *  5. Each module states which threads may call it and serialises its own
 *     hardware access.
 *  6. Runtime paths never abort: no ESP_ERROR_CHECK outside one-time init that
 *     cannot fail on working hardware. Use HAL_CHECK instead.
 *  7. Pin numbers live only in hal_pins.h.
 */
#ifndef DT_HAL_H
#define DT_HAL_H

#include "hal_display.h"
#include "hal_err.h"
#include "hal_pins.h"
#include "hal_sys.h"
#include "hal_wifi.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Version of the HAL, e.g. "0.1.0". Never NULL; static storage. */
const char *hal_version(void);

#ifdef __cplusplus
}
#endif

#endif /* DT_HAL_H */
