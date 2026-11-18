#include "hal_sys.h"

#include <string.h>

#include "esp_attr.h"
#include "esp_heap_caps.h"
#include "esp_rom_crc.h"
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

#define RETAINED_MAGIC 0x44545231u /* "DTR1" */

typedef struct {
    uint32_t magic;
    uint32_t len;
    uint32_t crc;
    uint8_t data[HAL_SYS_RETAINED_MAX];
} retained_t;

/* Not initialised at boot, so it keeps its contents across software resets. */
static RTC_NOINIT_ATTR retained_t s_retained;

static uint32_t retained_crc(const retained_t *r)
{
    uint32_t crc = esp_rom_crc32_le(0, (const uint8_t *)&r->len, sizeof r->len);
    return esp_rom_crc32_le(crc, r->data, r->len);
}

hal_err_t hal_sys_retained_read(uint8_t *buf, size_t cap, size_t *len)
{
    if (len == NULL || (buf == NULL && cap > 0)) {
        return HAL_ERR_INVALID_ARG;
    }
    *len = 0;
    if (s_retained.magic != RETAINED_MAGIC || s_retained.len > HAL_SYS_RETAINED_MAX ||
        retained_crc(&s_retained) != s_retained.crc) {
        return HAL_OK; /* empty or corrupt: nothing stored */
    }
    size_t n = s_retained.len < cap ? s_retained.len : cap;
    memcpy(buf, s_retained.data, n);
    *len = s_retained.len;
    return HAL_OK;
}

hal_err_t hal_sys_retained_write(const uint8_t *data, size_t len)
{
    if (len > HAL_SYS_RETAINED_MAX || (data == NULL && len > 0)) {
        return HAL_ERR_INVALID_ARG;
    }
    s_retained.magic = 0; /* invalid while being written */
    s_retained.len = (uint32_t)len;
    memcpy(s_retained.data, data, len);
    s_retained.crc = retained_crc(&s_retained);
    s_retained.magic = RETAINED_MAGIC;
    return HAL_OK;
}
