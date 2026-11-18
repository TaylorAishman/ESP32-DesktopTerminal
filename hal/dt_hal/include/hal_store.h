/*
 * Small named blobs in flash (NVS) that survive power loss. Used for state
 * that must outlive restarts, such as API call counts.
 *
 * Keys are at most 15 characters. Thread safety: any thread; each call opens
 * and closes its own handle.
 */
#ifndef DT_HAL_STORE_H
#define DT_HAL_STORE_H

#include <stddef.h>
#include <stdint.h>

#include "hal_err.h"

#ifdef __cplusplus
extern "C" {
#endif

#define HAL_STORE_KEY_MAX 15

/* Read `key` into `buf`. A missing or unreadable entry is not an error:
 * `*len` is 0. HAL_ERR_INVALID_ARG if the entry is larger than `cap`. */
hal_err_t hal_store_get(const char *key, uint8_t *buf, size_t cap, size_t *len);

/* Write `len` bytes under `key`, replacing any previous value. */
hal_err_t hal_store_set(const char *key, const uint8_t *data, size_t len);

#ifdef __cplusplus
}
#endif

#endif /* DT_HAL_STORE_H */
