#include "Renderer.hpp"

#include "../assets/weather_icons/weather_32.h"
#include "../assets/weather_icons/weather_64.h"
#include "../assets/weather_icons/weather_128.h"

// Links Adafruit library with display
GxEPD2_BW<GxEPD2_750_T7, GxEPD2_750_T7::HEIGHT> display(
  GxEPD2_750_T7(EPD_CS, EPD_DC, EPD_RST, EPD_BUSY)
);

void Renderer_Init(){
  display.init(115200);
  display.setRotation(0);
  display.firstPage();
}

void Renderer_ClearScreen() {
  display.firstPage();
  display.fillScreen(GxEPD_WHITE);
  display.setTextColor(GxEPD_BLACK);
  display.setFont(&CustomBold12pt7b);
  display.setCursor(10, 30);
}

void Renderer_RenderWeatherData(const WeatherData* data) {
  Renderer_ClearScreen();

  char str[128];
  snprintf(str, sizeof(str), "The temperature is: %.2f`F and the high today is %.2f`F with lows of %.2f`F", data->temp, data->daily_high, data->daily_low);

  display.print(str);
  display.drawBitmap(10, 40, weather_128_allArray[data->icon_id], 128, 128, GxEPD_BLACK);
  display.nextPage();
}

void Renderer_RenderErrorScreen(const char* message) {
  Renderer_ClearScreen();
  display.print(message);
  display.nextPage();
}