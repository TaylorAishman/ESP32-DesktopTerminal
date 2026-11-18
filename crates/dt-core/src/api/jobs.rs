//! The list of fetch jobs, built from the configuration.

use core::time::Duration;

use super::derive::{derive, JobSpec, JobTiming};
use crate::config::{Config, EndpointConfig, ProviderConfig};

/// Provider and endpoint ids the firmware uses (they must exist in
/// config/api.toml; dt-config checks this at build time).
pub const WEATHER_CURRENT: (&str, &str) = ("openweathermap", "current");
pub const WEATHER_FORECAST: (&str, &str) = ("openweathermap", "forecast");
pub const NEWS_HEADLINES: (&str, &str) = ("newsapi", "top_headlines");
pub const FINANCE_QUOTE: (&str, &str) = ("finnhub", "quote");

/// What a job fetches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum JobKind {
    CurrentWeather,
    Forecast,
    /// Index into `Config::topics`.
    News(usize),
    /// Index into `Config::instruments`.
    Quote(usize),
}

/// One recurring fetch.
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    pub kind: JobKind,
    pub provider: &'static ProviderConfig,
    pub endpoint: &'static EndpointConfig,
    /// Parameters added to the endpoint's fixed ones.
    pub params: Vec<(String, String)>,
    /// Interval and phase derived from the provider's limits.
    pub timing: JobTiming,
    /// Interval outside market hours, for quote jobs.
    pub off_hours_interval: Option<Duration>,
}

impl Job {
    /// Short name for logs, e.g. "news[2]".
    pub fn name(&self) -> String {
        match self.kind {
            JobKind::CurrentWeather => "weather.current".into(),
            JobKind::Forecast => "weather.forecast".into(),
            JobKind::News(i) => format!("news[{i}]"),
            JobKind::Quote(i) => format!("quote[{i}]"),
        }
    }

    pub fn params_ref(&self) -> Vec<(&str, &str)> {
        self.params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}

/// A job before its timing is derived: kind, (provider, endpoint), params.
type Wanted = (JobKind, (&'static str, &'static str), Vec<(String, String)>);

/// Build every job and derive its timing. Jobs whose endpoint is missing
/// from the config are skipped (dt-config rejects such configs at build time).
pub fn build_jobs(config: &'static Config) -> Vec<Job> {
    let loc = &config.location;
    let coords = vec![
        ("lat".to_owned(), format!("{:.4}", loc.latitude)),
        ("lon".to_owned(), format!("{:.4}", loc.longitude)),
    ];
    let mut wanted: Vec<Wanted> = vec![
        (JobKind::CurrentWeather, WEATHER_CURRENT, coords.clone()),
        (JobKind::Forecast, WEATHER_FORECAST, coords),
    ];
    for (i, t) in config.topics.iter().enumerate() {
        let params = t
            .params
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        wanted.push((JobKind::News(i), NEWS_HEADLINES, params));
    }
    for (i, inst) in config.instruments.iter().enumerate() {
        wanted.push((
            JobKind::Quote(i),
            FINANCE_QUOTE,
            vec![("symbol".to_owned(), inst.symbol.to_owned())],
        ));
    }

    let mut jobs: Vec<Job> = Vec::new();
    for provider in config.providers {
        let mine: Vec<_> = wanted
            .iter()
            .filter(|(_, (p, _), _)| *p == provider.id)
            .filter_map(|(kind, (_, e), params)| {
                provider.endpoint(e).map(|ep| (*kind, ep, params.clone()))
            })
            .collect();
        let specs: Vec<JobSpec> = mine
            .iter()
            .map(|(_, ep, _)| JobSpec {
                weight: ep.weight,
                min_interval: Duration::from_secs(u64::from(ep.min_interval_s)),
            })
            .collect();
        let timings = derive(provider.limits, provider.utilisation, &specs);
        for ((kind, endpoint, params), timing) in mine.into_iter().zip(timings) {
            jobs.push(Job {
                kind,
                provider,
                endpoint,
                params,
                timing,
                off_hours_interval: endpoint
                    .off_hours_interval_s
                    .map(|s| Duration::from_secs(u64::from(s))),
            });
        }
    }
    jobs
}
