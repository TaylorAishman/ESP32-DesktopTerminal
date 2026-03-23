#include "NewsData.hpp"

int NewsData_Parse(NewsData* data, int max_entries, HTTPClient* http) {
    String serverName =  "https://newsapi.org/v2/top-headlines?country=us&apiKey=" NEWS_API_KEY;
    http->begin(serverName.c_str());

    int httpResponseCode = http->GET();
    if (httpResponseCode > 0) {
        Serial.print("HTTP Response code: ");
        Serial.println(httpResponseCode);
        
        String payload = http->getString();
        JsonDocument doc;
        DeserializationError error = deserializeJson(doc, payload);
        
        if (error) {
            Serial.print("JSON parse error: ");
            Serial.println(error.c_str());
            http->end();
            return 0;
        }

        JsonArray articles = doc["articles"].as<JsonArray>();
        int count = 0;
        
        for (JsonObject article : articles) {
            if (count >= max_entries) break;
            
            data[count].publisher = article["source"]["name"].as<String>();
            data[count].title = article["title"].as<String>();
            data[count].description = article["description"].as<String>();
            data[count].timestamp = article["publishedAt"].as<time_t>();
            count++;
        }
        
        http->end();
        return count;  // Return number of entries parsed
    } else {
        Serial.print("News Data Error: ");
        Serial.println(httpResponseCode);
        http->end();
        return 0;
    }
}