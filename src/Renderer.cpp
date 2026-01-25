#include "Renderer.hpp"

// Links Adafruit library with display
GxEPD2_BW<GxEPD2_750_T7, GxEPD2_750_T7::HEIGHT> display(
  GxEPD2_750_T7(EPD_CS, EPD_DC, EPD_RST, EPD_BUSY)
);

void Renderer_Init(){
  display.init(115200);
  display.setRotation(0);
  display.firstPage();
}

void Renderer_RenderWeatherData(const WeatherData* data) {
  char str[128];
  snprintf(str, sizeof(str), "The temperature is: %.2f`F and the high today is %.2f`F with lows of %.2f`F", data->temp, data->daily_high, data->daily_low);
  do { Renderer_DrawScreen(str); } while (display.nextPage());
}

void Renderer_DrawScreen(const char* message) {
  Serial.println("Running draw screen!");
  display.fillScreen(GxEPD_WHITE);
  display.setTextColor(GxEPD_BLACK);
  display.setFont(&CustomBold12pt7b);
  display.setCursor(10, 30);
  display.print(message);
}

void Renderer_RenderErrorScreen(const char* message) {
  do { Renderer_DrawScreen(message); } while (display.nextPage());
}