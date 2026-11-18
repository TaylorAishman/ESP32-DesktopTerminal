/*
 * GPIO assignments for the Waveshare e-Paper ESP32 Driver Board, Rev 3.
 *
 * This is the only place pin numbers are defined (REQ-MAINT-01). Sources:
 * ReferenceDocs/Pins.md and the board schematic
 * (ReferenceDocs/E-Paper_ESP32_Driver_Board_V3.pdf); see Requirements §3.1.
 */
#ifndef DT_HAL_PINS_H
#define DT_HAL_PINS_H

/* E-ink panel, 4-wire SPI (BS is tied low on the board). */
#define HAL_PIN_EPD_MOSI 14 /* DIN  */
#define HAL_PIN_EPD_SCLK 13 /* SCLK */
#define HAL_PIN_EPD_CS 15   /* chip select, active low */
#define HAL_PIN_EPD_DC 27   /* low = command, high = data */
#define HAL_PIN_EPD_RST 26  /* reset, active low */
#define HAL_PIN_EPD_BUSY 25 /* panel busy output */

/*
 * Front-panel buttons 1 to 4: header J4, silk P16 to P19. Each button connects
 * its pin to GND; internal pull-ups are enabled, so low means pressed.
 */
#define HAL_PIN_BUTTON_1 16 /* Home    */
#define HAL_PIN_BUTTON_2 17 /* Weather */
#define HAL_PIN_BUTTON_3 18 /* News    */
#define HAL_PIN_BUTTON_4 19 /* Finance */
#define HAL_BUTTON_COUNT 4

/*
 * Reserved: never configure or drive these (REQ-HW-06).
 *   GPIO0  - BOOT key and auto-program circuit
 *   GPIO2  - on-board LED; boot strapping pin
 *   GPIO4  - panel power switch transistor (R35 not fitted on Rev 3)
 *   GPIO12 - boot strapping pin (flash voltage)
 * GPIO34, 35, 36 and 39 are input-only with no internal pull-ups.
 */

#endif /* DT_HAL_PINS_H */
