//! The generated configuration compiles and carries the values from the files.

use dt_core::api::{Limit, Period};
use dt_core::config::{AuthLocation, Units};
use dt_sim::config::CONFIG;

#[test]
fn round_trips_the_example_config() {
    assert!(CONFIG.user_agent.starts_with("ESP32-DesktopTerminal/"));
    assert_eq!(CONFIG.wifi.ssid, "YOUR_WIFI_NAME");
    assert_eq!(format!("{:?}", CONFIG.wifi.password), "***");
    assert_eq!(CONFIG.location.units, Units::Imperial);
    assert_eq!(CONFIG.location.latitude, 40.7128);

    let news = CONFIG.provider("newsapi").unwrap();
    assert_eq!(news.host, "newsapi.org");
    assert_eq!(news.auth.location, AuthLocation::Header);
    assert_eq!(news.auth.name, "X-Api-Key");
    assert_eq!(news.auth.key.expose(), "PASTE_NEWSAPI_KEY_HERE");
    assert_eq!(
        news.limits,
        [Limit {
            calls: 100,
            per: Period::Day
        }]
    );
    let th = news.endpoint("top_headlines").unwrap();
    assert_eq!(th.params, [("pageSize", "6")]);
    assert_eq!(th.max_response_bytes, 16384);

    assert_eq!(
        CONFIG
            .provider("finnhub")
            .unwrap()
            .endpoint("quote")
            .unwrap()
            .off_hours_interval_s,
        Some(3600)
    );
    assert_eq!(CONFIG.topics.len(), 4);
    assert_eq!(CONFIG.topics[1].title, "World");
    assert_eq!(CONFIG.instruments[0].label, "DJIA (DIA)");
    assert_eq!(CONFIG.instruments[0].symbol, "DIA");
}
