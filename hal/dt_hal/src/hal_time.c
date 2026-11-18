#include "hal_time.h"

#include <stdlib.h>
#include <string.h>
#include <sys/time.h>
#include <time.h>

#include "esp_netif_sntp.h"
#include "freertos/FreeRTOS.h"

#include "hal_internal.h"

static const char *TAG = "hal_time";

static portMUX_TYPE s_lock = portMUX_INITIALIZER_UNLOCKED;
static volatile bool s_synced;
static bool s_started;

static void on_sync(struct timeval *tv)
{
    (void)tv;
    bool first = !s_synced;
    s_synced = true;
    if (first) {
        ESP_LOGI(TAG, "clock synchronised");
    }
}

hal_err_t hal_time_start_sync(const char *posix_tz)
{
    if (posix_tz == NULL || posix_tz[0] == '\0') {
        return HAL_ERR_INVALID_ARG;
    }
    taskENTER_CRITICAL(&s_lock);
    bool already = s_started;
    s_started = true;
    taskEXIT_CRITICAL(&s_lock);
    if (already) {
        return HAL_ERR_INVALID_ARG;
    }
    setenv("TZ", posix_tz, 1);
    tzset();

    esp_sntp_config_t cfg = ESP_NETIF_SNTP_DEFAULT_CONFIG("pool.ntp.org");
    cfg.sync_cb = on_sync;
    HAL_CHECK(TAG, esp_netif_sntp_init(&cfg), HAL_ERR_INTERNAL);
    ESP_LOGI(TAG, "SNTP started");
    return HAL_OK;
}

bool hal_time_is_synced(void)
{
    return s_synced;
}

hal_err_t hal_time_now(int64_t *unix_seconds)
{
    if (unix_seconds == NULL) {
        return HAL_ERR_INVALID_ARG;
    }
    if (!s_synced) {
        return HAL_ERR_NOT_INIT;
    }
    struct timeval tv;
    gettimeofday(&tv, NULL);
    *unix_seconds = (int64_t)tv.tv_sec;
    return HAL_OK;
}

/* Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm). */
static int64_t days_from_civil(int64_t y, unsigned m, unsigned d)
{
    y -= m <= 2;
    const int64_t era = (y >= 0 ? y : y - 399) / 400;
    const unsigned yoe = (unsigned)(y - era * 400);
    const unsigned doy = (153 * (m + (m > 2 ? -3 : 9)) + 2) / 5 + d - 1;
    const unsigned doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    return era * 146097 + (int64_t)doe - 719468;
}

hal_err_t hal_time_utc_offset(int64_t unix_seconds, int32_t *offset_seconds)
{
    if (offset_seconds == NULL) {
        return HAL_ERR_INVALID_ARG;
    }
    time_t t = (time_t)unix_seconds;
    struct tm local;
    if (localtime_r(&t, &local) == NULL) {
        return HAL_ERR_INTERNAL;
    }
    /* The local broken-down time read back as if it were UTC, minus the
     * actual UTC time, is the offset. */
    int64_t as_utc = days_from_civil(local.tm_year + 1900, (unsigned)local.tm_mon + 1, (unsigned)local.tm_mday) * 86400 +
                     local.tm_hour * 3600 + local.tm_min * 60 + local.tm_sec;
    *offset_seconds = (int32_t)(as_utc - unix_seconds);
    return HAL_OK;
}
