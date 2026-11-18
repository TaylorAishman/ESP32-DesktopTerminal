#include "hal_store.h"

#include <string.h>

#include "nvs.h"

#include "hal_internal.h"
#include "nvs_init.h"

static const char *TAG = "hal_store";
static const char *NAMESPACE = "dt";

static bool key_ok(const char *key)
{
    return key != NULL && key[0] != '\0' && strlen(key) <= HAL_STORE_KEY_MAX;
}

hal_err_t hal_store_get(const char *key, uint8_t *buf, size_t cap, size_t *len)
{
    if (!key_ok(key) || len == NULL || (buf == NULL && cap > 0)) {
        return HAL_ERR_INVALID_ARG;
    }
    *len = 0;
    hal_err_t herr = hal_nvs_init();
    if (herr != HAL_OK) {
        return herr;
    }
    nvs_handle_t h;
    esp_err_t err = nvs_open(NAMESPACE, NVS_READONLY, &h);
    if (err == ESP_ERR_NVS_NOT_FOUND) {
        return HAL_OK; /* nothing ever written */
    }
    HAL_CHECK(TAG, err, HAL_ERR_IO);
    size_t size = cap;
    err = nvs_get_blob(h, key, buf, &size);
    nvs_close(h);
    switch (err) {
    case ESP_OK:
        *len = size;
        return HAL_OK;
    case ESP_ERR_NVS_NOT_FOUND:
        return HAL_OK;
    case ESP_ERR_NVS_INVALID_LENGTH:
        return HAL_ERR_INVALID_ARG;
    default:
        ESP_LOGW(TAG, "read '%s': %s; treating as empty", key, esp_err_to_name(err));
        return HAL_OK;
    }
}

hal_err_t hal_store_set(const char *key, const uint8_t *data, size_t len)
{
    if (!key_ok(key) || (data == NULL && len > 0)) {
        return HAL_ERR_INVALID_ARG;
    }
    hal_err_t herr = hal_nvs_init();
    if (herr != HAL_OK) {
        return herr;
    }
    nvs_handle_t h;
    HAL_CHECK(TAG, nvs_open(NAMESPACE, NVS_READWRITE, &h), HAL_ERR_IO);
    esp_err_t err = nvs_set_blob(h, key, data, len);
    if (err == ESP_OK) {
        err = nvs_commit(h);
    }
    nvs_close(h);
    if (err != ESP_OK) {
        ESP_LOGW(TAG, "write '%s': %s", key, esp_err_to_name(err));
        return err == ESP_ERR_NVS_NOT_ENOUGH_SPACE ? HAL_ERR_NO_MEM : HAL_ERR_IO;
    }
    return HAL_OK;
}
