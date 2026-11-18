#include "hal_http.h"

#include <stdlib.h>
#include <string.h>
#include <strings.h>

#include "esp_crt_bundle.h"
#include "esp_http_client.h"
#include "esp_tls.h"

#include "hal_internal.h"

static const char *TAG = "hal_http";

/* Receive buffer for headers; large enough for typical API responses. */
#define HEADER_BUFFER 2048
/* Transmit buffer: request line and headers, including the URL with key. */
#define TX_BUFFER 1024

/* Why a connection failed, from the TLS layer's last error. */
static hal_err_t classify_open_failure(esp_http_client_handle_t client, esp_err_t open_err)
{
    int tls_code = 0;
    int cert_flags = 0;
    esp_err_t tls = esp_http_client_get_and_clear_last_tls_error(client, &tls_code, &cert_flags);
    hal_err_t mapped;
    switch (tls) {
    case ESP_ERR_ESP_TLS_CANNOT_RESOLVE_HOSTNAME:
        mapped = HAL_ERR_DNS;
        break;
    case ESP_ERR_ESP_TLS_CANNOT_CREATE_SOCKET:
    case ESP_ERR_ESP_TLS_FAILED_CONNECT_TO_HOST:
        mapped = HAL_ERR_CONNECT;
        break;
    case ESP_ERR_ESP_TLS_CONNECTION_TIMEOUT:
        mapped = HAL_ERR_TIMEOUT;
        break;
    case ESP_OK:
        mapped = open_err == ESP_ERR_HTTP_EAGAIN ? HAL_ERR_TIMEOUT : HAL_ERR_CONNECT;
        break;
    default:
        /* Handshake, certificate or other TLS failure. */
        mapped = HAL_ERR_TLS;
        break;
    }
    ESP_LOGW(TAG, "connect failed: %s (tls %s, code %d, cert flags 0x%x) -> %s", esp_err_to_name(open_err),
             esp_err_to_name(tls), tls_code, (unsigned)cert_flags, hal_err_str(mapped));
    return mapped;
}

static esp_err_t on_event(esp_http_client_event_t *evt)
{
    if (evt->event_id == HTTP_EVENT_ON_HEADER && evt->user_data != NULL &&
        strcasecmp(evt->header_key, "Retry-After") == 0) {
        hal_http_response_t *resp = evt->user_data;
        char *end = NULL;
        long secs = strtol(evt->header_value, &end, 10);
        if (end != evt->header_value && secs >= 0) {
            resp->retry_after_s = (int32_t)(secs > INT32_MAX ? INT32_MAX : secs);
        }
    }
    return ESP_OK;
}

hal_err_t hal_http_get(const hal_http_request_t *req, uint8_t *body, size_t body_cap, hal_http_response_t *resp)
{
    if (req == NULL || req->url == NULL || resp == NULL || (body == NULL && body_cap > 0) ||
        (req->headers == NULL && req->header_count > 0) || strncmp(req->url, "https://", 8) != 0) {
        return HAL_ERR_INVALID_ARG;
    }
    memset(resp, 0, sizeof *resp);
    resp->retry_after_s = -1;

    esp_http_client_config_t cfg = {
        .url = req->url,
        .method = HTTP_METHOD_GET,
        .timeout_ms = (int)req->timeout_ms,
        .crt_bundle_attach = esp_crt_bundle_attach,
        .buffer_size = HEADER_BUFFER,
        .buffer_size_tx = TX_BUFFER,
        .keep_alive_enable = false,
        .event_handler = on_event,
        .user_data = resp,
    };
    esp_http_client_handle_t client = esp_http_client_init(&cfg);
    if (client == NULL) {
        return HAL_ERR_NO_MEM;
    }
    for (size_t i = 0; i < req->header_count; i++) {
        esp_http_client_set_header(client, req->headers[i].name, req->headers[i].value);
    }

    hal_err_t result = HAL_OK;
    esp_err_t err = esp_http_client_open(client, 0);
    if (err != ESP_OK) {
        result = classify_open_failure(client, err);
        goto done;
    }
    int64_t headers = esp_http_client_fetch_headers(client);
    if (headers < 0) {
        result = headers == -ESP_ERR_HTTP_EAGAIN ? HAL_ERR_TIMEOUT : HAL_ERR_IO;
        ESP_LOGW(TAG, "reading response headers failed -> %s", hal_err_str(result));
        goto done;
    }
    resp->status = esp_http_client_get_status_code(client);

    size_t len = 0;
    while (len < body_cap) {
        int n = esp_http_client_read(client, (char *)body + len, (int)(body_cap - len));
        if (n < 0) {
            result = n == -ESP_ERR_HTTP_EAGAIN ? HAL_ERR_TIMEOUT : HAL_ERR_IO;
            ESP_LOGW(TAG, "read failed after %u bytes -> %s", (unsigned)len, hal_err_str(result));
            goto done;
        }
        if (n == 0) {
            break;
        }
        len += (size_t)n;
    }
    resp->body_len = len;
    if (len == body_cap) {
        /* Buffer full: is there more? */
        char probe;
        resp->truncated = esp_http_client_read(client, &probe, 1) > 0;
    }

done:
    esp_http_client_close(client);
    esp_http_client_cleanup(client);
    return result;
}
