#include "hal_buttons.h"

#include <stdbool.h>

#include "driver/gpio.h"
#include "freertos/FreeRTOS.h"
#include "freertos/queue.h"
#include "freertos/task.h"

#include "hal_internal.h"
#include "hal_pins.h"

static const char *TAG = "hal_buttons";

#define SAMPLE_MS 10
/* A level must hold for this many samples to count (30 ms). */
#define DEBOUNCE_SAMPLES 3
#define LONG_PRESS_MS 1000
#define QUEUE_DEPTH 8

static const gpio_num_t PINS[HAL_BUTTON_COUNT] = {
    HAL_PIN_BUTTON_1, HAL_PIN_BUTTON_2, HAL_PIN_BUTTON_3, HAL_PIN_BUTTON_4,
};

typedef struct {
    bool pressed;      /* debounced state */
    uint8_t agree;     /* consecutive samples differing from `pressed` */
    TickType_t since;  /* when the current press started */
    bool long_sent;    /* LONG_PRESS already emitted for this press */
} button_t;

static QueueHandle_t s_queue;
static volatile uint8_t s_held;
static portMUX_TYPE s_lock = portMUX_INITIALIZER_UNLOCKED;

static void emit(uint8_t button, hal_button_action_t action)
{
    hal_button_event_t e = {.button = button, .action = action};
    if (xQueueSend(s_queue, &e, 0) != pdTRUE) {
        ESP_LOGW(TAG, "event queue full; dropping button %u", button);
    }
}

static void sample_task(void *arg)
{
    (void)arg;
    button_t b[HAL_BUTTON_COUNT] = {0};
    TickType_t wake = xTaskGetTickCount();
    for (;;) {
        vTaskDelayUntil(&wake, pdMS_TO_TICKS(SAMPLE_MS));
        TickType_t now = xTaskGetTickCount();
        uint8_t held = 0;
        for (int i = 0; i < HAL_BUTTON_COUNT; i++) {
            bool raw = gpio_get_level(PINS[i]) == 0; /* active low */
            if (raw != b[i].pressed) {
                if (++b[i].agree >= DEBOUNCE_SAMPLES) {
                    b[i].pressed = raw;
                    b[i].agree = 0;
                    if (raw) {
                        b[i].since = now;
                        b[i].long_sent = false;
                    } else if (!b[i].long_sent) {
                        emit((uint8_t)(i + 1), HAL_BTN_PRESS);
                    }
                }
            } else {
                b[i].agree = 0;
            }
            if (b[i].pressed) {
                held |= (uint8_t)(1u << i);
                if (!b[i].long_sent && now - b[i].since >= pdMS_TO_TICKS(LONG_PRESS_MS)) {
                    b[i].long_sent = true;
                    emit((uint8_t)(i + 1), HAL_BTN_LONG_PRESS);
                }
            }
        }
        s_held = held;
    }
}

hal_err_t hal_buttons_init(void)
{
    taskENTER_CRITICAL(&s_lock);
    bool started = s_queue != NULL;
    taskEXIT_CRITICAL(&s_lock);
    if (started) {
        return HAL_OK;
    }
    gpio_config_t cfg = {
        .mode = GPIO_MODE_INPUT,
        .pull_up_en = GPIO_PULLUP_ENABLE,
    };
    for (int i = 0; i < HAL_BUTTON_COUNT; i++) {
        cfg.pin_bit_mask |= 1ULL << PINS[i];
    }
    HAL_CHECK(TAG, gpio_config(&cfg), HAL_ERR_IO);
    QueueHandle_t q = xQueueCreate(QUEUE_DEPTH, sizeof(hal_button_event_t));
    if (q == NULL) {
        return HAL_ERR_NO_MEM;
    }
    taskENTER_CRITICAL(&s_lock);
    s_queue = q;
    taskEXIT_CRITICAL(&s_lock);
    if (xTaskCreate(sample_task, "hal_buttons", 2048, NULL, 5, NULL) != pdPASS) {
        return HAL_ERR_NO_MEM;
    }
    ESP_LOGI(TAG, "buttons on GPIO %d, %d, %d, %d", PINS[0], PINS[1], PINS[2], PINS[3]);
    return HAL_OK;
}

hal_err_t hal_buttons_wait(hal_button_event_t *out, uint32_t timeout_ms)
{
    if (out == NULL) {
        return HAL_ERR_INVALID_ARG;
    }
    if (s_queue == NULL) {
        return HAL_ERR_NOT_INIT;
    }
    return xQueueReceive(s_queue, out, pdMS_TO_TICKS(timeout_ms)) == pdTRUE ? HAL_OK : HAL_ERR_TIMEOUT;
}

hal_err_t hal_buttons_held(uint8_t *mask)
{
    if (mask == NULL) {
        return HAL_ERR_INVALID_ARG;
    }
    if (s_queue == NULL) {
        return HAL_ERR_NOT_INIT;
    }
    *mask = s_held;
    return HAL_OK;
}
