/*
 * System services: uptime, memory, reset reason, restart.
 *
 * Thread safety: all functions may be called from any thread.
 */
#ifndef DT_HAL_SYS_H
#define DT_HAL_SYS_H

#include <stddef.h>
#include <stdint.h>

#include "hal_err.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    HAL_RESET_UNKNOWN = 0,
    HAL_RESET_POWER_ON = 1,
    HAL_RESET_SOFTWARE = 2, /* hal_sys_restart() or esp_restart() */
    HAL_RESET_PANIC = 3,    /* exception or abort */
    HAL_RESET_WATCHDOG = 4, /* task, interrupt or other watchdog */
    HAL_RESET_BROWNOUT = 5,
    HAL_RESET_EXTERNAL = 6, /* EN/reset button */
    HAL_RESET_DEEP_SLEEP = 7,
} hal_reset_reason_t;

/* Milliseconds since boot. */
uint64_t hal_sys_uptime_ms(void);

/* Free heap now and the lowest it has been since boot, in bytes. */
hal_err_t hal_sys_heap(uint32_t *free_bytes, uint32_t *min_free_bytes);

/* Why the chip last reset. */
hal_reset_reason_t hal_sys_reset_reason(void);

/* Restart the chip. Does not return. */
void hal_sys_restart(void);

/*
 * Task watchdog. A thread that registers must call hal_sys_wdt_feed() at
 * least every `timeout_ms`, or the chip restarts (reset reason: watchdog).
 * The watchdog has one timeout for all threads: the largest requested.
 * Both act on the calling thread.
 */
hal_err_t hal_sys_wdt_register(uint32_t timeout_ms);
hal_err_t hal_sys_wdt_feed(void);

/* Largest block hal_sys_retained_write() accepts. */
#define HAL_SYS_RETAINED_MAX 256

/*
 * A small block of RTC memory that survives software resets, panics and
 * watchdog resets, but not power loss. Protected by a magic number and a CRC:
 * after power-on, or if the contents are corrupt, it reads as empty.
 *
 * hal_sys_retained_read copies up to `cap` bytes into `buf` and sets `*len`
 * to the stored length (0 when empty). Not thread-safe; call from one thread.
 */
hal_err_t hal_sys_retained_read(uint8_t *buf, size_t cap, size_t *len);
hal_err_t hal_sys_retained_write(const uint8_t *data, size_t len);

#ifdef __cplusplus
}
#endif

#endif /* DT_HAL_SYS_H */
