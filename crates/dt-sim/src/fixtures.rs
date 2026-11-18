//! A store filled from the sample provider responses, at a fixed time, so
//! page renders are deterministic.

use dt_core::model::Forecast;
use dt_core::parse::{finnhub, newsapi, owm};
use dt_core::platform::UnixTime;
use dt_core::store::{Store, Update, UpdateKind};
use dt_ui::view::Context;

use crate::config::CONFIG;

const CURRENT: &[u8] = include_bytes!("../../dt-core/tests/fixtures/owm_current.json");
const FORECAST: &[u8] = include_bytes!("../../dt-core/tests/fixtures/owm_forecast.json");
const NEWS: &[u8] = include_bytes!("../../dt-core/tests/fixtures/newsapi_top_headlines.json");

/// Tue 17 Nov 2026, 15:20 UTC (10:20 in New York).
pub const NOW: UnixTime = UnixTime(1_794_928_800);
/// New York in November.
pub const OFFSET: i32 = -5 * 3600;

pub fn context() -> Context {
    Context::from_config(&CONFIG, Some(NOW), OFFSET, Some(-58))
}

/// Every slot filled with data fetched a few minutes before [`NOW`].
pub fn full_store() -> Store {
    let mut s = Store::new(CONFIG.topics.len(), CONFIG.instruments.len());
    let at = UnixTime(NOW.0 - 300);
    s.apply(Update {
        at,
        kind: UpdateKind::Current(Ok(owm::parse_current(CURRENT).unwrap())),
    });
    let points = owm::parse_forecast(FORECAST).unwrap();
    s.apply(Update {
        at,
        kind: UpdateKind::Forecast(Ok(Forecast::from_points(points, |_| OFFSET))),
    });
    for (i, t) in CONFIG.topics.iter().enumerate() {
        let topic = newsapi::parse_top_headlines(&rotated_news(i), t.title).unwrap();
        s.apply(Update {
            at,
            kind: UpdateKind::News(i, Ok(topic)),
        });
    }
    for (i, inst) in CONFIG.instruments.iter().enumerate() {
        fill_quote(&mut s, i, inst.label, inst.symbol);
    }
    s
}

/// The sample news with its articles rotated, so topics differ.
fn rotated_news(i: usize) -> Vec<u8> {
    let mut v: serde_json::Value = serde_json::from_slice(NEWS).unwrap();
    let a = v["articles"].as_array_mut().unwrap();
    let n = a.len();
    a.rotate_left(i % n);
    serde_json::to_vec(&v).unwrap()
}

/// A morning of one-minute samples, rising or falling by instrument.
fn fill_quote(s: &mut Store, i: usize, label: &str, symbol: &str) {
    let base = [430.0_f32, 501.0, 68.0, 290.0][i % 4];
    let drift = [0.004_f32, -0.003, 0.002, -0.001][i % 4];
    let open = UnixTime(NOW.0 - 50 * 60);
    let samples = 50;
    for k in 0..samples {
        let t = UnixTime(open.0 + k * 60);
        let wobble = ((k as f32) / 5.0 + i as f32).sin() * base * 0.0012;
        let price = base * (1.0 + drift * k as f32 / samples as f32) + wobble;
        let body = format!(
            r#"{{"c":{price},"d":{d},"dp":{dp},"h":{h},"l":{l},"o":{base},"pc":{base},"t":{t}}}"#,
            d = price - base,
            dp = (price - base) / base * 100.0,
            h = base * 1.004,
            l = base * 0.996,
            t = t.0
        );
        let q = finnhub::parse_quote(body.as_bytes(), label, symbol)
            .unwrap()
            .unwrap();
        s.apply(Update {
            at: t,
            kind: UpdateKind::Quote(i, Ok(q)),
        });
    }
}

/// Nothing fetched yet.
pub fn empty_store() -> Store {
    Store::new(CONFIG.topics.len(), CONFIG.instruments.len())
}

fn err(code: dt_core::ErrorCode, message: &str) -> dt_core::AppError {
    dt_core::AppError::with_message(code, message)
}

/// Every source failed before any data arrived, each in its own way.
pub fn failed_store() -> Store {
    use dt_core::ErrorCode::*;
    let mut s = empty_store();
    let at = UnixTime(NOW.0 - 120);
    s.apply(Update {
        at,
        kind: UpdateKind::Current(Err(err(
            NetDns,
            "Could not find api.openweathermap.org (DNS lookup failed).",
        ))),
    });
    s.apply(Update {
        at,
        kind: UpdateKind::Forecast(Err(err(
            NetTimeout,
            "api.openweathermap.org did not respond in time.",
        ))),
    });
    for i in 0..CONFIG.topics.len() {
        let e = err(
            ApiUnauthorized,
            "NewsAPI rejected the API key (HTTP 401): Your API key is invalid or incorrect.",
        )
        .hint("Check the key 'newsapi' in config/secrets.toml.");
        s.apply(Update {
            at,
            kind: UpdateKind::News(i, Err(e)),
        });
    }
    for (i, inst) in CONFIG.instruments.iter().enumerate() {
        let e = if i == 0 {
            err(
                ApiUnknownSymbol,
                &format!("Finnhub does not know the symbol '{}'.", inst.symbol),
            )
            .hint(format!(
                "Check the symbol for '{}' in config/user.toml.",
                inst.label
            ))
        } else {
            err(ApiRateLimited, "Finnhub rate limit reached (HTTP 429).")
        };
        s.apply(Update {
            at,
            kind: UpdateKind::Quote(i, Err(e)),
        });
    }
    s
}

/// Data from earlier, then every refresh failing (e.g. Wi-Fi down): shown
/// with stale markers.
pub fn stale_store() -> Store {
    use dt_core::ErrorCode::*;
    let mut s = full_store();
    let at = UnixTime(NOW.0 - 60);
    let off = || err(NetOffline, "No network connection.");
    s.apply(Update {
        at,
        kind: UpdateKind::Current(Err(off())),
    });
    s.apply(Update {
        at,
        kind: UpdateKind::Forecast(Err(off())),
    });
    for i in 0..CONFIG.topics.len() {
        s.apply(Update {
            at,
            kind: UpdateKind::News(i, Err(off())),
        });
    }
    for i in 0..CONFIG.instruments.len() {
        s.apply(Update {
            at,
            kind: UpdateKind::Quote(i, Err(off())),
        });
    }
    s
}
