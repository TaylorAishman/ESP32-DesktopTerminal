//! In-memory implementations of the platform traits for tests and the
//! simulator. Each fake is a cheap handle: clones share state, so a test can
//! keep one handle to inspect or drive the fake while the code under test owns
//! another.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use super::*;
use crate::error::{AppError, ErrorCode};

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    // A panic in another test thread must not cascade into every later call.
    m.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------

#[derive(Debug)]
struct ClockState {
    monotonic: Instant,
    /// Wall-clock time at `monotonic == 0`, if synchronised.
    epoch_offset: Option<i64>,
    utc_offset: i32,
}

/// A clock that only moves when told to.
#[derive(Clone, Debug)]
pub struct FakeClock(Arc<Mutex<ClockState>>);

impl FakeClock {
    /// Unsynchronised clock at boot.
    pub fn new() -> Self {
        FakeClock(Arc::new(Mutex::new(ClockState {
            monotonic: Instant::ZERO,
            epoch_offset: None,
            utc_offset: 0,
        })))
    }

    /// Clock synchronised so that `now()` currently returns `at`.
    pub fn synced_at(at: UnixTime) -> Self {
        let c = Self::new();
        c.sync(at);
        c
    }

    /// Simulate time synchronisation: `now()` returns `at` from this moment.
    pub fn sync(&self, at: UnixTime) {
        let mut s = lock(&self.0);
        s.epoch_offset = Some(at.0 - (s.monotonic.as_millis() / 1000) as i64);
    }

    pub fn set_utc_offset(&self, seconds: i32) {
        lock(&self.0).utc_offset = seconds;
    }

    pub fn advance(&self, d: Duration) {
        let mut s = lock(&self.0);
        s.monotonic = s.monotonic + d;
    }
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for FakeClock {
    fn monotonic(&self) -> Instant {
        lock(&self.0).monotonic
    }

    fn now(&self) -> Option<UnixTime> {
        let s = lock(&self.0);
        s.epoch_offset
            .map(|e| UnixTime(e + (s.monotonic.as_millis() / 1000) as i64))
    }

    fn utc_offset(&self, _at: UnixTime) -> i32 {
        lock(&self.0).utc_offset
    }
}

// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct DisplayState {
    frames: Vec<Vec<u8>>,
    clears: usize,
    sleeps: usize,
    fail_next: Option<AppError>,
}

/// Records every frame shown.
#[derive(Clone, Debug, Default)]
pub struct FakeDisplay(Arc<Mutex<DisplayState>>);

impl FakeDisplay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of refreshes performed.
    pub fn show_count(&self) -> usize {
        lock(&self.0).frames.len()
    }

    pub fn last_frame(&self) -> Option<Vec<u8>> {
        lock(&self.0).frames.last().cloned()
    }

    pub fn clear_count(&self) -> usize {
        lock(&self.0).clears
    }

    pub fn sleep_count(&self) -> usize {
        lock(&self.0).sleeps
    }

    /// Make the next `show` fail with `error`.
    pub fn fail_next(&self, error: AppError) {
        lock(&self.0).fail_next = Some(error);
    }
}

impl Display for FakeDisplay {
    fn show(&mut self, frame: &[u8]) -> PlatformResult<()> {
        let mut s = lock(&self.0);
        if let Some(e) = s.fail_next.take() {
            return Err(e);
        }
        s.frames.push(frame.to_vec());
        Ok(())
    }

    fn clear(&mut self) -> PlatformResult<()> {
        lock(&self.0).clears += 1;
        Ok(())
    }

    fn sleep(&mut self) -> PlatformResult<()> {
        lock(&self.0).sleeps += 1;
        Ok(())
    }
}

// ---------------------------------------------------------------------------

/// Buttons fed from a queue.
#[derive(Clone, Debug, Default)]
pub struct FakeButtons(Arc<Mutex<VecDeque<ButtonEvent>>>);

impl FakeButtons {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn press(&self, button: Button) {
        self.push(ButtonEvent {
            button,
            action: ButtonAction::Press,
        });
    }

    pub fn push(&self, event: ButtonEvent) {
        lock(&self.0).push_back(event);
    }
}

impl Buttons for FakeButtons {
    /// Returns the next queued event immediately, or `None` without waiting.
    fn wait(&mut self, _timeout: Duration) -> PlatformResult<Option<ButtonEvent>> {
        Ok(lock(&self.0).pop_front())
    }
}

// ---------------------------------------------------------------------------

/// Current status and the credentials passed to `start`.
type WifiInner = (WifiStatus, Option<(String, String)>);

/// Wi-Fi whose status the test sets directly.
#[derive(Clone, Debug)]
pub struct FakeWifi(Arc<Mutex<WifiInner>>);

impl FakeWifi {
    pub fn new() -> Self {
        FakeWifi(Arc::new(Mutex::new((
            WifiStatus {
                state: WifiState::Idle,
                rssi: None,
                ip: None,
                last_error: None,
            },
            None,
        ))))
    }

