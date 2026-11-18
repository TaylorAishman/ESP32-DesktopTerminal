//! OpenWeatherMap 2.5 API: `/weather` and `/forecast`.
//!
//! Numbers are deserialised as `f64` and narrowed afterwards: the Xtensa LLVM
//! backend cannot compile serde_json's `f32` visitor ("Cannot select:
//! XtensaISD::PCREL_WRAPPER").
//! https://openweathermap.org/current, https://openweathermap.org/forecast5

use serde::Deserialize;

use crate::model::{Condition, Current, HourlyPoint};
use crate::platform::UnixTime;

/// Map an OpenWeatherMap condition id to a [`Condition`].
/// https://openweathermap.org/weather-conditions
pub fn condition(id: u32) -> Condition {
    match id {
        200..=299 => Condition::Thunderstorm,
        300..=399 => Condition::Drizzle,
        500 | 501 | 520 | 521 => Condition::Rain,
        502..=504 | 522 | 531 => Condition::HeavyRain,
        511 | 611..=616 => Condition::Sleet,
        600..=602 | 620..=622 => Condition::Snow,
        781 => Condition::Thunderstorm,
        700..=799 => Condition::Fog,
        800 => Condition::Clear,
        801 | 802 => Condition::PartlyCloudy,
        803 | 804 => Condition::Cloudy,
        _ => Condition::Unknown,
    }
}

#[derive(Deserialize)]
pub(crate) struct WeatherEntry {
    pub id: u32,
    #[serde(default)]
    pub description: String,
    /// e.g. "10d"; the last letter is d(ay) or n(ight).
    #[serde(default)]
    pub icon: String,
}

impl WeatherEntry {
    pub fn is_day(&self) -> bool {
        !self.icon.ends_with('n')
    }
}

