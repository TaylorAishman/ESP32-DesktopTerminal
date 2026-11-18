//! The fetch executor against scripted HTTP responses.

use std::time::Duration;

use dt_core::api::{build_jobs, execute, Job, JobKind, Outcome};
use dt_core::platform::fake::{FakeClock, FakeHttp, FakeResponse};
use dt_core::platform::UnixTime;
use dt_core::store::UpdateKind;
use dt_core::ErrorCode;
use dt_sim::config::CONFIG;

const CURRENT: &[u8] = include_bytes!("../../dt-core/tests/fixtures/owm_current.json");
const FORECAST: &[u8] = include_bytes!("../../dt-core/tests/fixtures/owm_forecast.json");
const NEWS: &[u8] = include_bytes!("../../dt-core/tests/fixtures/newsapi_top_headlines.json");
const QUOTE: &[u8] = include_bytes!("../../dt-core/tests/fixtures/finnhub_quote.json");
const UNKNOWN: &[u8] = include_bytes!("../../dt-core/tests/fixtures/finnhub_quote_unknown.json");

fn job(kind: JobKind) -> Job {
    build_jobs(&CONFIG)
        .into_iter()
        .find(|j| j.kind == kind)
        .unwrap()
}

fn run(kind: JobKind, response: FakeResponse) -> (dt_core::api::FetchReport, FakeHttp) {
    let http = FakeHttp::new();
    http.enqueue(response);
    let clock = FakeClock::synced_at(UnixTime(1_794_924_400));
    clock.set_utc_offset(-5 * 3600);
    let mut buf = vec![0u8; 32 * 1024];
    let report = execute(&job(kind), &CONFIG, &mut http.clone(), &mut buf, &clock);
    (report, http)
}

fn error_of(report: &dt_core::api::FetchReport) -> &dt_core::AppError {
    match &report.outcome {
        Outcome::Failure { error, .. } => error,
        Outcome::Success => panic!("expected failure: {report:?}"),
    }
}

#[test]
fn every_kind_succeeds_with_its_fixture() {
    let (r, http) = run(JobKind::CurrentWeather, FakeResponse::ok(CURRENT));
    assert_eq!(r.outcome, Outcome::Success);
    assert!(matches!(r.update.kind, UpdateKind::Current(Ok(_))));
    assert_eq!(r.update.at, UnixTime(1_794_924_400));
    let req = &http.requests()[0];
    assert!(req.url.starts_with("https://api.openweathermap.org/data/2.5/weather?units=metric&lat=40.7128&lon=-74.0060&appid="));
    assert!(r.redacted_url.ends_with("appid=***"));
    assert_eq!(
        req.headers[0],
        ("User-Agent".into(), CONFIG.user_agent.into())
    );

    let (r, _) = run(JobKind::Forecast, FakeResponse::ok(FORECAST));
    match r.update.kind {
        UpdateKind::Forecast(Ok(f)) => {
            assert_eq!(f.hourly.len(), 40);
            assert_eq!(f.daily.len(), 6);
        }
        other => panic!("{other:?}"),
    }

    let (r, http) = run(JobKind::News(2), FakeResponse::ok(NEWS));
    match r.update.kind {
        UpdateKind::News(2, Ok(t)) => assert_eq!(t.title(), "Business"),
        other => panic!("{other:?}"),
    }
    let req = &http.requests()[0];
    assert_eq!(
        req.url,
        "https://newsapi.org/v2/top-headlines?pageSize=6&category=business&country=us"
    );
    assert!(req.headers.iter().any(|(k, _)| k == "X-Api-Key"));

    let (r, _) = run(JobKind::Quote(0), FakeResponse::ok(QUOTE));
    match r.update.kind {
        UpdateKind::Quote(0, Ok(q)) => {
            assert_eq!(q.label, "DIA");
            assert_eq!(q.symbol, "DIA");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn http_errors_are_classified() {
    let (r, _) = run(
        JobKind::News(0),
        FakeResponse::status(
            401,
            r#"{"status":"error","code":"apiKeyInvalid","message":"Your API key is invalid or incorrect."}"#,
        ),
    );
    let e = error_of(&r);
    assert_eq!(e.code(), ErrorCode::ApiUnauthorized);
    assert_eq!(
        e.hint_text(),
        "Check the key 'newsapi' in config/secrets.toml."
    );
    assert!(matches!(r.update.kind, UpdateKind::News(0, Err(_))));
    assert_eq!(r.status, Some(401));
}

#[test]
fn retry_after_is_passed_on() {
    let (r, _) = run(
        JobKind::Quote(1),
        FakeResponse::Ok {
            status: 429,
            body: b"{}".to_vec(),
            retry_after: Some(Duration::from_secs(30)),
        },
    );
    match &r.outcome {
        Outcome::Failure { error, retry_after } => {
            assert_eq!(error.code(), ErrorCode::ApiRateLimited);
            assert_eq!(*retry_after, Some(Duration::from_secs(30)));
        }
        Outcome::Success => panic!(),
    }
}

#[test]
fn transport_errors_name_the_host() {
    let (r, _) = run(
        JobKind::CurrentWeather,
        FakeResponse::error(ErrorCode::NetTimeout),
    );
    let e = error_of(&r);
    assert_eq!(
        e.message(),
        "api.openweathermap.org did not respond in time."
    );
    assert_eq!(r.status, None);
}

#[test]
fn truncated_response() {
    // The quote endpoint's buffer is 512 bytes.
    let (r, _) = run(JobKind::Quote(0), FakeResponse::ok(vec![b' '; 600]));
    let e = error_of(&r);
    assert_eq!(e.code(), ErrorCode::ApiResponseTooLarge);
    assert!(
        e.message().contains("exceeded 512 bytes"),
        "{}",
        e.message()
    );
}

#[test]
fn unparseable_body() {
    let (r, _) = run(
        JobKind::Forecast,
        FakeResponse::ok("<html>maintenance</html>"),
    );
    let e = error_of(&r);
    assert_eq!(e.code(), ErrorCode::ApiBadResponse);
    assert!(
        e.message()
            .starts_with("OpenWeatherMap response from 'forecast' could not be read"),
        "{}",
        e.message()
    );
}

#[test]
fn unknown_symbol() {
    let (r, _) = run(JobKind::Quote(3), FakeResponse::ok(UNKNOWN));
    let e = error_of(&r);
    assert_eq!(e.code(), ErrorCode::ApiUnknownSymbol);
    assert_eq!(e.message(), "Finnhub does not know the symbol 'VTI'.");
    assert_eq!(
        e.hint_text(),
        "Check the symbol for 'VTI' in config/user.toml."
    );
}
