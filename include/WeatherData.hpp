#pragma once
#include <Arduino.h>
#include <ArduinoJson.h>
#include <WiFi.h>
#include <HTTPClient.h>
#include "credentials.h"

typedef struct {
  int weather_id;
  const char* weather_description;
  int icon_id;
} WeatherCondition;

typedef struct {
  int weather_id;
  float temp;
  float feels_like;
  float daily_low;
  float daily_high;
  WeatherCondition condition;
} WeatherData;

WeatherCondition WeatherData_GetCondition (int weather_id);
int WeatherData_Parse(WeatherData* data, HTTPClient* http);