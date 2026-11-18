//! Schema of `config/user.toml`: location, news topics and instruments.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::error::ConfigError;

pub const USER_FILE: &str = "config/user.toml";
pub const USER_EXAMPLE_FILE: &str = "config/user.example.toml";

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserConfig {
    pub location: Location,
    pub news: News,
    pub finance: Finance,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    /// Shown on screen.
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    /// POSIX TZ string, e.g. "EST5EDT,M3.2.0,M11.1.0".
    pub timezone: String,
    pub units: Units,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    Metric,
    Imperial,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct News {
    pub topics: Vec<NewsTopic>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewsTopic {
    pub title: String,
    /// Query parameters for the top-headlines endpoint.
    #[serde(default)]
    pub params: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finance {
    pub instruments: Vec<Instrument>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instrument {
    /// Shown on screen.
    pub label: String,
    /// Ticker sent to the provider.
    pub symbol: String,
}

/// Parse the text of a user config file (`file` is used in messages).
pub fn parse_user(file: &str, text: &str) -> Result<UserConfig, ConfigError> {
    toml::from_str(text).map_err(|e| ConfigError::from_toml(file, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../../../config/user.example.toml");

    #[test]
    fn parses_example() {
        let u = parse_user(USER_EXAMPLE_FILE, EXAMPLE).unwrap();
        assert_eq!(u.location.units, Units::Imperial);
        assert!(u.location.timezone.starts_with("EST5EDT"));
        let titles: Vec<_> = u.news.topics.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, ["U.S.", "World", "Business", "Technology"]);
        assert_eq!(u.news.topics[0].params["country"], "us");
        let symbols: Vec<_> = u
            .finance
            .instruments
            .iter()
            .map(|i| i.symbol.as_str())
            .collect();
        assert_eq!(symbols, ["DIA", "VOO", "VXUS", "VTI"]);
        assert_eq!(u.finance.instruments[0].label, "DIA");
    }

    #[test]
    fn bad_units_name_the_file() {
        let text = EXAMPLE.replace(r#"units = "imperial""#, r#"units = "kelvin""#);
        let e = parse_user(USER_FILE, &text).unwrap_err();
        assert_eq!(e.file, USER_FILE);
        assert!(e.message.contains("kelvin"));
    }
}
