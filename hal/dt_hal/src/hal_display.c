#include "hal_display.h"

#include <stdbool.h>

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "epd_transport.h"
#include "hal_internal.h"

static const char *TAG = "hal_display";

/* UC8179 commands used here (Waveshare 7.5" V2 reference driver). */
#define CMD_PANEL_SETTING 0x00
#define CMD_POWER_SETTING 0x01
#define CMD_POWER_OFF 0x02
#define CMD_POWER_ON 0x04
#define CMD_BOOSTER_SOFT_START 0x06
#define CMD_DEEP_SLEEP 0x07
#define CMD_DUAL_SPI 0x15
#define CMD_VCOM_DATA_INTERVAL 0x50
#define CMD_TCON 0x60
#define CMD_RESOLUTION 0x61

#define DEEP_SLEEP_CHECK 0xA5

/* Power-on and power-off finish in well under a second on a healthy panel. */
#define POWER_TIMEOUT_MS 5000

/* Panel state as far as this module knows. Anything unexpected sends the next
 * call back through a full init. */
static bool s_powered;

#define TRY(expr)                                                              \
    do {                                                                       \
        hal_err_t try_rc_ = (expr);                                            \
        if (try_rc_ != HAL_OK) {                                               \
            ESP_LOGE(TAG, "%s: %s", #expr, hal_err_str(try_rc_));              \
            s_powered = false;                                                 \
            return try_rc_;                                                    \
        }                                                                      \
    } while (0)

static hal_err_t send(uint8_t cmd, const uint8_t *data, size_t len)
{
    return epd_command_data(cmd, data, len);
}

hal_err_t hal_display_init(void)
{
    s_powered = false;
    TRY(epd_transport_init());
    TRY(epd_reset());

    /* VGH = 20 V, VGL = -20 V, VDH = 15 V, VDL = -15 V. */
    static const uint8_t power[] = {0x07, 0x07, 0x3F, 0x3F};
    TRY(send(CMD_POWER_SETTING, power, sizeof power));

    static const uint8_t booster[] = {0x17, 0x17, 0x28, 0x17};
    TRY(send(CMD_BOOSTER_SOFT_START, booster, sizeof booster));

    TRY(epd_command(CMD_POWER_ON));
    vTaskDelay(pdMS_TO_TICKS(100));
    TRY(epd_wait_idle(POWER_TIMEOUT_MS));

    /* Black/white mode, waveform from OTP, scan up, shift right. */
    static const uint8_t panel[] = {0x1F};
    TRY(send(CMD_PANEL_SETTING, panel, sizeof panel));

    static const uint8_t resolution[] = {
        HAL_DISPLAY_WIDTH >> 8, HAL_DISPLAY_WIDTH & 0xFF,
        HAL_DISPLAY_HEIGHT >> 8, HAL_DISPLAY_HEIGHT & 0xFF,
    };
    TRY(send(CMD_RESOLUTION, resolution, sizeof resolution));

    static const uint8_t single_spi[] = {0x00};
    TRY(send(CMD_DUAL_SPI, single_spi, sizeof single_spi));

    static const uint8_t vcom[] = {0x10, 0x07};
    TRY(send(CMD_VCOM_DATA_INTERVAL, vcom, sizeof vcom));

    static const uint8_t tcon[] = {0x22};
    TRY(send(CMD_TCON, tcon, sizeof tcon));

    s_powered = true;
    ESP_LOGI(TAG, "panel initialised");
    return HAL_OK;
}

hal_err_t hal_display_sleep(void)
{
    if (!s_powered) {
        /* Already off, or in an unknown state after an error: a reset plus
         * deep sleep is still the safest end state, but needs the bus. */
        TRY(epd_transport_init());
        TRY(epd_reset());
    }
    s_powered = false;

    /* Border floating while off, as in the vendor driver. */
    static const uint8_t vcom_off[] = {0xF7};
    TRY(send(CMD_VCOM_DATA_INTERVAL, vcom_off, sizeof vcom_off));
    TRY(epd_command(CMD_POWER_OFF));
    TRY(epd_wait_idle(POWER_TIMEOUT_MS));

    static const uint8_t check[] = {DEEP_SLEEP_CHECK};
    TRY(send(CMD_DEEP_SLEEP, check, sizeof check));
    ESP_LOGI(TAG, "panel asleep");
    return HAL_OK;
}
