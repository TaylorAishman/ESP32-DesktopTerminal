#pragma once
#include <Arduino.h>
#include <ArduinoJson.h>
#include <SPI.h>
#include <GxEPD2_BW.h>
#include <epd/GxEPD2_750_T7.h>
#include "CustomBold12pt7b.h"
#include "WeatherData/WeatherData.hpp"
#include "config.h"

void Renderer_Init();
void Renderer_ClearScreen();
void Renderer_DrawCenteredText(const char *buf, int x, int y);
void Renderer_RenderWeatherData(const WeatherData* data);
void Renderer_RenderWeatherForecast(const WeatherData* data, int count);
void Renderer_RenderErrorScreen(const char* message);