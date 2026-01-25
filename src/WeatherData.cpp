#include "WeatherData.hpp"

int WeatherData_Parse(WeatherData* data, HTTPClient* http){
  String serverName = "https://api.openweathermap.org/data/2.5/weather?lat=" LATITUDE "&lon=" LONGITUDE "&appid=" API_KEY "&units=imperial";
  http->begin(serverName.c_str());

  int httpResponseCode = http->GET();
    if (httpResponseCode>0) {
      char str[50];
      Serial.print("HTTP Response code: ");
      Serial.println(httpResponseCode);
      String payload = http->getString();
      Serial.println(payload);

      JsonDocument doc;
      deserializeJson(doc, payload);

      data->weather_id = doc["weather"]["id"].as<int>();
      data->temp = doc["main"]["temp"].as<float>();
      data->feels_like = doc["main"]["feels_like"].as<float>();
      data->daily_low = doc["main"]["temp_min"].as<float>();
      data->daily_high = doc["main"]["temp_max"].as<float>();
    }
    else {
      Serial.print("Error code: ");
      Serial.println(httpResponseCode);
      return 0;
    }
    http->end();
    return 1;
}