#include "WeatherData.hpp"


WeatherCondition WeatherData_GetCondition (int weather_id){
  WeatherCondition condition;
  condition.weather_id = weather_id;
  condition.icon_id = 0;

  Serial.println(weather_id);


  switch(weather_id) {
    case 200 ... 232: // Thunderstorm
      condition.icon_id = 1;
      condition.weather_description = "there is a Thunderstorm lol";
      break;
    case 300 ... 321: // Drizzle
      condition.icon_id = 2;
      condition.weather_description = "there is a Drizzle lol";
      break;
    case 500 ... 531: // Rain
      condition.icon_id = 3;
      condition.weather_description = "there is a Rain lol";
      break;
    case 600 ... 622: // Snow
      condition.icon_id = 4;
      condition.weather_description = "there is a Snow lol";
      break;
    case 701 ... 781: // Atmosphere
      condition.icon_id = 5;
      condition.weather_description = "there is a Atmosphere lol";
      
      break;
    case 801 ... 804: // Clouds
      condition.icon_id = 7;
      condition.weather_description = "there is a Cloud lol";
      break;
    default:
      condition.icon_id = 8; // Unknown weather ID
      condition.weather_description = "there is a UNKNOWN lol";
      break;
   }

   Serial.println(condition.icon_id);
   Serial.println(condition.weather_description);
   return condition;
}


int WeatherData_Parse(WeatherData* data, HTTPClient* http){
  String serverName = "https://api.openweathermap.org/data/2.5/weather?lat=" LATITUDE "&lon=" LONGITUDE "&appid=" API_KEY "&units=imperial";
  http->begin(serverName.c_str());

  int httpResponseCode = http->GET();
    if (httpResponseCode>0) {
      String payload = http->getString();
      JsonDocument doc;
      deserializeJson(doc, payload);
      data->temp = doc["main"]["temp"].as<float>();
      data->feels_like = doc["main"]["feels_like"].as<float>();
      data->daily_low = doc["main"]["temp_min"].as<float>();
      data->daily_high = doc["main"]["temp_max"].as<float>();
      Serial.println(payload); 
      Serial.println(doc["weather"]["id"].as<int>());
      data->condition = WeatherData_GetCondition(doc["weather"][0]["id"].as<int>());

    }
    else {
      return 0;
    }
    http->end();
    return 1;
}