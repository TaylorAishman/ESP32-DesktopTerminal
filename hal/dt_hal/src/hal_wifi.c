#include "hal_wifi.h"

#include <stdbool.h>
#include <string.h>

#include "esp_event.h"
#include "esp_netif.h"
#include "esp_timer.h"
#include "esp_wifi.h"
#include "freertos/FreeRTOS.h"
#include "freertos/event_groups.h"

#include "hal_internal.h"
#include "nvs_init.h"

static const char *TAG = "hal_wifi";

/* Reconnect delays: 1 s doubling to at most 60 s. */
#define BACKOFF_MIN_MS 1000
#define BACKOFF_MAX_MS 60000

#define BIT_CONNECTED BIT0

static portMUX_TYPE s_lock = portMUX_INITIALIZER_UNLOCKED;
static bool s_started;
static hal_wifi_state_t s_state = HAL_WIFI_IDLE;
static uint32_t s_ipv4;
static hal_err_t s_last_error = HAL_OK;
static uint32_t s_backoff_ms = BACKOFF_MIN_MS;
static esp_timer_handle_t s_retry_timer;
static EventGroupHandle_t s_events;

static void set_state(hal_wifi_state_t state, hal_err_t err)
{
    taskENTER_CRITICAL(&s_lock);
    s_state = state;
    if (state != HAL_WIFI_CONNECTED) {
        s_ipv4 = 0;
    }
    if (state == HAL_WIFI_CONNECTED) {
        s_last_error = HAL_OK;
    } else if (err != HAL_OK) {
        /* Until the next successful connection, a generic failure does not
         * hide a specific one: a rejected password often alternates with
         * plain connection failures while the access point backs off. */
        bool specific = s_last_error == HAL_ERR_WIFI_AUTH || s_last_error == HAL_ERR_WIFI_NO_AP;
        if (!(specific && err == HAL_ERR_WIFI_DISCONNECTED)) {
            s_last_error = err;
        }
    }
    taskEXIT_CRITICAL(&s_lock);
}

static void retry_cb(void *arg)
{
    (void)arg;
    set_state(HAL_WIFI_CONNECTING, HAL_OK);
    esp_err_t err = esp_wifi_connect();
    if (err != ESP_OK) {
        ESP_LOGW(TAG, "connect: %s", esp_err_to_name(err));
    }
}

/* Map a disconnect reason to a HAL error. */
hal_err_t hal_wifi_reason(uint8_t reason);

static void on_event(void *arg, esp_event_base_t base, int32_t id, void *data)
{
    (void)arg;
    if (base == WIFI_EVENT && id == WIFI_EVENT_STA_START) {
        set_state(HAL_WIFI_CONNECTING, HAL_OK);
        esp_wifi_connect();
    } else if (base == WIFI_EVENT && id == WIFI_EVENT_STA_DISCONNECTED) {
        const wifi_event_sta_disconnected_t *d = data;
        hal_err_t why = hal_wifi_reason(d->reason);
        xEventGroupClearBits(s_events, BIT_CONNECTED);
        set_state(HAL_WIFI_DISCONNECTED, why);
        uint32_t delay = s_backoff_ms;
        s_backoff_ms = s_backoff_ms * 2 > BACKOFF_MAX_MS ? BACKOFF_MAX_MS : s_backoff_ms * 2;
        ESP_LOGW(TAG, "disconnected (reason %u: %s); retrying in %u ms", d->reason, hal_err_str(why), (unsigned)delay);
        esp_timer_stop(s_retry_timer);
        esp_timer_start_once(s_retry_timer, (uint64_t)delay * 1000);
    } else if (base == IP_EVENT && id == IP_EVENT_STA_GOT_IP) {
        const ip_event_got_ip_t *e = data;
        taskENTER_CRITICAL(&s_lock);
        s_ipv4 = e->ip_info.ip.addr;
        taskEXIT_CRITICAL(&s_lock);
        set_state(HAL_WIFI_CONNECTED, HAL_OK);
        s_backoff_ms = BACKOFF_MIN_MS;
        xEventGroupSetBits(s_events, BIT_CONNECTED);
        ESP_LOGI(TAG, "connected, IP " IPSTR, IP2STR(&e->ip_info.ip));
    }
}

hal_err_t hal_wifi_reason(uint8_t reason)
{
    switch (reason) {
    /* The access point rejected our credentials. */
    case WIFI_REASON_AUTH_EXPIRE:
    case WIFI_REASON_MIC_FAILURE:
    case WIFI_REASON_4WAY_HANDSHAKE_TIMEOUT:
    case WIFI_REASON_802_1X_AUTH_FAILED:
    case WIFI_REASON_AUTH_FAIL:
    case WIFI_REASON_HANDSHAKE_TIMEOUT:
        return HAL_ERR_WIFI_AUTH;
    /* No network with that name (and acceptable security) in range. */
    case WIFI_REASON_NO_AP_FOUND:
    case WIFI_REASON_NO_AP_FOUND_W_COMPATIBLE_SECURITY:
    case WIFI_REASON_NO_AP_FOUND_IN_AUTHMODE_THRESHOLD:
    case WIFI_REASON_NO_AP_FOUND_IN_RSSI_THRESHOLD:
        return HAL_ERR_WIFI_NO_AP;
    default:
        return HAL_ERR_WIFI_DISCONNECTED;
    }
}