    pub fn set_connected(&self) {
        let mut s = lock(&self.0);
        s.0 = WifiStatus {
            state: WifiState::Connected,
            rssi: Some(-55),
            ip: Some([192, 168, 1, 50]),
            last_error: None,
        };
    }

    pub fn set_disconnected(&self, error: Option<AppError>) {
        let mut s = lock(&self.0);
        s.0 = WifiStatus {
            state: WifiState::Disconnected,
            rssi: None,
            ip: None,
            last_error: error,
        };
    }

    /// Credentials passed to `start`, if it was called.
    pub fn started_with(&self) -> Option<(String, String)> {
        lock(&self.0).1.clone()
    }
}

impl Default for FakeWifi {
    fn default() -> Self {
        Self::new()
    }
}

impl Wifi for FakeWifi {
    fn start(&self, ssid: &str, password: &str) -> PlatformResult<()> {
        let mut s = lock(&self.0);
        s.1 = Some((ssid.to_owned(), password.to_owned()));
        if s.0.state == WifiState::Idle {
            s.0.state = WifiState::Connecting;
        }
        Ok(())
    }

    fn status(&self) -> WifiStatus {
        lock(&self.0).0.clone()
    }

    fn wait_connected(&self, _timeout: Duration) -> PlatformResult<()> {
        let s = lock(&self.0);
        if s.0.is_connected() {
            Ok(())
        } else {
            Err(s
                .0
                .last_error
                .clone()
                .unwrap_or_else(|| AppError::new(ErrorCode::NetOffline)))
        }
    }
}

// ---------------------------------------------------------------------------

/// A scripted HTTP outcome.
#[derive(Clone, Debug)]
pub enum FakeResponse {
    Ok {
        status: u16,
        body: Vec<u8>,
        retry_after: Option<Duration>,
    },
    Err(AppError),
}

impl FakeResponse {
    pub fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self::status(200, body)
    }

    pub fn status(status: u16, body: impl Into<Vec<u8>>) -> Self {
        FakeResponse::Ok {
            status,
            body: body.into(),
            retry_after: None,
        }
    }

    pub fn error(code: ErrorCode) -> Self {
        FakeResponse::Err(AppError::new(code))
    }
}

/// A request the fake received.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Default)]
struct HttpState {
    /// One-shot responses, used first, in order.
    queue: VecDeque<FakeResponse>,
    /// Standing responses: the first whose pattern occurs in the URL wins.
    routes: Vec<(String, FakeResponse)>,
    log: Vec<RecordedRequest>,
}

/// HTTP client with scripted responses and a request log. A request matching
/// no queued response and no route fails with `NetConnect`.
#[derive(Clone, Debug, Default)]
pub struct FakeHttp(Arc<Mutex<HttpState>>);

impl FakeHttp {
    pub fn new() -> Self {
        Self::default()
    }

    /// Answer every request whose URL contains `pattern` with `response`.
    pub fn route(&self, pattern: &str, response: FakeResponse) {
        lock(&self.0).routes.push((pattern.to_owned(), response));
    }

    /// Answer the next request (of any URL) with `response`.
    pub fn enqueue(&self, response: FakeResponse) {
        lock(&self.0).queue.push_back(response);
    }

    pub fn requests(&self) -> Vec<RecordedRequest> {
        lock(&self.0).log.clone()
    }

    pub fn request_count(&self) -> usize {
        lock(&self.0).log.len()
    }
}

impl HttpClient for FakeHttp {
    fn get<'b>(
        &mut self,
        request: &HttpRequest<'_>,
        buf: &'b mut [u8],
    ) -> PlatformResult<HttpResponse<'b>> {
        let mut s = lock(&self.0);
        s.log.push(RecordedRequest {
            url: request.url.to_owned(),
            headers: request
                .headers
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
        });
        let response = match s.queue.pop_front() {
            Some(r) => r,
            None => s
                .routes
                .iter()
                .find(|(p, _)| request.url.contains(p.as_str()))
                .map(|(_, r)| r.clone())
                .unwrap_or_else(|| FakeResponse::error(ErrorCode::NetConnect)),
        };
        match response {
            FakeResponse::Err(e) => Err(e),
            FakeResponse::Ok {
                status,
                body,
                retry_after,
            } => {
                let n = body.len().min(buf.len());
                buf[..n].copy_from_slice(&body[..n]);
                Ok(HttpResponse {
                    status,
                    body: &buf[..n],
                    truncated: n < body.len(),
                    retry_after,
                })
            }
        }
    }
}

// ---------------------------------------------------------------------------

#[derive(Debug)]
struct SystemState {
    heap: HeapStats,
    reset_reason: ResetReason,
    restarts: usize,
    retained: Vec<u8>,
    watchdog_feeds: usize,
}

/// System services with settable heap and reset reason. `restart` only counts.
#[derive(Clone, Debug)]
pub struct FakeSystem(Arc<Mutex<SystemState>>);

impl FakeSystem {
    pub fn new() -> Self {
        FakeSystem(Arc::new(Mutex::new(SystemState {
            heap: HeapStats {
                free_bytes: 120_000,
                min_free_bytes: 80_000,
            },
            reset_reason: ResetReason::PowerOn,
            restarts: 0,
            retained: Vec::new(),
            watchdog_feeds: 0,
        })))
    }

