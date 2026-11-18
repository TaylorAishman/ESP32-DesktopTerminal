/*
 * Error codes returned by every HAL function.
 *
 * Values are stable: dt-hal maps each one to a Rust error, so append new codes
 * at the end and never renumber.
 */
#ifndef DT_HAL_ERR_H
#define DT_HAL_ERR_H

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    HAL_OK = 0,
    HAL_ERR_INVALID_ARG = 1,       /* bad pointer, length or value */
    HAL_ERR_NOT_INIT = 2,          /* module used before its init function */
    HAL_ERR_NO_MEM = 3,            /* allocation inside ESP-IDF failed */
    HAL_ERR_TIMEOUT = 4,           /* a bounded wait expired */
    HAL_ERR_IO = 5,                /* bus or peripheral failure */
    HAL_ERR_WIFI_AUTH = 6,         /* access point rejected the password */
    HAL_ERR_WIFI_NO_AP = 7,        /* network not found */
    HAL_ERR_WIFI_DISCONNECTED = 8, /* not connected (other or unknown reason) */
    HAL_ERR_DNS = 9,               /* host name could not be resolved */
    HAL_ERR_CONNECT = 10,          /* TCP connection failed */
    HAL_ERR_TLS = 11,              /* TLS handshake or certificate check failed */
    HAL_ERR_HTTP_TOO_LARGE = 12,   /* response headers did not fit */
    HAL_ERR_NOT_FOUND = 13,        /* requested item does not exist */
    HAL_ERR_INTERNAL = 14,         /* unexpected ESP-IDF error; see the log */
} hal_err_t;

/* Short name of an error code, e.g. "HAL_ERR_TIMEOUT". Never NULL. */
const char *hal_err_str(hal_err_t err);

#ifdef __cplusplus
}
#endif

#endif /* DT_HAL_ERR_H */
