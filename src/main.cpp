#include <Arduino.h>
#include <SPI.h>
#include <WiFi.h>

#include "CustomBold12pt7b.h"
#include "credentials.h"
#include "WeatherData.hpp"
#include "Renderer.hpp"
#include "config.h"

// WiFi Credentials
const char* ssid = SSID;
const char* password = PASSWORD;

HTTPClient http;
WeatherData weatherData;

void setup() {
  Serial.begin(115200);
  WiFi.begin(ssid, password);
  while (WiFi.status() != WL_CONNECTED) {
    delay(500);
    Serial.print(".");
  }

  Serial.println("Connected to WiFi!");
  Serial.println("IP address: ");
  Serial.println(WiFi.localIP());

  SPI.begin(EPD_SCK, EPD_MISO, EPD_MOSI, EPD_CS);
  Renderer_Init();
}

void loop() {

  if(WiFi.status() == WL_CONNECTED) {
    if (WeatherData_Parse(&weatherData, &http)){
      Renderer_RenderWeatherData(&weatherData);
    } else {
      Renderer_RenderErrorScreen("Failed API call to Open Weather");
    }

  }
  delay(900000);
}