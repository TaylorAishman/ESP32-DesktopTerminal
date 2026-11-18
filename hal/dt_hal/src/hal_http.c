#include "hal_http.h"

#include <stdlib.h>
#include <string.h>
#include <strings.h>

#include "esp_crt_bundle.h"
#include "esp_http_client.h"

#include "hal_internal.h"

static const char *TAG = "hal_http";

/* Receive buffer for headers; large enough for typical API responses. */
#define HEADER_BUFFER 2048
/* Transmit buffer: request line and headers, including the URL with key. */
#define TX_BUFFER 1024

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
        ESP_LOGW(TAG, "connect failed: %s", esp_err_to_name(err));
        result = HAL_ERR_CONNECT;
        goto done;
    }
    if (esp_http_client_fetch_headers(client) < 0) {
        ESP_LOGW(TAG, "no response headers");
        result = HAL_ERR_IO;
        goto done;
    }
    resp->status = esp_http_client_get_status_code(client);

    size_t len = 0;
    while (len < body_cap) {
        int n = esp_http_client_read(client, (char *)body + len, (int)(body_cap - len));
        if (n < 0) {
            ESP_LOGW(TAG, "read failed after %u bytes", (unsigned)len);
            result = HAL_ERR_IO;
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