#[derive(Deserialize)]
struct CurrentBody {
    weather: Vec<WeatherEntry>,
    main: CurrentMain,
    #[serde(default)]
    wind: Wind,
    dt: i64,
    sys: Sys,
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct CurrentMain {
    temp: f64,
    feels_like: f64,
    humidity: u8,
}

#[derive(Deserialize, Default)]
struct Wind {
    #[serde(default)]
    speed: f64,
    deg: Option<u16>,
}

#[derive(Deserialize)]
struct Sys {
    sunrise: i64,
    sunset: i64,
}

/// Parse a `/weather` response (requested with `units=metric`).
pub fn parse_current(body: &[u8]) -> Result<Current, String> {
    let b: CurrentBody = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    let w = b.weather.first().ok_or("no weather entry")?;
    Ok(Current {
        observed_at: UnixTime(b.dt),
        location: b.name,
        temperature_c: b.main.temp as f32,
        feels_like_c: b.main.feels_like as f32,
        condition: condition(w.id),
        description: w.description.clone(),
        is_day: w.is_day(),
        humidity_pct: b.main.humidity,
        wind_speed_ms: b.wind.speed as f32,
        wind_deg: b.wind.deg,
        sunrise: UnixTime(b.sys.sunrise),
        sunset: UnixTime(b.sys.sunset),
    })
}

#[derive(Deserialize)]
struct ForecastBody {
    list: Vec<ForecastEntry>,
}

#[derive(Deserialize)]
struct ForecastEntry {
    dt: i64,
    main: ForecastMain,
    weather: Vec<WeatherEntry>,
    /// Probability of precipitation, 0 to 1.
    #[serde(default)]
    pop: f64,
    #[serde(default)]
    rain: Option<Volume>,
    #[serde(default)]
    snow: Option<Volume>,
    #[serde(default)]
    sys: Option<PartOfDay>,
}

#[derive(Deserialize)]
struct ForecastMain {
    temp: f64,
}

#[derive(Deserialize)]
struct Volume {
    #[serde(rename = "3h", default)]
    three_hours: f64,
}

#[derive(Deserialize)]
struct PartOfDay {
    pod: String,
}

/// Parse a `/forecast` response (requested with `units=metric`) into its
/// 3-hour steps, in time order.
pub fn parse_forecast(body: &[u8]) -> Result<Vec<HourlyPoint>, String> {
    let b: ForecastBody = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    if b.list.is_empty() {
        return Err("forecast has no entries".into());
    }
    let mut points: Vec<HourlyPoint> = b
        .list
        .into_iter()
        .map(|e| {
            let w = e.weather.first();
            HourlyPoint {
                at: UnixTime(e.dt),
                temperature_c: e.main.temp as f32,
                condition: w.map_or(Condition::Unknown, |w| condition(w.id)),
                is_day: match &e.sys {
                    Some(s) => s.pod != "n",
                    None => w.is_none_or(WeatherEntry::is_day),
                },
                precip_probability: e.pop.clamp(0.0, 1.0) as f32,
                precip_mm: (e.rain.map_or(0.0, |v| v.three_hours)
                    + e.snow.map_or(0.0, |v| v.three_hours)) as f32,
            }
        })
        .collect();
    points.sort_by_key(|p| p.at);
    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = include_bytes!("../../tests/fixtures/owm_current.json");
    const FORECAST: &[u8] = include_bytes!("../../tests/fixtures/owm_forecast.json");

    #[test]
    fn parses_forecast_fixture() {
        let f = parse_forecast(FORECAST).unwrap();
        assert_eq!(f.len(), 40);
        assert_eq!(f[0].at, UnixTime(1_794_927_600)); // 2026-11-17 15:00 UTC
        assert!(f.windows(2).all(|w| w[1].at.0 - w[0].at.0 == 10_800));
        assert_eq!(f[0].condition, Condition::Clear);
        assert!(f[0].is_day);
        assert_eq!(f[5].condition, Condition::Rain);
        assert_eq!(f[5].precip_mm, 0.6);
        assert_eq!(f[10].condition, Condition::Snow);
        assert_eq!(f[10].precip_mm, 1.1);
        assert!((f[4].precip_probability - 0.8).abs() < 1e-6);
    }

    #[test]
    fn forecast_fits_the_configured_buffer() {
        // config/api.toml: forecast max_response_bytes = 24576.
        assert!(FORECAST.len() < 24_576 * 3 / 4, "{} bytes", FORECAST.len());
    }

    #[test]
    fn empty_forecast_is_an_error() {
        assert_eq!(
            parse_forecast(br#"{"list":[]}"#).unwrap_err(),
            "forecast has no entries"
        );
    }

    #[test]
    fn parses_fixture() {
        let c = parse_current(FIXTURE).unwrap();
        assert_eq!(c.observed_at, UnixTime(1_794_924_309));
        assert_eq!(c.location, "New York");
        assert_eq!(c.temperature_c, 8.42);
        assert_eq!(c.feels_like_c, 5.61);
        assert_eq!(c.condition, Condition::Rain);
        assert_eq!(c.description, "light rain");
        assert!(c.is_day);
        assert_eq!(c.humidity_pct, 81);
        assert_eq!(c.wind_speed_ms, 4.63);
        assert_eq!(c.wind_deg, Some(250));
        assert_eq!(c.sunrise, UnixTime(1_794_917_400));
    }

    #[test]
    fn optional_fields_may_be_missing() {
        let body = br#"{"weather":[{"id":800}],"main":{"temp":1,"feels_like":0,"humidity":50},"dt":5,"sys":{"sunrise":1,"sunset":2}}"#;
        let c = parse_current(body).unwrap();
        assert_eq!(c.condition, Condition::Clear);
        assert_eq!(c.wind_deg, None);
        assert_eq!(c.wind_speed_ms, 0.0);
        assert!(c.is_day);
    }

    #[test]
    fn errors_are_reported() {
        assert!(parse_current(b"{}").unwrap_err().contains("missing field"));
        let no_entry = br#"{"weather":[],"main":{"temp":1,"feels_like":0,"humidity":50},"dt":5,"sys":{"sunrise":1,"sunset":2}}"#;
        assert_eq!(parse_current(no_entry).unwrap_err(), "no weather entry");
    }

    #[test]
    fn condition_mapping() {
        assert_eq!(condition(211), Condition::Thunderstorm);
        assert_eq!(condition(301), Condition::Drizzle);
        assert_eq!(condition(502), Condition::HeavyRain);
        assert_eq!(condition(511), Condition::Sleet);
        assert_eq!(condition(601), Condition::Snow);
        assert_eq!(condition(741), Condition::Fog);
        assert_eq!(condition(781), Condition::Thunderstorm);
        assert_eq!(condition(802), Condition::PartlyCloudy);
        assert_eq!(condition(804), Condition::Cloudy);
        assert_eq!(condition(999), Condition::Unknown);
    }
}
