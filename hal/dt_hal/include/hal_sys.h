/*
 * System services: uptime, memory, reset reason, restart.
 *
 * Thread safety: all functions may be called from any thread.
 */
#ifndef DT_HAL_SYS_H
#define DT_HAL_SYS_H

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

#ifdef __cplusplus
}
#endif

#endif /* DT_HAL_SYS_H */
