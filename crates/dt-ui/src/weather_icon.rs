//! Which icon shows which weather.

use dt_core::model::Condition;

use crate::icon::Icon;

/// The icon for a condition, by day or by night.
pub fn condition_icon(condition: Condition, is_day: bool) -> Icon {
    match (condition, is_day) {
        (Condition::Clear, true) => Icon::ClearDay,
        (Condition::Clear, false) => Icon::ClearNight,
        (Condition::PartlyCloudy, true) => Icon::PartlyCloudyDay,
        (Condition::PartlyCloudy, false) => Icon::PartlyCloudyNight,
        (Condition::Cloudy, _) => Icon::Cloudy,
        (Condition::Fog, _) => Icon::Fog,
        (Condition::Drizzle, _) => Icon::Drizzle,
        (Condition::Rain, _) => Icon::Rain,
        (Condition::HeavyRain, _) => Icon::RainHeavy,
        (Condition::Thunderstorm, _) => Icon::Thunderstorm,
        (Condition::Snow, _) => Icon::Snow,
        (Condition::Sleet, _) => Icon::Sleet,
        (Condition::Unknown, _) => Icon::Unknown,
    }
}

/// Weather icons in a sensible review order.
pub const WEATHER_ICONS: [Icon; 18] = [
    Icon::ClearDay,
    Icon::ClearNight,
    Icon::PartlyCloudyDay,
    Icon::PartlyCloudyNight,
    Icon::Cloudy,
    Icon::Fog,
    Icon::Drizzle,
    Icon::Rain,
    Icon::RainHeavy,
    Icon::Thunderstorm,
    Icon::Snow,
    Icon::Sleet,
    Icon::Unknown,
    Icon::Wind,
    Icon::Humidity,
    Icon::Precipitation,
    Icon::Sunrise,
    Icon::Sunset,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_condition_has_day_and_night_icons() {
        for c in Condition::ALL {
            let _ = condition_icon(c, true);
            let _ = condition_icon(c, false);
        }
        assert_ne!(
            condition_icon(Condition::Clear, true),
            condition_icon(Condition::Clear, false)
        );
    }
}
