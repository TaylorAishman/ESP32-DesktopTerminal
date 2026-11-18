#include "nvs_init.h"

#include <stdbool.h>

#include "freertos/FreeRTOS.h"
#include "nvs_flash.h"

#include "hal_internal.h"

static const char *TAG = "hal_nvs";
static bool s_done;
static portMUX_TYPE s_lock = portMUX_INITIALIZER_UNLOCKED;

hal_err_t hal_nvs_init(void)
{
    taskENTER_CRITICAL(&s_lock);
    bool done = s_done;
    taskEXIT_CRITICAL(&s_lock);
    if (done) {
        return HAL_OK;
    }
    esp_err_t err = nvs_flash_init();
    if (err == ESP_ERR_NVS_NO_FREE_PAGES || err == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_LOGW(TAG, "NVS partition unusable (%s); erasing", esp_err_to_name(err));
        HAL_CHECK(TAG, nvs_flash_erase(), HAL_ERR_IO);
        err = nvs_flash_init();
    }
    HAL_CHECK(TAG, err, HAL_ERR_IO);
    taskENTER_CRITICAL(&s_lock);
    s_done = true;
    taskEXIT_CRITICAL(&s_lock);
    return HAL_OK;
}
