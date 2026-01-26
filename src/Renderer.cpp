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

void Renderer_DrawCenteredText(const char *buf, int x, int y)
{
    int16_t x1, y1;
    uint16_t w, h;
    display.getTextBounds(buf, x, y, &x1, &y1, &w, &h); //calc width of new string
    display.setCursor(x - w / 2, y);
    display.print(buf);
}

void Renderer_RenderWeatherData(const WeatherData* data) {
  Renderer_ClearScreen();

  char str[128];

  display.drawBitmap(36, 15, weather_128_allArray[data->condition.icon_id], 128, 128, GxEPD_BLACK);
  snprintf(str, sizeof(str), "%d`F (%d`F)", (int) round(data->temp), (int) round(data->feels_like));
  Renderer_DrawCenteredText(str, 100, 164);
  snprintf(str, sizeof(str), "%d`F / %d`F", (int) round(data->daily_low), (int) round(data->daily_high));
  Renderer_DrawCenteredText(str, 100, 189);

  display.nextPage();
}

void Renderer_RenderErrorScreen(const char* message) {
  Renderer_ClearScreen();
  display.print(message);
  display.nextPage();
}
