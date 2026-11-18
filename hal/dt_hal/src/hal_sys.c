#include "hal_sys.h"

#include "esp_heap_caps.h"
#include "esp_system.h"
#include "esp_timer.h"

uint64_t hal_sys_uptime_ms(void)
{
    return (uint64_t)(esp_timer_get_time() / 1000);
}

hal_err_t hal_sys_heap(uint32_t *free_bytes, uint32_t *min_free_bytes)
{
    if (free_bytes == NULL || min_free_bytes == NULL) {
        return HAL_ERR_INVALID_ARG;
    }
    *free_bytes = (uint32_t)heap_caps_get_free_size(MALLOC_CAP_8BIT);
    *min_free_bytes = (uint32_t)heap_caps_get_minimum_free_size(MALLOC_CAP_8BIT);
    return HAL_OK;
}

hal_reset_reason_t hal_sys_reset_reason(void)
{
    switch (esp_reset_reason()) {
    case ESP_RST_POWERON: return HAL_RESET_POWER_ON;
    case ESP_RST_SW: return HAL_RESET_SOFTWARE;
    case ESP_RST_PANIC: return HAL_RESET_PANIC;
    case ESP_RST_INT_WDT:
    case ESP_RST_TASK_WDT:
    case ESP_RST_WDT: return HAL_RESET_WATCHDOG;
    case ESP_RST_BROWNOUT: return HAL_RESET_BROWNOUT;
    case ESP_RST_EXT: return HAL_RESET_EXTERNAL;
    case ESP_RST_DEEPSLEEP: return HAL_RESET_DEEP_SLEEP;
    default: return HAL_RESET_UNKNOWN;
    }
}

void hal_sys_restart(void)
{
    esp_restart();
}
