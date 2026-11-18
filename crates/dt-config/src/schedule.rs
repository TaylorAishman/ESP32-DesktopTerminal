//! The derived polling schedule, printed by the build and checked against each
//! endpoint's `max_staleness_s` (REQ-API-04, REQ-API-07).

use core::fmt::Write as _;
use core::time::Duration;

use dt_core::api::derive::binding_rate;
use dt_core::api::{derive, JobSpec, JobTiming, Limit};

use crate::api::{ApiConfig, Endpoint, Provider};
use crate::error::{ConfigError, ConfigErrors};
use crate::user::UserConfig;
use crate::validate::{FINANCE_QUOTE, NEWS_HEADLINES};

/// One endpoint's share of the schedule.
#[derive(Clone, Debug, PartialEq)]
pub struct EndpointSchedule {
    pub provider: String,
    pub endpoint: String,
    /// Number of jobs using this endpoint (topics, instruments, or 1).
    pub jobs: usize,
    /// Timing of each job, in order.
    pub timings: Vec<JobTiming>,
    pub max_staleness: Duration,
}

/// The whole schedule, provider by provider.
#[derive(Clone, Debug, PartialEq)]
pub struct Schedule {
    pub endpoints: Vec<EndpointSchedule>,
}

/// How many jobs an endpoint gets from the user's config.
pub fn job_count(provider: &str, endpoint: &str, user: &UserConfig) -> usize {
    match (provider, endpoint) {
        p if p == NEWS_HEADLINES => user.news.topics.len(),
        p if p == FINANCE_QUOTE => user.finance.instruments.len(),
        _ => 1,
    }
}

pub fn limits_of(p: &Provider) -> Vec<Limit> {
    p.limits
        .iter()
        .map(|l| Limit {
            calls: l.calls,
            per: l.per,
        })
        .collect()
}

fn spec(e: &Endpoint) -> JobSpec {
    JobSpec {
        weight: e.weight,
        min_interval: Duration::from_secs(u64::from(e.min_interval_s)),
    }
}

/// Derive the schedule for every endpoint of every provider.
pub fn derive_schedule(api: &ApiConfig, user: &UserConfig) -> Schedule {
    let mut endpoints = Vec::new();
    for (pid, p) in &api.providers {
        // Jobs of all endpoints of a provider share its budget.
        let mut owners = Vec::new();
        let mut specs = Vec::new();
        for (eid, e) in &p.endpoints {
            for _ in 0..job_count(pid, eid, user) {
                owners.push(eid.as_str());
                specs.push(spec(e));
            }
        }
        let timings = derive(&limits_of(p), p.utilisation, &specs);
        for (eid, e) in &p.endpoints {
            let mine: Vec<JobTiming> = owners
                .iter()
                .zip(&timings)
                .filter(|(o, _)| **o == eid.as_str())
                .map(|(_, t)| *t)
                .collect();
            endpoints.push(EndpointSchedule {
                provider: pid.clone(),
                endpoint: eid.clone(),
                jobs: mine.len(),
                timings: mine,
                max_staleness: Duration::from_secs(u64::from(e.max_staleness_s)),
            });
        }
    }
    Schedule { endpoints }
}

/// Fail if any job would refresh less often than its endpoint allows.
pub fn check_staleness(
    api: &ApiConfig,
    schedule: &Schedule,
    api_file: &str,
) -> Result<(), ConfigErrors> {
    let mut errors = Vec::new();
    for es in &schedule.endpoints {
        let Some(worst) = es.timings.iter().map(|t| t.interval).max() else {
            continue;
        };
        if worst <= es.max_staleness {
            continue;
        }
        let p = &api.providers[&es.provider];
        let (_, binding) =
            binding_rate(&limits_of(p)).expect("staleness can only exceed with a limit");
        let provider_jobs: usize = schedule
            .endpoints
            .iter()
            .filter(|o| o.provider == es.provider)
            .map(|o| o.jobs)
            .sum();
        let usable = f64::from(binding.calls) * p.utilisation;
        errors.push(
            ConfigError::new(
                api_file,
                format!(
                    "{provider_jobs} jobs share {usable:.0} calls per {per} ({calls}/{per} × {util}), \
                     so each '{ep}' job refreshes every {interval}, longer than max_staleness_s = {max} ({maxh})",
                    per = period_name(binding),
                    calls = binding.calls,
                    util = p.utilisation,
                    ep = es.endpoint,
                    interval = human(worst),
                    max = es.max_staleness.as_secs(),
                    maxh = human(es.max_staleness),
                ),
            )
            .at(format!("providers.{}.endpoints.{}.max_staleness_s", es.provider, es.endpoint))
            .fix("Use fewer topics or instruments in config/user.toml, raise the limit if your plan allows more, or accept staler data by raising max_staleness_s."),
        );
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ConfigErrors(errors))
    }
}

