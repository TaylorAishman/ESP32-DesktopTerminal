#include "hal_err.h"

const char *hal_err_str(hal_err_t err)
{
    switch (err) {
    case HAL_OK: return "HAL_OK";
    case HAL_ERR_INVALID_ARG: return "HAL_ERR_INVALID_ARG";
    case HAL_ERR_NOT_INIT: return "HAL_ERR_NOT_INIT";
    case HAL_ERR_NO_MEM: return "HAL_ERR_NO_MEM";
    case HAL_ERR_TIMEOUT: return "HAL_ERR_TIMEOUT";
    case HAL_ERR_IO: return "HAL_ERR_IO";
    case HAL_ERR_WIFI_AUTH: return "HAL_ERR_WIFI_AUTH";
    case HAL_ERR_WIFI_NO_AP: return "HAL_ERR_WIFI_NO_AP";
    case HAL_ERR_WIFI_DISCONNECTED: return "HAL_ERR_WIFI_DISCONNECTED";
    case HAL_ERR_DNS: return "HAL_ERR_DNS";
    case HAL_ERR_CONNECT: return "HAL_ERR_CONNECT";
    case HAL_ERR_TLS: return "HAL_ERR_TLS";
    case HAL_ERR_HTTP_TOO_LARGE: return "HAL_ERR_HTTP_TOO_LARGE";
    case HAL_ERR_NOT_FOUND: return "HAL_ERR_NOT_FOUND";
    case HAL_ERR_INTERNAL: return "HAL_ERR_INTERNAL";
    }
    return "HAL_ERR_UNKNOWN";
}
