/*
 * Wall-clock time: SNTP synchronisation and the local time zone.
 *
 * Thread safety: all functions may be called from any thread.
 */
#ifndef DT_HAL_TIME_H
#define DT_HAL_TIME_H

#include <stdbool.h>
#include <stdint.h>

#include "hal_err.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Set the local time zone (POSIX TZ string, e.g. "EST5EDT,M3.2.0,M11.1.0")
 * and start periodic SNTP synchronisation (pool.ntp.org). Needs Wi-Fi
 * started; synchronisation happens once connected. Call once. */
hal_err_t hal_time_start_sync(const char *posix_tz);

/* Whether the clock has been set by SNTP since boot. */
bool hal_time_is_synced(void);

/* Seconds since the Unix epoch, UTC. HAL_ERR_NOT_INIT until synced. */
hal_err_t hal_time_now(int64_t *unix_seconds);

/* Local offset from UTC at `unix_seconds`, in seconds (DST included). */
hal_err_t hal_time_utc_offset(int64_t unix_seconds, int32_t *offset_seconds);

#ifdef __cplusplus
}
#endif

#endif /* DT_HAL_TIME_H */