hal_err_t hal_wifi_wait_connected(uint32_t timeout_ms)
{
    if (s_events == NULL) {
        return HAL_ERR_NOT_INIT;
    }
    EventBits_t bits = xEventGroupWaitBits(s_events, BIT_CONNECTED, pdFALSE, pdTRUE, pdMS_TO_TICKS(timeout_ms));
    if (bits & BIT_CONNECTED) {
        return HAL_OK;
    }
    taskENTER_CRITICAL(&s_lock);
    hal_err_t last = s_last_error;
    taskEXIT_CRITICAL(&s_lock);
    return last != HAL_OK ? last : HAL_ERR_TIMEOUT;
}

hal_err_t hal_wifi_start(const char *ssid, const char *password)
{
    if (ssid == NULL || password == NULL || strlen(ssid) == 0 || strlen(ssid) > 32 || strlen(password) > 63) {
        return HAL_ERR_INVALID_ARG;
    }
    taskENTER_CRITICAL(&s_lock);
    bool already = s_started;
    s_started = true;
    taskEXIT_CRITICAL(&s_lock);
    if (already) {
        return HAL_ERR_INVALID_ARG;
    }

    /* The Wi-Fi driver logs the SSID at INFO; keep credentials out of logs. */
    esp_log_level_set("wifi", ESP_LOG_WARN);
    esp_log_level_set("wifi_init", ESP_LOG_WARN);

    hal_err_t herr = hal_nvs_init();
    if (herr != HAL_OK) {
        return herr;
    }
    s_events = xEventGroupCreate();
    if (s_events == NULL) {
        return HAL_ERR_NO_MEM;
    }
    HAL_CHECK(TAG, esp_netif_init(), HAL_ERR_INTERNAL);
    esp_err_t err = esp_event_loop_create_default();
    if (err != ESP_OK && err != ESP_ERR_INVALID_STATE) {
        HAL_CHECK(TAG, err, HAL_ERR_INTERNAL);
    }
    if (esp_netif_create_default_wifi_sta() == NULL) {
        return HAL_ERR_NO_MEM;
    }
    const esp_timer_create_args_t targs = {.callback = retry_cb, .name = "wifi_retry"};
    HAL_CHECK(TAG, esp_timer_create(&targs, &s_retry_timer), HAL_ERR_NO_MEM);

    wifi_init_config_t init = WIFI_INIT_CONFIG_DEFAULT();
    HAL_CHECK(TAG, esp_wifi_init(&init), HAL_ERR_NO_MEM);
    HAL_CHECK(TAG, esp_event_handler_register(WIFI_EVENT, ESP_EVENT_ANY_ID, on_event, NULL), HAL_ERR_INTERNAL);
    HAL_CHECK(TAG, esp_event_handler_register(IP_EVENT, IP_EVENT_STA_GOT_IP, on_event, NULL), HAL_ERR_INTERNAL);

    wifi_config_t cfg = {0};
    strncpy((char *)cfg.sta.ssid, ssid, sizeof cfg.sta.ssid);
    strncpy((char *)cfg.sta.password, password, sizeof cfg.sta.password);
    cfg.sta.threshold.authmode = password[0] ? WIFI_AUTH_WPA2_PSK : WIFI_AUTH_OPEN;
    cfg.sta.pmf_cfg.capable = true;
    HAL_CHECK(TAG, esp_wifi_set_mode(WIFI_MODE_STA), HAL_ERR_INTERNAL);
    HAL_CHECK(TAG, esp_wifi_set_storage(WIFI_STORAGE_RAM), HAL_ERR_INTERNAL);
    HAL_CHECK(TAG, esp_wifi_set_config(WIFI_IF_STA, &cfg), HAL_ERR_INVALID_ARG);
    memset(&cfg, 0, sizeof cfg);
    HAL_CHECK(TAG, esp_wifi_start(), HAL_ERR_INTERNAL);
    ESP_LOGI(TAG, "station started");
    return HAL_OK;
}

hal_err_t hal_wifi_status(hal_wifi_status_t *out)
{
    if (out == NULL) {
        return HAL_ERR_INVALID_ARG;
    }
    taskENTER_CRITICAL(&s_lock);
    out->state = s_state;
    out->ipv4 = s_ipv4;
    out->last_error = s_last_error;
    taskEXIT_CRITICAL(&s_lock);
    out->rssi = 0;
    if (out->state == HAL_WIFI_CONNECTED) {
        wifi_ap_record_t ap;
        if (esp_wifi_sta_get_ap_info(&ap) == ESP_OK) {
            out->rssi = ap.rssi;
        }
    }
    return HAL_OK;
}
