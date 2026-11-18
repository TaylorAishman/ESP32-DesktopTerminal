//! Weather models. Provider-neutral and always metric: temperatures in °C,
//! wind in m/s, precipitation in mm. Conversion to the user's units happens
//! when formatting for display.

use crate::platform::UnixTime;
use crate::time::Date;

/// Sky and precipitation, coarse enough to map to one icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Condition {
    Clear,
    PartlyCloudy,
    Cloudy,
    Fog,
    Drizzle,
    Rain,
    HeavyRain,
    Thunderstorm,
    Snow,
    Sleet,
    Unknown,
}

impl Condition {
    /// Every condition, for icon coverage tests.
    pub const ALL: [Condition; 11] = [
        Condition::Clear,
        Condition::PartlyCloudy,
        Condition::Cloudy,
        Condition::Fog,
        Condition::Drizzle,
        Condition::Rain,
        Condition::HeavyRain,
        Condition::Thunderstorm,
        Condition::Snow,
        Condition::Sleet,
        Condition::Unknown,
    ];

    /// Ranking used when several conditions compete for one summary icon:
    /// the more significant weather wins.
    pub fn severity(self) -> u8 {
        match self {
            Condition::Unknown => 0,
            Condition::Clear => 1,
            Condition::PartlyCloudy => 2,
            Condition::Cloudy => 3,
            Condition::Fog => 4,
            Condition::Drizzle => 5,
            Condition::Rain => 6,
            Condition::Sleet => 7,
            Condition::Snow => 8,
            Condition::HeavyRain => 9,
            Condition::Thunderstorm => 10,
        }
    }
}

/// Conditions now.
#[derive(Clone, Debug, PartialEq)]
pub struct Current {
    /// When the provider observed these values.
    pub observed_at: UnixTime,
    /// Place name as reported by the provider.
    pub location: String,
    pub temperature_c: f32,
    pub feels_like_c: f32,
    pub condition: Condition,
    /// Short provider description, e.g. "light rain".
    pub description: String,
    pub is_day: bool,
    pub humidity_pct: u8,
    pub wind_speed_ms: f32,
    /// Direction the wind comes from, degrees clockwise from north.
    pub wind_deg: Option<u16>,
    pub sunrise: UnixTime,
    pub sunset: UnixTime,
}

/// One forecast step (3 hours with the OpenWeatherMap 2.5 API).
#[derive(Clone, Debug, PartialEq)]
pub struct HourlyPoint {
    pub at: UnixTime,
    pub temperature_c: f32,
    pub condition: Condition,
    pub is_day: bool,
    /// Probability of precipitation, 0.0 to 1.0.
    pub precip_probability: f32,
    /// Expected precipitation during the step, in mm.
    pub precip_mm: f32,
}

/// One local calendar day, summarised from forecast steps.
#[derive(Clone, Debug, PartialEq)]
pub struct DailySummary {
    pub date: Date,
    pub min_c: f32,
    pub max_c: f32,
    pub condition: Condition,
    /// Highest probability of precipitation during the day, 0.0 to 1.0.
    pub precip_probability: f32,
}

/// The forecast: steps as provided, and per-day summaries computed from them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Forecast {
    pub hourly: Vec<HourlyPoint>,
    pub daily: Vec<DailySummary>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_is_a_total_order() {
        let mut s: Vec<u8> = Condition::ALL.iter().map(|c| c.severity()).collect();
        s.sort_unstable();
        s.dedup();
        assert_eq!(s.len(), Condition::ALL.len());
    }
}