/// The schedule as a table, for the build log.
pub fn render_table(api: &ApiConfig, schedule: &Schedule) -> String {
    let mut out = String::new();
    out.push_str(
        "provider        endpoint       jobs      interval  per job/day  provider/day  binding limit\n",
    );
    for es in &schedule.endpoints {
        let p = &api.providers[&es.provider];
        let provider_total: f64 = schedule
            .endpoints
            .iter()
            .filter(|o| o.provider == es.provider)
            .flat_map(|o| o.timings.iter().map(JobTiming::calls_per_day))
            .sum();
        let (interval, per_day) = match es.timings.first() {
            Some(t) => {
                let floor = if t.floored { " (floor)" } else { "" };
                (
                    format!("{}{floor}", human(t.interval)),
                    format!("{:.1}", t.calls_per_day()),
                )
            }
            None => ("-".into(), "-".into()),
        };
        let binding = binding_rate(&limits_of(p))
            .map(|(_, l)| format!("{}/{} × {}", l.calls, period_name(l), p.utilisation))
            .unwrap_or_else(|| "none".into());
        let _ = writeln!(
            out,
            "{:<15} {:<14} {:>4} {:>13} {:>12} {:>13.1}  {}",
            es.provider, es.endpoint, es.jobs, interval, per_day, provider_total, binding
        );
    }
    out
}

fn period_name(l: Limit) -> &'static str {
    use dt_core::api::Period::*;
    match l.per {
        Minute => "minute",
        Hour => "hour",
        Day => "day",
        Month => "month",
    }
}

/// "6s", "10m", "1h04m".
pub fn human(d: Duration) -> String {
    let s = d.as_secs();
    match s {
        0..=59 => format!("{s}s"),
        60..=3599 if s % 60 == 0 => format!("{}m", s / 60),
        60..=3599 => format!("{}m{:02}s", s / 60, s % 60),
        _ => format!("{}h{:02}m", s / 3600, (s % 3600) / 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_api, parse_user};

    const API: &str = include_str!("../../../config/api.toml");
    const USER: &str = include_str!("../../../config/user.example.toml");

    fn schedule(api: &str, user: &str) -> (ApiConfig, Schedule) {
        let api = parse_api(api).unwrap();
        let user = parse_user("config/user.toml", user).unwrap();
        let s = derive_schedule(&api, &user);
        (api, s)
    }

    fn find<'a>(s: &'a Schedule, p: &str, e: &str) -> &'a EndpointSchedule {
        s.endpoints
            .iter()
            .find(|x| x.provider == p && x.endpoint == e)
            .unwrap()
    }

    #[test]
    fn default_schedule_matches_the_architecture() {
        let (api, s) = schedule(API, USER);
        check_staleness(&api, &s, "config/api.toml").unwrap();
        let news = find(&s, "newsapi", "top_headlines");
        assert_eq!(news.jobs, 4);
        assert!(news
            .timings
            .iter()
            .all(|t| t.interval == Duration::from_secs(3840)));
        assert_eq!(
            find(&s, "openweathermap", "current").timings[0].interval,
            Duration::from_secs(600)
        );
        assert_eq!(
            find(&s, "openweathermap", "forecast").timings[0].interval,
            Duration::from_secs(1800)
        );
        assert_eq!(find(&s, "finnhub", "quote").jobs, 4);
        assert_eq!(
            find(&s, "finnhub", "quote").timings[0].interval,
            Duration::from_secs(60)
        );

        let table = render_table(&api, &s);
        assert!(
            table.contains(
                "newsapi         top_headlines     4         1h04m         22.5          90.0  100/day × 0.9"
            ),
            "\n{table}"
        );
        assert!(table.contains("10m (floor)"), "\n{table}");
    }

    #[test]
    fn too_little_budget_fails_with_the_arithmetic() {
        let api = API.replace(
            "{ calls = 100, per = \"day\" }",
            "{ calls = 20, per = \"day\" }",
        );
        let (api, s) = schedule(&api, USER);
        let e = check_staleness(&api, &s, "config/api.toml")
            .unwrap_err()
            .to_string();
        assert!(
            e.contains("4 jobs share 18 calls per day (20/day × 0.9), so each 'top_headlines' job refreshes every 5h20m, longer than max_staleness_s = 14400 (4h00m)"),
            "{e}"
        );
        assert!(
            e.contains(
                "config/api.toml: providers.newsapi.endpoints.top_headlines.max_staleness_s"
            ),
            "{e}"
        );
    }

    #[test]
    fn human_durations() {
        assert_eq!(human(Duration::from_secs(6)), "6s");
        assert_eq!(human(Duration::from_secs(600)), "10m");
        assert_eq!(human(Duration::from_secs(90)), "1m30s");
        assert_eq!(human(Duration::from_secs(3840)), "1h04m");
    }
}
