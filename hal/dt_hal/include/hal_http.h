/*
 * HTTPS GET with server certificate validation (ESP-IDF certificate bundle).
 *
 * Thread safety: may be called from any thread; each call is independent.
 * The application makes one request at a time (Architecture D-6).
 */
#ifndef DT_HAL_HTTP_H
#define DT_HAL_HTTP_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "hal_err.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    const char *name;
    const char *value;
} hal_http_header_t;

typedef struct {
    const char *url; /* https:// only; may contain an API key: never logged */
    const hal_http_header_t *headers;
    size_t header_count;
    uint32_t timeout_ms;
} hal_http_request_t;

typedef struct {
    int status;            /* HTTP status code */
    size_t body_len;       /* bytes written to the body buffer */
    bool truncated;        /* the body did not fit in the buffer */
    int32_t retry_after_s; /* Retry-After in seconds, or -1 if absent */
} hal_http_response_t;

/* Perform the request, writing up to `body_cap` bytes of the response body
 * into `body`. Any HTTP status is a response (HAL_OK); transport failures
 * are errors. */
hal_err_t hal_http_get(const hal_http_request_t *req, uint8_t *body, size_t body_cap, hal_http_response_t *resp);

#ifdef __cplusplus
}
#endif

#endif /* DT_HAL_HTTP_H */
