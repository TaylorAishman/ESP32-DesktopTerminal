/*
 * Wi-Fi station. Once started, it reconnects by itself, forever, with
 * backoff between attempts.
 *
 * Thread safety: all functions may be called from any thread.
 */
#ifndef DT_HAL_WIFI_H
#define DT_HAL_WIFI_H

#include <stdint.h>

#include "hal_err.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    HAL_WIFI_IDLE = 0,         /* not started */
    HAL_WIFI_CONNECTING = 1,   /* attempting to join */
    HAL_WIFI_CONNECTED = 2,    /* joined, with an IPv4 address */
    HAL_WIFI_DISCONNECTED = 3, /* lost or failed; will retry */
} hal_wifi_state_t;

typedef struct {
    hal_wifi_state_t state;
    int8_t rssi;          /* dBm, valid when connected */
    uint32_t ipv4;        /* network byte order, valid when connected */
    hal_err_t last_error; /* why the last attempt failed; HAL_OK if none */
} hal_wifi_status_t;

/* Start the station and begin connecting. Copies both strings. An empty
 * password means an open network. Calling it again is an error. */
hal_err_t hal_wifi_start(const char *ssid, const char *password);

hal_err_t hal_wifi_status(hal_wifi_status_t *out);

#ifdef __cplusplus
}
#endif

#endif /* DT_HAL_WIFI_H */
