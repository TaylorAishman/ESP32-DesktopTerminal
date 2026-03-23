#pragma once
#include <Arduino.h>
#include <ArduinoJson.h>
#include <WiFi.h>
#include <HTTPClient.h>
#include "credentials.h"

#define MAX_TICKERS 8

typedef struct {
    String symbol;
    float  price;
    float  change;
    float  change_pct;
    String timestamp;
    bool   market_open;   // <-- new
} MarketData;

int MarketData_Fetch(MarketData* data, int max_entries, HTTPClient* http);