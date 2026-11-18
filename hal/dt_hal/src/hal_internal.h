/* Helpers shared by HAL sources. Not part of the public API. */
#ifndef DT_HAL_INTERNAL_H
#define DT_HAL_INTERNAL_H

#include "esp_err.h"
#include "esp_log.h"
#include "hal_err.h"

/*
 * Log an unexpected ESP-IDF error with its location and map it to a HAL code.
 * Used instead of ESP_ERROR_CHECK, which would abort (HAL rule 6).
 */
#define HAL_CHECK(tag, expr, on_error)                                         \
    do {                                                                       \
        esp_err_t err_rc_ = (expr);                                            \
        if (err_rc_ != ESP_OK) {                                               \
            ESP_LOGE(tag, "%s failed: %s (%s:%d)", #expr,                      \
                     esp_err_to_name(err_rc_), __FILE__, __LINE__);            \
            return (on_error);                                                 \
        }                                                                      \
    } while (0)

/* Map a generic ESP-IDF error to the closest HAL code. */
static inline hal_err_t hal_from_esp(esp_err_t err)
{
    switch (err) {
    case ESP_OK: return HAL_OK;
    case ESP_ERR_INVALID_ARG:
    case ESP_ERR_INVALID_SIZE: return HAL_ERR_INVALID_ARG;
    case ESP_ERR_NO_MEM: return HAL_ERR_NO_MEM;
    case ESP_ERR_TIMEOUT: return HAL_ERR_TIMEOUT;
    case ESP_ERR_INVALID_STATE: return HAL_ERR_NOT_INIT;
    case ESP_ERR_NOT_FOUND: return HAL_ERR_NOT_FOUND;
    default: return HAL_ERR_INTERNAL;
    }
}

#endif /* DT_HAL_INTERNAL_H */
