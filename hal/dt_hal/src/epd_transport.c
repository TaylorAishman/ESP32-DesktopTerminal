#include "epd_transport.h"

#include <stdbool.h>

#include "driver/gpio.h"
#include "driver/spi_master.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "hal_internal.h"
#include "hal_pins.h"

static const char *TAG = "hal_epd";

/* The panel accepts up to 20 MHz writes; 4 MHz leaves margin for long wires
 * and costs ~0.1 s per 48 KB frame, negligible next to a multi-second refresh. */
#define EPD_SPI_HZ (4 * 1000 * 1000)
#define EPD_SPI_HOST SPI2_HOST
/* Largest single SPI transaction; bigger buffers are sent in chunks. */
#define EPD_MAX_CHUNK 4096
/* On the UC8179 controller the BUSY line is LOW while busy. */
#define EPD_BUSY_ACTIVE_LEVEL 0
#define EPD_BUSY_POLL_MS 10

static spi_device_handle_t s_spi;
static bool s_ready;

hal_err_t epd_transport_init(void)
{
    if (s_ready) {
        return HAL_OK;
    }

    const gpio_config_t outputs = {
        .pin_bit_mask = (1ULL << HAL_PIN_EPD_DC) | (1ULL << HAL_PIN_EPD_RST),
        .mode = GPIO_MODE_OUTPUT,
    };
    HAL_CHECK(TAG, gpio_config(&outputs), HAL_ERR_IO);
    gpio_set_level(HAL_PIN_EPD_RST, 1);

    const gpio_config_t busy = {
        .pin_bit_mask = 1ULL << HAL_PIN_EPD_BUSY,
        .mode = GPIO_MODE_INPUT,
        /* With the panel unplugged the line floats; pull it to "busy" so a
         * missing panel shows up as a timeout instead of false "idle". */
        .pull_down_en = GPIO_PULLDOWN_ENABLE,
    };
    HAL_CHECK(TAG, gpio_config(&busy), HAL_ERR_IO);

    const spi_bus_config_t bus = {
        .mosi_io_num = HAL_PIN_EPD_MOSI,
        .miso_io_num = -1,
        .sclk_io_num = HAL_PIN_EPD_SCLK,
        .quadwp_io_num = -1,
        .quadhd_io_num = -1,
        .max_transfer_sz = EPD_MAX_CHUNK,
    };
    esp_err_t err = spi_bus_initialize(EPD_SPI_HOST, &bus, SPI_DMA_CH_AUTO);
    if (err != ESP_OK && err != ESP_ERR_INVALID_STATE) { /* already initialised is fine */
        ESP_LOGE(TAG, "spi_bus_initialize: %s", esp_err_to_name(err));
        return hal_from_esp(err);
    }

    const spi_device_interface_config_t dev = {
        .clock_speed_hz = EPD_SPI_HZ,
        .mode = 0,
        .spics_io_num = HAL_PIN_EPD_CS,
        .queue_size = 1,
    };
    HAL_CHECK(TAG, spi_bus_add_device(EPD_SPI_HOST, &dev, &s_spi), HAL_ERR_IO);

    s_ready = true;
    return HAL_OK;
}

hal_err_t epd_reset(void)
{
    if (!s_ready) {
        return HAL_ERR_NOT_INIT;
    }
    /* Timing from the panel vendor's reference driver. */
    gpio_set_level(HAL_PIN_EPD_RST, 1);
    vTaskDelay(pdMS_TO_TICKS(20));
    gpio_set_level(HAL_PIN_EPD_RST, 0);
    vTaskDelay(pdMS_TO_TICKS(2));
    gpio_set_level(HAL_PIN_EPD_RST, 1);
    vTaskDelay(pdMS_TO_TICKS(20));
    return HAL_OK;
}

static hal_err_t transmit(const uint8_t *data, size_t len, int dc_level)
{
    if (!s_ready) {
        return HAL_ERR_NOT_INIT;
    }
    if (len == 0) {
        return HAL_OK;
    }
    if (data == NULL) {
        return HAL_ERR_INVALID_ARG;
    }
    gpio_set_level(HAL_PIN_EPD_DC, dc_level);
    while (len > 0) {
        size_t n = len > EPD_MAX_CHUNK ? EPD_MAX_CHUNK : len;
        spi_transaction_t t = {
            .length = n * 8,
            .tx_buffer = data,
        };
        HAL_CHECK(TAG, spi_device_polling_transmit(s_spi, &t), HAL_ERR_IO);
        data += n;
        len -= n;
    }
    return HAL_OK;
}

hal_err_t epd_command(uint8_t cmd)
{
    return transmit(&cmd, 1, 0);
}

hal_err_t epd_data(const uint8_t *data, size_t len)
{
    return transmit(data, len, 1);
}

hal_err_t epd_command_data(uint8_t cmd, const uint8_t *data, size_t len)
{
    hal_err_t err = epd_command(cmd);
    if (err != HAL_OK) {
        return err;
    }
    return epd_data(data, len);
}

int epd_busy_level(void)
{
    return gpio_get_level(HAL_PIN_EPD_BUSY);
}

hal_err_t epd_wait_idle(uint32_t timeout_ms)
{
    if (!s_ready) {
        return HAL_ERR_NOT_INIT;
    }
    TickType_t start = xTaskGetTickCount();
    TickType_t limit = pdMS_TO_TICKS(timeout_ms);
    while (gpio_get_level(HAL_PIN_EPD_BUSY) == EPD_BUSY_ACTIVE_LEVEL) {
        if (xTaskGetTickCount() - start >= limit) {
            ESP_LOGW(TAG, "panel still busy after %u ms", (unsigned)timeout_ms);
            return HAL_ERR_TIMEOUT;
        }
        vTaskDelay(pdMS_TO_TICKS(EPD_BUSY_POLL_MS));
    }
    return HAL_OK;
}