    pub fn set_reset_reason(&self, reason: ResetReason) {
        lock(&self.0).reset_reason = reason;
    }

    pub fn set_heap(&self, heap: HeapStats) {
        lock(&self.0).heap = heap;
    }

    pub fn restart_count(&self) -> usize {
        lock(&self.0).restarts
    }

    pub fn watchdog_feeds(&self) -> usize {
        lock(&self.0).watchdog_feeds
    }
}

impl Default for FakeSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for FakeSystem {
    fn heap(&self) -> HeapStats {
        lock(&self.0).heap
    }

    fn reset_reason(&self) -> ResetReason {
        lock(&self.0).reset_reason
    }

    fn restart(&self) {
        lock(&self.0).restarts += 1;
    }

    fn watchdog_register(&self, _timeout: Duration) -> PlatformResult<()> {
        Ok(())
    }

    fn watchdog_feed(&self) -> PlatformResult<()> {
        lock(&self.0).watchdog_feeds += 1;
        Ok(())
    }

    fn retained_read(&self, buf: &mut [u8]) -> PlatformResult<usize> {
        let s = lock(&self.0);
        let n = s.retained.len().min(buf.len());
        buf[..n].copy_from_slice(&s.retained[..n]);
        Ok(n)
    }

    fn retained_write(&self, data: &[u8]) -> PlatformResult<()> {
        lock(&self.0).retained = data.to_vec();
        Ok(())
    }
}

// ---------------------------------------------------------------------------

/// Key-value storage in memory.
#[derive(Clone, Debug, Default)]
pub struct FakeStorage(Arc<Mutex<HashMap<String, Vec<u8>>>>);

impl FakeStorage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Raw access for tests, e.g. to corrupt a stored value.
    pub fn raw(&self, key: &str) -> Option<Vec<u8>> {
        lock(&self.0).get(key).cloned()
    }

    pub fn set_raw(&self, key: &str, data: &[u8]) {
        lock(&self.0).insert(key.to_owned(), data.to_vec());
    }
}

impl Storage for FakeStorage {
    fn get(&self, key: &str, buf: &mut [u8]) -> PlatformResult<Option<usize>> {
        let s = lock(&self.0);
        Ok(s.get(key).and_then(|v| {
            (v.len() <= buf.len()).then(|| {
                buf[..v.len()].copy_from_slice(v);
                v.len()
            })
        }))
    }

    fn set(&self, key: &str, data: &[u8]) -> PlatformResult<()> {
        lock(&self.0).insert(key.to_owned(), data.to_vec());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_advances_and_syncs() {
        let c = FakeClock::new();
        assert_eq!(c.now(), None);
        c.advance(Duration::from_secs(5));
        c.sync(UnixTime(1_000_000));
        c.advance(Duration::from_secs(10));
        assert_eq!(c.monotonic().as_millis(), 15_000);
        assert_eq!(c.now(), Some(UnixTime(1_000_010)));
    }

    #[test]
    fn http_queue_then_routes_then_failure() {
        let shared = FakeHttp::new();
        shared.route("weather", FakeResponse::ok("route"));
        shared.enqueue(FakeResponse::status(500, "queued"));
        let mut http = shared.clone();
        let mut buf = [0u8; 16];
        let req = |url| HttpRequest {
            url,
            headers: &[("User-Agent", "dt")],
            timeout: Duration::from_secs(1),
        };

        let r = http.get(&req("https://x/weather"), &mut buf).unwrap();
        assert_eq!((r.status, r.body), (500, &b"queued"[..]));
        let r = http.get(&req("https://x/weather"), &mut buf).unwrap();
        assert_eq!((r.status, r.body), (200, &b"route"[..]));
        let e = http.get(&req("https://x/news"), &mut buf).unwrap_err();
        assert_eq!(e.code(), ErrorCode::NetConnect);
        assert_eq!(shared.request_count(), 3);
        assert_eq!(shared.requests()[0].headers[0].0, "User-Agent");
    }

    #[test]
    fn http_truncates_into_small_buffer() {
        let shared = FakeHttp::new();
        shared.enqueue(FakeResponse::ok("0123456789"));
        let mut http = shared.clone();
        let mut buf = [0u8; 4];
        let r = http
            .get(
                &HttpRequest {
                    url: "u",
                    headers: &[],
                    timeout: Duration::ZERO,
                },
                &mut buf,
            )
            .unwrap();
        assert!(r.truncated);
        assert_eq!(r.body, b"0123");
    }

    #[test]
    fn storage_round_trip_and_small_buffer() {
        let s = FakeStorage::new();
        s.set("k", b"hello").unwrap();
        let mut buf = [0u8; 8];
        assert_eq!(s.get("k", &mut buf).unwrap(), Some(5));
        assert_eq!(&buf[..5], b"hello");
        assert_eq!(s.get("k", &mut [0u8; 2]).unwrap(), None);
        assert_eq!(s.get("missing", &mut buf).unwrap(), None);
    }
}
