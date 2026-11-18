//! The example configuration yields the expected job table.

use std::time::Duration;

use dt_core::api::{build_jobs, JobKind};
use dt_sim::config::CONFIG;

#[test]
fn example_config_job_table() {
    let jobs = build_jobs(&CONFIG);
    let kinds: Vec<JobKind> = jobs.iter().map(|j| j.kind).collect();
    // Providers in config order (finnhub, newsapi, openweathermap).
    assert_eq!(
        kinds,
        [
            JobKind::Quote(0),
            JobKind::Quote(1),
            JobKind::Quote(2),
            JobKind::Quote(3),
            JobKind::News(0),
            JobKind::News(1),
            JobKind::News(2),
            JobKind::News(3),
            JobKind::CurrentWeather,
            JobKind::Forecast,
        ]
    );

    let quote = &jobs[1];
    assert_eq!(quote.params, [("symbol".to_owned(), "VOO".to_owned())]);
    assert_eq!(quote.timing.interval, Duration::from_secs(60));
    assert_eq!(quote.off_hours_interval, Some(Duration::from_secs(3600)));

    let news: Vec<_> = jobs
        .iter()
        .filter(|j| matches!(j.kind, JobKind::News(_)))
        .collect();
    assert!(news
        .iter()
        .all(|j| j.timing.interval == Duration::from_secs(3840)));
    let phases: Vec<u64> = news.iter().map(|j| j.timing.phase.as_secs()).collect();
    assert_eq!(phases, [0, 960, 1920, 2880]);
    assert_eq!(
        news[1].params,
        [(
            "sources".to_owned(),
            "bbc-news,associated-press,al-jazeera-english".to_owned()
        )]
    );

    let current = jobs
        .iter()
        .find(|j| j.kind == JobKind::CurrentWeather)
        .unwrap();
    assert_eq!(
        current.params,
        [
            ("lat".to_owned(), "40.7128".to_owned()),
            ("lon".to_owned(), "-74.0060".to_owned())
        ]
    );
    assert_eq!(current.timing.interval, Duration::from_secs(600));
    assert_eq!(current.name(), "weather.current");
}
