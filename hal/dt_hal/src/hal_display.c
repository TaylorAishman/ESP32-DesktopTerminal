#include "hal_display.h"

#include <stdbool.h>

#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"
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
#define CMD_DATA_OLD 0x10
#define CMD_DISPLAY_REFRESH 0x12
#define CMD_DATA_NEW 0x13
#define CMD_DUAL_SPI 0x15
#define CMD_VCOM_DATA_INTERVAL 0x50
#define CMD_TCON 0x60
#define CMD_RESOLUTION 0x61

#define DEEP_SLEEP_CHECK 0xA5

/* Power-on and power-off finish in well under a second on a healthy panel. */
#define POWER_TIMEOUT_MS 5000
/* A full refresh takes about 4 s at room temperature, longer in the cold. */
#define REFRESH_TIMEOUT_MS 30000
/* Rows are transformed through this buffer when they must be inverted. */
#define CHUNK_BYTES 1000
_Static_assert(HAL_DISPLAY_FRAME_BYTES % CHUNK_BYTES == 0, "chunk must divide the frame");

/* Longest a caller waits for another thread's display operation. A refresh
 * takes ~5 s, so this covers a refresh plus a full re-init with margin. */
#define LOCK_TIMEOUT_MS 60000

/* Serialises all public calls (HAL rule 5). Created on first use. */
static SemaphoreHandle_t s_lock;
static StaticSemaphore_t s_lock_storage;
static portMUX_TYPE s_lock_init = portMUX_INITIALIZER_UNLOCKED;

static hal_err_t lock(void)
{
    taskENTER_CRITICAL(&s_lock_init);
    if (s_lock == NULL) {
        s_lock = xSemaphoreCreateMutexStatic(&s_lock_storage);
    }
    taskEXIT_CRITICAL(&s_lock_init);
    if (xSemaphoreTake(s_lock, pdMS_TO_TICKS(LOCK_TIMEOUT_MS)) != pdTRUE) {
        ESP_LOGE(TAG, "display busy in another thread for %d ms", LOCK_TIMEOUT_MS);
        return HAL_ERR_TIMEOUT;
    }
    return HAL_OK;
}

static void unlock(void)
{
    xSemaphoreGive(s_lock);
}

/* Run `call` with the display lock held. */
#define LOCKED(call)                                                           \
    do {                                                                       \
        hal_err_t lock_rc_ = lock();                                           \
        if (lock_rc_ != HAL_OK) {                                              \
            return lock_rc_;                                                   \
        }                                                                      \
        hal_err_t call_rc_ = (call);                                           \
        unlock();                                                              \
        return call_rc_;                                                       \
    } while (0)

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

static hal_err_t init_locked(void)
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

static hal_err_t sleep_locked(void);

/* Frame data in either polarity. The controller's "new" RAM takes our layout
 * as-is (1 = black); the "old" RAM takes the inverse. */
typedef enum { FILL_NONE, FILL_WHITE } fill_t;

static hal_err_t send_plane(uint8_t cmd, const uint8_t *frame, bool invert, fill_t fill)
{
    static uint8_t chunk[CHUNK_BYTES];
    hal_err_t err = epd_command(cmd);
    if (err != HAL_OK) {
        return err;
    }
    for (size_t off = 0; off < HAL_DISPLAY_FRAME_BYTES; off += CHUNK_BYTES) {
        for (size_t i = 0; i < CHUNK_BYTES; i++) {
            uint8_t b = fill == FILL_WHITE ? 0x00 : frame[off + i];
            chunk[i] = invert ? (uint8_t)~b : b;
        }
        err = epd_data(chunk, CHUNK_BYTES);
        if (err != HAL_OK) {
            return err;
        }
    }
    return HAL_OK;
}

static hal_err_t refresh(void)
{
    TRY(epd_command(CMD_DISPLAY_REFRESH));
    vTaskDelay(pdMS_TO_TICKS(100));
    TRY(epd_wait_idle(REFRESH_TIMEOUT_MS));
    return HAL_OK;
}

static hal_err_t show_planes(const uint8_t *frame, fill_t fill)
{
    if (!s_powered) {
        hal_err_t err = init_locked();
        if (err != HAL_OK) {
            return err;
        }
    }
    TRY(send_plane(CMD_DATA_OLD, frame, true, fill));
    TRY(send_plane(CMD_DATA_NEW, frame, false, fill));
    TRY(refresh());
    return sleep_locked();
}


static hal_err_t sleep_locked(void)
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

/* ---- Public API: every call holds the display lock. ---- */

hal_err_t hal_display_init(void)
{
    LOCKED(init_locked());
}

hal_err_t hal_display_sleep(void)
{
    LOCKED(sleep_locked());
}

hal_err_t hal_display_show(const uint8_t *frame, size_t len)
{
    if (frame == NULL || len != HAL_DISPLAY_FRAME_BYTES) {
        ESP_LOGE(TAG, "show: bad frame (len %u, expected %u)", (unsigned)len,
                 (unsigned)HAL_DISPLAY_FRAME_BYTES);
        return HAL_ERR_INVALID_ARG;
    }
    LOCKED(show_planes(frame, FILL_NONE));
}

hal_err_t hal_display_clear(void)
{
    LOCKED(show_planes(NULL, FILL_WHITE));
}
