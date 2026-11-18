//! The configuration as compiled into the firmware.
//!
//! `dt-config` validates `config/*.toml` at build time and generates a
//! `static CONFIG: Config` from them (Architecture D-4), so the device never
//! parses configuration and a bad file can never reach it.

use crate::api::Limit;
use crate::secret::Secret;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub user_agent: &'static str,
    pub wifi: WifiConfig,
    pub location: LocationConfig,
    pub providers: &'static [ProviderConfig],
    /// News topics in display order.
    pub topics: &'static [TopicConfig],
    /// Instruments in display order.
    pub instruments: &'static [InstrumentConfig],
}

impl Config {
    pub fn provider(&self, id: &str) -> Option<&'static ProviderConfig> {
        self.providers.iter().find(|p| p.id == id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WifiConfig {
    pub ssid: &'static str,
    pub password: Secret<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Units {
    Metric,
    Imperial,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocationConfig {
    pub name: &'static str,
    pub latitude: f64,
    pub longitude: f64,
    /// POSIX TZ string.
    pub timezone: &'static str,
    pub units: Units,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthLocation {
    Query,
    Header,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthConfig {
    pub location: AuthLocation,
    /// Query parameter or header name.
    pub name: &'static str,
    /// Name of the key in config/secrets.toml, for error hints.
    pub secret_name: &'static str,
    /// The API key itself.
    pub key: Secret<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProviderConfig {
    /// e.g. "newsapi".
    pub id: &'static str,
    /// e.g. "NewsAPI".
    pub name: &'static str,
    pub base_url: &'static str,
    /// Host part of `base_url`, for messages.
    pub host: &'static str,
    pub auth: AuthConfig,
    pub limits: &'static [Limit],
    pub utilisation: f64,
    pub timeout_ms: u32,
    pub endpoints: &'static [EndpointConfig],
}

impl ProviderConfig {
    pub fn endpoint(&self, id: &str) -> Option<&'static EndpointConfig> {
        self.endpoints.iter().find(|e| e.id == id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndpointConfig {
    pub id: &'static str,
    pub path: &'static str,
    pub params: &'static [(&'static str, &'static str)],
    pub weight: u32,
    pub min_interval_s: u32,
    pub max_staleness_s: u32,
    pub max_response_bytes: u32,
    pub off_hours_interval_s: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TopicConfig {
    pub title: &'static str,
    pub params: &'static [(&'static str, &'static str)],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstrumentConfig {
    pub label: &'static str,
    pub symbol: &'static str,
}
