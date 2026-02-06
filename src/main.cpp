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
WeatherData weatherDataCurrent;
WeatherData weatherDataForecast[MAX_FORECAST_ENTRIES];  

void setup() {
  Serial.begin(115200);
  Serial.println("\n=== SETUP START ===");
  
  Serial.print("Connecting to WiFi: ");
  Serial.println(ssid);
  WiFi.begin(ssid, password);
  
  int timeout = 0;
  while (WiFi.status() != WL_CONNECTED && timeout < 40) {  // 20 second timeout
    delay(500);
    Serial.print(".");
    timeout++;
  }
  Serial.println();
  
  if (WiFi.status() == WL_CONNECTED) {
    Serial.println("WiFi connected successfully!");
    Serial.print("IP address: ");
    Serial.println(WiFi.localIP());
  } else {
    Serial.println("WiFi connection FAILED!");
  }

  Serial.println("Initializing SPI...");
  SPI.begin(EPD_SCK, EPD_MISO, EPD_MOSI, EPD_CS);
  
  Serial.println("Initializing Renderer...");
  Renderer_Init();
  
  Serial.println("=== SETUP COMPLETE ===\n");
}

void loop() {
  Serial.println("\n=== LOOP START ===");
  Serial.print("WiFi status: ");
  Serial.println(WiFi.status() == WL_CONNECTED ? "CONNECTED" : "DISCONNECTED");
  
  if(WiFi.status() == WL_CONNECTED) {
    Serial.println("Fetching forecast data...");
    
    // Note: You're passing weatherDataCurrent but should pass weatherDataForecast
    int count = WeatherData_ForecastParse(weatherDataForecast, MAX_FORECAST_ENTRIES, &http);
    
    Serial.print("Forecast entries received: ");
    Serial.println(count);
    
    if (count > 0) {
      Serial.println("Rendering weather forecast...");
      Renderer_RenderWeatherForecast(weatherDataForecast, count);
      Serial.println("Render complete!");
    } else {
      Serial.println("ERROR: Parse failed, showing error screen");
      Renderer_RenderErrorScreen("Failed API call to Open Weather");
    }
  } else {
    Serial.println("ERROR: WiFi not connected, showing error screen");
    Renderer_RenderErrorScreen("WiFi not connected");
  }
  
  Serial.println("Sleeping for 15 minutes...");
  Serial.println("=== LOOP END ===\n");
  delay(900000);
}