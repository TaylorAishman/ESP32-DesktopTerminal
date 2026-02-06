#include "WeatherData.hpp"


WeatherCondition WeatherData_GetCondition (int weather_id){
  WeatherCondition condition;
  condition.weather_id = weather_id;
  condition.icon_id = 0;

  switch(weather_id) {
    // Group 2xx: Thunderstorm
    case 200 ... 232:
      condition.icon_id = 38; // weather_128_tstorms
      break;
      
    // Group 3xx: Drizzle
    case 300 ... 321:
      condition.icon_id = 1; // weather_128_chancerain
      break;
      
    // Group 5xx: Rain
    case 500 ... 504: // Light to extreme rain
      condition.icon_id = 34; // weather_128_rain
      break;
    case 511: // Freezing rain
      condition.icon_id = 35; // weather_128_sleet
      break;
    case 520 ... 531: // Shower rain
      condition.icon_id = 34; // weather_128_rain
      break;
      
    // Group 6xx: Snow
    case 600 ... 602: // Light to heavy snow
      condition.icon_id = 36; // weather_128_snow
      break;
    case 611 ... 613: // Sleet
      condition.icon_id = 35; // weather_128_sleet
      break;
    case 615 ... 616: // Rain and snow
      condition.icon_id = 35; // weather_128_sleet
      break;
    case 620 ... 622: // Shower snow
      condition.icon_id = 7; // weather_128_flurries
      break;
      
    // Group 7xx: Atmosphere
    case 701: // Mist
      condition.icon_id = 8; // weather_128_fog
      break;
    case 711: // Smoke
      condition.icon_id = 9; // weather_128_hazy
      break;
    case 721: // Haze
      condition.icon_id = 9; // weather_128_hazy
      break;
    case 731: // Sand/dust whirls
      condition.icon_id = 9; // weather_128_hazy
      break;
    case 741: // Fog
      condition.icon_id = 8; // weather_128_fog
      break;
    case 751: // Sand
      condition.icon_id = 9; // weather_128_hazy
      break;
    case 761: // Dust
      condition.icon_id = 9; // weather_128_hazy
      break;
    case 762: // Volcanic ash
      condition.icon_id = 9; // weather_128_hazy
      break;
    case 771: // Squalls
      condition.icon_id = 38; // weather_128_tstorms
      break;
    case 781: // Tornado
      condition.icon_id = 38; // weather_128_tstorms
      break;
      
    // Group 800: Clear
    case 800:
      condition.icon_id = 37; // weather_128_sunny
      break;
      
    // Group 80x: Clouds
    case 801: // Few clouds: 11-25%
      condition.icon_id = 32; // weather_128_partlycloudy
      break;
    case 802: // Scattered clouds: 25-50%
      condition.icon_id = 32; // weather_128_partlycloudy
      break;
    case 803: // Broken clouds: 51-84%
      condition.icon_id = 10; // weather_128_mostlycloudy
      break;
    case 804: // Overcast clouds: 85-100%
      condition.icon_id = 6; // weather_128_cloudy
      break;
      
    default:
      condition.icon_id = 39; // weather_128_unknown
      break;
   }

   return condition;
}

int WeatherData_CurrentParse(WeatherData* data, HTTPClient* http){
  String serverName = "https://api.openweathermap.org/data/2.5/weather?lat=" LATITUDE "&lon=" LONGITUDE "&appid=" API_KEY "&units=imperial";
  http->begin(serverName.c_str());

  int httpResponseCode = http->GET();
  if (httpResponseCode > 0) {
    Serial.print("Current Weather HTTP Response: ");
    Serial.println(httpResponseCode);
    
    String payload = http->getString();
    JsonDocument doc;
    DeserializationError error = deserializeJson(doc, payload);
    
    if (error) {
      Serial.print("Current Weather JSON parse error: ");
      Serial.println(error.c_str());
      http->end();
      return 0;
    }

    // Current weather endpoint structure is different - no "list" array
    data->temp = doc["main"]["temp"].as<float>();
    data->feels_like = doc["main"]["feels_like"].as<float>();
    data->daily_low = doc["main"]["temp_min"].as<float>();
    data->daily_high = doc["main"]["temp_max"].as<float>();
    data->timestamp = doc["dt"].as<time_t>();
    data->condition = WeatherData_GetCondition(doc["weather"][0]["id"].as<int>());
    
    http->end();
    return 1;  // Success - 1 entry parsed
  } else {
    Serial.print("Current Weather HTTP Error: ");
    Serial.println(httpResponseCode);
    http->end();
    return 0;
  }
}

int WeatherData_ForecastParse(WeatherData* data, int max_entries, HTTPClient* http){
  String serverName = "https://api.openweathermap.org/data/2.5/forecast?lat=" LATITUDE "&lon=" LONGITUDE "&appid=" API_KEY "&units=imperial";
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

    JsonArray list = doc["list"].as<JsonArray>();
    int count = 0;

    for (JsonObject item : list) {
      if (count >= max_entries) {
        break;
      }
      
      data[count].temp = item["main"]["temp"].as<float>();
      data[count].feels_like = item["main"]["feels_like"].as<float>();
      data[count].daily_low = item["main"]["temp_min"].as<float>();
      data[count].daily_high = item["main"]["temp_max"].as<float>();
      data[count].timestamp = item["dt"].as<time_t>();
      data[count].condition = WeatherData_GetCondition(item["weather"][0]["id"].as<int>());
      
      count++;
    }
    
    http->end();
    return count;  // Return actual count, not 1
  } else {
    Serial.print("HTTP Error code: ");
    Serial.println(httpResponseCode);
    http->end();
    return 0;
  }
}