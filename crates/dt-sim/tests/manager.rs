//! The API manager end to end with fake HTTP, clock and Wi-Fi.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dt_core::api::{ApiManager, Step};
use dt_core::platform::fake::{FakeClock, FakeHttp, FakeResponse, FakeWifi};
use dt_core::platform::UnixTime;
use dt_core::store::{PageSet, Store};
use dt_core::ErrorCode;
use dt_sim::config::CONFIG;

const CURRENT: &[u8] = include_bytes!("../../dt-core/tests/fixtures/owm_current.json");
const FORECAST: &[u8] = include_bytes!("../../dt-core/tests/fixtures/owm_forecast.json");
const NEWS: &[u8] = include_bytes!("../../dt-core/tests/fixtures/newsapi_top_headlines.json");
const QUOTE: &[u8] = include_bytes!("../../dt-core/tests/fixtures/finnhub_quote.json");

struct Rig {
    manager: ApiManager<FakeHttp>,
    http: FakeHttp,
    clock: FakeClock,
    wifi: FakeWifi,
    store: Arc<Mutex<Store>>,
    notified: Arc<Mutex<PageSet>>,
}

fn rig() -> Rig {
    let http = FakeHttp::new();
    http.route("/weather?", FakeResponse::ok(CURRENT));
    http.route("/forecast?", FakeResponse::ok(FORECAST));
    http.route("/top-headlines?", FakeResponse::ok(NEWS));
    http.route("/quote?", FakeResponse::ok(QUOTE));
    let clock = FakeClock::new();
    let wifi = FakeWifi::new();
    let store = Arc::new(Mutex::new(Store::new(
        CONFIG.topics.len(),
        CONFIG.instruments.len(),
    )));
    let notified = Arc::new(Mutex::new(PageSet::EMPTY));
    let n = notified.clone();
    let manager = ApiManager::new(
        &CONFIG,
        http.clone(),
        store.clone(),
        Arc::new(clock.clone()),
        Arc::new(wifi.clone()),
        Box::new(move |pages| {
            let mut n = n.lock().unwrap();
            *n = n.union(pages);
        }),
    );
    Rig {
        manager,
        http,
        clock,
        wifi,
        store,
        notified,
    }
}

/// Step the manager for `secs` of simulated time; fetches take 1 s.
fn run_for(r: &mut Rig, secs: u64) {
    let end = r.clock.monotonic_ms() + secs * 1000;
    while r.clock.monotonic_ms() < end {
        match r.manager.step() {
            Step::Offline | Step::WaitingForTime => r.clock.advance(Duration::from_secs(1)),
            Step::Idle(d) => r.clock.advance(d.min(Duration::from_secs(5))),
            Step::Fetched { .. } => r.clock.advance(Duration::from_secs(1)),
        }
    }
}

trait Ms {
    fn monotonic_ms(&self) -> u64;
}
impl Ms for FakeClock {
    fn monotonic_ms(&self) -> u64 {
        use dt_core::platform::Clock;
        self.monotonic().as_millis()
    }
}

#[test]
fn waits_for_wifi_and_time_then_fills_the_store() {
    let mut r = rig();
    assert_eq!(r.manager.step(), Step::Offline);
    r.wifi.set_connected();
    assert_eq!(r.manager.step(), Step::WaitingForTime);
    assert_eq!(r.http.request_count(), 0);
    r.clock.sync(UnixTime(1_794_924_000));

    run_for(&mut r, 3600);
    let s = r.store.lock().unwrap();
    assert!(s.current.data.is_some());
    assert_eq!(s.forecast.data.as_ref().unwrap().daily.len(), 6);
    assert!(s.news.iter().all(|t| t.data.is_some()));
    assert!(s.quotes.iter().all(|q| q.quote.data.is_some()));
    assert_eq!(*r.notified.lock().unwrap(), PageSet::ALL);
}

#[test]
fn simulated_day_stays_within_every_limit() {
    let mut r = rig();
    r.wifi.set_connected();
    r.clock.sync(UnixTime(1_794_873_600));
    run_for(&mut r, 86_400);

    let mut per_host: HashMap<&str, usize> = HashMap::new();
    for req in r.http.requests() {
        let host = ["openweathermap", "newsapi", "finnhub"]
            .into_iter()
            .find(|h| req.url.contains(h))
            .unwrap();
        *per_host.entry(host).or_default() += 1;
    }
    assert!(per_host["newsapi"] <= 100, "{per_host:?}");
    assert!(per_host["newsapi"] >= 88, "{per_host:?}");
    // 144 current + 48 forecast.
    assert!(
        (190..=194).contains(&per_host["openweathermap"]),
        "{per_host:?}"
    );
    // Quotes every minute: 4 × 1440 (market hours are not modelled here).
    assert!((5750..=5770).contains(&per_host["finnhub"]), "{per_host:?}");
}

#[test]
fn failures_are_recorded_and_others_continue() {
    let mut r = rig();
    // NewsAPI rejects the key; everything else works.
    let http = FakeHttp::new();
    http.route("/top-headlines?", FakeResponse::status(401, "{}"));
    http.route("/weather?", FakeResponse::ok(CURRENT));
    http.route("/forecast?", FakeResponse::ok(FORECAST));
    http.route("/quote?", FakeResponse::ok(QUOTE));
    r.manager = ApiManager::new(
        &CONFIG,
        http,
        r.store.clone(),
        Arc::new(r.clock.clone()),
        Arc::new(r.wifi.clone()),
        Box::new(|_| {}),
    );
    r.wifi.set_connected();
    r.clock.sync(UnixTime(1_794_924_000));
    run_for(&mut r, 3600);
    let s = r.store.lock().unwrap();
    assert!(s
        .news
        .iter()
        .all(|t| t.data.is_none()
            && t.last_error.as_ref().unwrap().code() == ErrorCode::ApiUnauthorized));
    assert!(s.current.data.is_some() && s.quotes[0].quote.data.is_some());
    // A rejected key backs off to the full interval: each topic tried once
    // or twice in the hour, not hammered.
    assert!(r.manager.scheduler().failures(4) <= 2);
}
