/*
 * Desktop terminal hardware abstraction layer.
 *
 * Umbrella header: includes every HAL module. This file is also the input to
 * bindgen, so everything declared here is visible to Rust (dt-hal).
 */
#ifndef DT_HAL_H
#define DT_HAL_H

#ifdef __cplusplus
extern "C" {
#endif

/* Version of the HAL, e.g. "0.1.0". Never NULL; static storage. */
const char *hal_version(void);

#ifdef __cplusplus
}
#endif

#endif /* DT_HAL_H */
