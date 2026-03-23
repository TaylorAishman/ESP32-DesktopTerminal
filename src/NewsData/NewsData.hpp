#pragma once
#include <Arduino.h>
#include <ArduinoJson.h>
#include <WiFi.h>
#include <HTTPClient.h>
#include "credentials.h"

#define MAX_NEWS_ENTRIES 5

typedef struct {
    String publisher;
    String title;
    String description;
    time_t timestamp;
} NewsData;

int NewsData_Parse(NewsData* data, int max_entries, HTTPClient* http);