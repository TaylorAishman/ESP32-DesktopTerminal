#pragma once
#include <Arduino.h>
#include <ArduinoJson.h>
#include <WiFi.h>
#include <HTTPClient.h>
#include "credentials.h"

#define MAX_FORECAST_ENTRIES 10

typedef struct {
  int weather_id;
  int icon_id;
} WeatherCondition;

typedef struct {
  int weather_id;
  float temp;
  float feels_like;
  float daily_low;
  float daily_high;
  time_t timestamp;
  WeatherCondition condition;
} WeatherData;



WeatherCondition WeatherData_GetCondition(int weather_id);
int WeatherData_CurrentParse(WeatherData* data, HTTPClient* http);
int WeatherData_ForecastParse(WeatherData* data, int max_entries, HTTPClient* http);