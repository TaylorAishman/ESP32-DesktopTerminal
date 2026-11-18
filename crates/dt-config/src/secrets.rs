//! Schema of `config/secrets.toml`: Wi-Fi credentials and API keys.

use std::collections::BTreeMap;

use dt_core::Secret;
use serde::Deserialize;

use crate::error::ConfigError;

pub const SECRETS_FILE: &str = "config/secrets.toml";
pub const SECRETS_EXAMPLE_FILE: &str = "config/secrets.example.toml";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecretsConfig {
    pub wifi: Wifi,
    /// Keyed by the `auth.secret` names in config/api.toml.
    pub api_keys: BTreeMap<String, Secret<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wifi {
    pub ssid: String,
    pub password: Secret<String>,
}

/// Parse the text of a secrets file (`file` is used in messages).
///
/// TOML parse errors quote the offending line, which could contain a secret,
/// so only the position is kept.
pub fn parse_secrets(file: &str, text: &str) -> Result<SecretsConfig, ConfigError> {
    toml::from_str(text).map_err(|e| {
        let position = e
            .span()
            .map(|span| {
                let line = text[..span.start].matches('\n').count() + 1;
                format!(" at line {line}")
            })
            .unwrap_or_default();
        ConfigError::new(file, format!("invalid TOML{position}: {}", e.message()))
            .fix("Compare with config/secrets.example.toml.")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../../../config/secrets.example.toml");

    #[test]
    fn parses_example() {
        let s = parse_secrets(SECRETS_EXAMPLE_FILE, EXAMPLE).unwrap();
        assert_eq!(s.wifi.ssid, "YOUR_WIFI_NAME");
        assert_eq!(
            s.api_keys.keys().collect::<Vec<_>>(),
            ["finnhub", "newsapi", "openweathermap"]
        );
    }

    #[test]
    fn formatting_never_shows_keys() {
        let text = EXAMPLE
            .replace("PASTE_NEWSAPI_KEY_HERE", "real-news-key-123")
            .replace("YOUR_WIFI_PASSWORD", "real-wifi-pass");
        let s = parse_secrets(SECRETS_FILE, &text).unwrap();
        assert_eq!(s.api_keys["newsapi"].expose(), "real-news-key-123");
        let shown = format!("{s:?} {:#?}", s);
        assert!(!shown.contains("real-news-key-123"));
        assert!(!shown.contains("real-wifi-pass"));
    }

    #[test]
    fn parse_errors_do_not_quote_the_file() {
        let text = "[wifi]\nssid = \"x\"\npassword = \"leaky-secret\n";
        let e = parse_secrets(SECRETS_FILE, text).unwrap_err();
        assert!(!e.to_string().contains("leaky-secret"), "{e}");
        assert!(e.message.contains("line 3"), "{e}");
    }
}
