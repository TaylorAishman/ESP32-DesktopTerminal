#include "FinData.hpp"

static const char* SYMBOLS = "VOO,VXUS,SOXX,IRBO,ICLN,IAU,SLV,DIA";

int MarketData_Fetch(MarketData* data, int max_entries, HTTPClient* http) {
    String url = "https://api.twelvedata.com/quote?symbol=";
    url += SYMBOLS;
    url += "&apikey=";
    url += TWELVE_DATA_API_KEY;

    http->begin(url.c_str());
    int httpResponseCode = http->GET();

    if (httpResponseCode > 0) {
        String payload = http->getString();
        JsonDocument doc;
        DeserializationError error = deserializeJson(doc, payload);

        if (error) {
            Serial.print("JSON parse error: ");
            Serial.println(error.c_str());
            http->end();
            return 0;
        }

        int count = 0;
        for (JsonPair kv : doc.as<JsonObject>()) {
            if (count >= max_entries) break;

            JsonObject quote      = kv.value().as<JsonObject>();
            data[count].symbol     = kv.key().c_str();
            data[count].price      = quote["close"].as<float>();
            data[count].change     = quote["change"].as<float>();
            data[count].change_pct = quote["percent_change"].as<float>();
            data[count].timestamp  = quote["datetime"].as<String>();
            data[count].market_open = quote["is_market_open"].as<bool>(); // <-- new
            count++;
        }

        http->end();
        return count;
    } else {
        Serial.print("Market Data Error: ");
        Serial.println(httpResponseCode);
        http->end();
        return 0;
    }
}