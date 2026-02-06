#include "Renderer.hpp"

#include "../assets/weather_icons/weather_32.h"
#include "../assets/weather_icons/weather_64.h"
#include "../assets/weather_icons/weather_128.h"

// Links Adafruit library with display
GxEPD2_BW<GxEPD2_750_T7, GxEPD2_750_T7::HEIGHT> display(
  GxEPD2_750_T7(EPD_CS, EPD_DC, EPD_RST, EPD_BUSY)
);

const char* DAYS_OF_WEEK[] = { "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday" };

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

void Renderer_DrawCenteredText(const char *buf, int x, int y) {
    int16_t x1, y1;
    uint16_t w, h;
    display.getTextBounds(buf, x, y, &x1, &y1, &w, &h); //calc width of new string
    display.setCursor(x - w / 2, y);
    display.print(buf);
}

void Renderer_CenterSubdivide(int screen_length, int subdivisions, int spacing, int* outvalues) {
  int start_offset = (screen_length - (subdivisions - 1) * spacing) / 2;
  for (int i = 0; i < subdivisions; i++) {
    outvalues[i] = start_offset + i * spacing;
  }
}

void Renderer_PlaceForecastIcon(const WeatherData* data, int x, int y) {
  char str[32];
  display.drawBitmap(x - 64, y - 86, weather_128_allArray[data->condition.icon_id], 128, 128, GxEPD_BLACK);

  snprintf(str, sizeof(str), "%d`F", (int) round(data->temp));
  Renderer_DrawCenteredText(str, x, y + 65);

  struct tm * timeinfo = localtime(&data->timestamp);
  strftime(str, sizeof(str), "%I %p", timeinfo);
  Renderer_DrawCenteredText(str, x, y + 85);

}

void Renderer_RenderWeatherForecast(const WeatherData* data, int count) {
  Renderer_ClearScreen();

  static int num_x = 5;
  static int num_y = 2;
  int x_positions[num_x];
  int y_positions[num_y];

  Renderer_CenterSubdivide(display.width(), num_x, 153, x_positions);
  Renderer_CenterSubdivide(display.height(), num_y, 193, y_positions);\

  for (int y = 0; y < num_y; y++) {
    for (int x = 0; x < num_x; x++) {
      int index = y * num_x + x;
      if (index < count) {
        Renderer_PlaceForecastIcon(&data[index], x_positions[x], y_positions[y]);
      }
    }
  }

  display.nextPage();
}

void Renderer_RenderWeatherData(const WeatherData* data) {
  Renderer_ClearScreen();

  char str[32];

  display.drawBitmap(36, 15, weather_128_allArray[data->condition.icon_id], 128, 128, GxEPD_BLACK);
  snprintf(str, sizeof(str), "%d`F (%d`F)", (int) round(data->temp), (int) round(data->feels_like));
  Renderer_DrawCenteredText(str, 100, 165);
  snprintf(str, sizeof(str), "%d`F / %d`F", (int) round(data->daily_low), (int) round(data->daily_high));
  Renderer_DrawCenteredText(str, 100, 190);

  display.nextPage();
}

void Renderer_RenderErrorScreen(const char* message) {
  Renderer_ClearScreen();
  display.print(message);
  display.nextPage();
}
