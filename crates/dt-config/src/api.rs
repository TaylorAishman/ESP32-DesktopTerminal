//! Schema of `config/api.toml`: providers, endpoints and limits.

use std::collections::BTreeMap;

use dt_core::api::Period;
use serde::Deserialize;

use crate::error::ConfigError;

pub const API_FILE: &str = "config/api.toml";

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiConfig {
    /// Sent as the `User-Agent` header on every request.
    pub user_agent: String,
    /// Keyed by provider id, e.g. `newsapi`.
    pub providers: BTreeMap<String, Provider>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provider {
    /// Display name, e.g. "NewsAPI".
    pub name: String,
    pub base_url: String,
    pub auth: Auth,
    pub limits: Vec<LimitSpec>,
    /// Share of the tightest limit the schedule may use, 0 < u <= 1.
    #[serde(default = "default_utilisation")]
    pub utilisation: f64,
    pub timeout_ms: u32,
    /// Keyed by endpoint id, e.g. `top_headlines`.
    pub endpoints: BTreeMap<String, Endpoint>,
}

fn default_utilisation() -> f64 {
    0.9
}

/// How the API key is sent.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Auth {
    pub send_as: AuthLocation,
    /// Query parameter or header name.
    pub name: String,
    /// Key in `[api_keys]` of config/secrets.toml.
    pub secret: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthLocation {
    Query,
    Header,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimitSpec {
    pub calls: u32,
    pub per: Period,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    /// Appended to the provider's `base_url`.
    pub path: String,
    /// Fixed query parameters.
    #[serde(default)]
    pub params: BTreeMap<String, String>,
    /// Share of the provider's budget for each job using this endpoint.
    #[serde(default = "default_weight")]
    pub weight: u32,
    /// Never poll faster than this.
    pub min_interval_s: u32,
    /// Build fails if the derived interval is longer than this.
    pub max_staleness_s: u32,
    /// Size of the receive buffer for this endpoint's responses.
    pub max_response_bytes: u32,
    /// Interval outside US equity market hours (quotes only).
    #[serde(default)]
    pub off_hours_interval_s: Option<u32>,
}

fn default_weight() -> u32 {
    1
}

/// Parse the text of `config/api.toml`.
pub fn parse_api(text: &str) -> Result<ApiConfig, ConfigError> {
    toml::from_str(text).map_err(|e| ConfigError::from_toml(API_FILE, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMITTED: &str = include_str!("../../../config/api.toml");

    #[test]
    fn parses_committed_api_toml() {
        let api = parse_api(COMMITTED).expect("config/api.toml parses");
        assert!(api.user_agent.starts_with("ESP32-DesktopTerminal/"));
        assert_eq!(
            api.providers.keys().collect::<Vec<_>>(),
            ["finnhub", "newsapi", "openweathermap"]
        );
        let news = &api.providers["newsapi"];
        assert_eq!(news.auth.send_as, AuthLocation::Header);
        assert_eq!(
            news.limits,
            [LimitSpec {
                calls: 100,
                per: Period::Day
            }]
        );
        let th = &news.endpoints["top_headlines"];
        assert_eq!(th.params["pageSize"], "6");
        assert_eq!(th.weight, 1);
        let owm = &api.providers["openweathermap"];
        assert_eq!(owm.limits.len(), 2);
        assert_eq!(owm.auth.send_as, AuthLocation::Query);
        let quote = &api.providers["finnhub"].endpoints["quote"];
        assert_eq!(quote.off_hours_interval_s, Some(3600));
    }

    #[test]
    fn unknown_keys_are_rejected_with_location() {
        let text = COMMITTED.replace("min_interval_s = 900", "min_intervall_s = 900");
        let e = parse_api(&text).unwrap_err();
        assert_eq!(e.file, API_FILE);
        assert!(e.message.contains("min_intervall_s"), "{}", e.message);
        assert!(e.message.contains("line"), "{}", e.message);
    }

    #[test]
    fn bad_period_is_rejected() {
        let text = COMMITTED.replace(r#"per = "day""#, r#"per = "fortnight""#);
        assert!(parse_api(&text).unwrap_err().message.contains("fortnight"));
    }
}
