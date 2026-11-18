//! Interfaces to the hardware and platform services.
//!
//! `dt-hal` implements these traits on the device by wrapping the C HAL. Tests
//! and the simulator use the in-memory fakes in [`fake`] (feature `testing`).
//! Everything else in the application depends only on these traits, which is
//! what lets it build and run on a PC.

use core::ops::{Add, Sub};
use core::time::Duration;

use crate::error::AppError;

/// Result type for platform calls.
pub type PlatformResult<T> = Result<T, AppError>;

// ---------------------------------------------------------------------------
// Time
// ---------------------------------------------------------------------------

/// Monotonic time since boot, in milliseconds. Never goes backwards and is
/// unaffected by clock synchronisation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(u64);

impl Instant {
    pub const ZERO: Instant = Instant(0);

    pub const fn from_millis(ms: u64) -> Self {
        Instant(ms)
    }

    pub const fn as_millis(self) -> u64 {
        self.0
    }

    /// Time elapsed from `earlier` to `self`, or zero if `earlier` is later.
    pub fn saturating_duration_since(self, earlier: Instant) -> Duration {
        Duration::from_millis(self.0.saturating_sub(earlier.0))
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;
    fn add(self, d: Duration) -> Instant {
        Instant(self.0.saturating_add(d.as_millis() as u64))
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;
    fn sub(self, earlier: Instant) -> Duration {
        self.saturating_duration_since(earlier)
    }
}

/// Wall-clock time: seconds since the Unix epoch, UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnixTime(pub i64);

impl UnixTime {
    pub const fn seconds(self) -> i64 {
        self.0
    }
}

impl Add<Duration> for UnixTime {
    type Output = UnixTime;
    fn add(self, d: Duration) -> UnixTime {
        UnixTime(self.0 + d.as_secs() as i64)
    }
}

/// Monotonic and wall-clock time.
pub trait Clock: Send + Sync {
    /// Time since boot.
    fn monotonic(&self) -> Instant;

    /// Current UTC time, or `None` until the clock has been synchronised.
    fn now(&self) -> Option<UnixTime>;

    /// Offset of local time from UTC at `at`, in seconds (daylight saving
    /// included).
    fn utc_offset(&self, at: UnixTime) -> i32;
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

/// The e-ink panel.
pub trait Display {
    /// Show a full frame (see `dt-ui::FrameBuffer` for the layout). Wakes the
    /// panel, refreshes it and puts it back to sleep. Blocks for several
    /// seconds.
    fn show(&mut self, frame: &[u8]) -> PlatformResult<()>;

    /// Clear the panel to white.
    fn clear(&mut self) -> PlatformResult<()>;

    /// Put the panel into deep sleep.
    fn sleep(&mut self) -> PlatformResult<()>;
}

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------

/// One of the four front-panel buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Button {
    One,
    Two,
    Three,
    Four,
}

impl Button {
    pub const ALL: [Button; 4] = [Button::One, Button::Two, Button::Three, Button::Four];

    /// Button from its number, 1 to 4.
    pub fn from_number(n: u8) -> Option<Button> {
        match n {
            1 => Some(Button::One),
            2 => Some(Button::Two),
            3 => Some(Button::Three),
            4 => Some(Button::Four),
            _ => None,
        }
    }

    pub fn number(self) -> u8 {
        match self {
            Button::One => 1,
            Button::Two => 2,
            Button::Three => 3,
            Button::Four => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonAction {
    Press,
    LongPress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ButtonEvent {
    pub button: Button,
    pub action: ButtonAction,
}

/// The front-panel buttons.
pub trait Buttons {
    /// Wait up to `timeout` for the next button event.
    fn wait(&mut self, timeout: Duration) -> PlatformResult<Option<ButtonEvent>>;
}

// ---------------------------------------------------------------------------
// Wi-Fi
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WifiState {
    /// Not started.
    Idle,
    /// Trying to join the network.
    Connecting,
    /// Joined, with an IP address.
    Connected,
    /// Lost or failed; reconnecting automatically.
    Disconnected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WifiStatus {
    pub state: WifiState,
    /// Signal strength in dBm, when connected.
    pub rssi: Option<i8>,
    /// IPv4 address, when connected.
    pub ip: Option<[u8; 4]>,
    /// Why the last attempt failed, if it did.
    pub last_error: Option<AppError>,
}

impl WifiStatus {
    pub fn is_connected(&self) -> bool {
        self.state == WifiState::Connected
    }
}

/// Wi-Fi station. Reconnects by itself once started.
pub trait Wifi: Send + Sync {
    fn start(&self, ssid: &str, password: &str) -> PlatformResult<()>;
    fn status(&self) -> WifiStatus;
    /// Wait until connected or `timeout` passes.
    fn wait_connected(&self, timeout: Duration) -> PlatformResult<()>;
}

// ---------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------

/// An HTTPS GET request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest<'a> {
    pub url: &'a str,
    pub headers: &'a [(&'a str, &'a str)],
    pub timeout: Duration,
}

/// The response to an [`HttpRequest`]. The body borrows the caller's buffer.
#[derive(Debug, PartialEq, Eq)]
pub struct HttpResponse<'b> {
    pub status: u16,
    pub body: &'b [u8],
    /// The body did not fit in the buffer and was cut off.
    pub truncated: bool,
    /// Value of a `Retry-After` header, if present.
    pub retry_after: Option<Duration>,
}

/// HTTPS client. Only the API manager uses it.
pub trait HttpClient {
    /// Perform a GET, writing the body into `buf`. Transport failures (DNS,
    /// connect, TLS, timeout) are errors; any HTTP status is a response.
    fn get<'b>(
        &mut self,
        request: &HttpRequest<'_>,
        buf: &'b mut [u8],
    ) -> PlatformResult<HttpResponse<'b>>;
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeapStats {
    pub free_bytes: u32,
    /// Lowest free heap seen since boot.
    pub min_free_bytes: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetReason {
    PowerOn,
    /// Requested restart (including after a panic).
    Software,
    Panic,
    Watchdog,
    Brownout,
    External,
    DeepSleep,
    Unknown,
}

impl ResetReason {
    /// A reset the firmware did not ask for.
    pub fn is_abnormal(self) -> bool {
        matches!(
            self,
            ResetReason::Panic | ResetReason::Watchdog | ResetReason::Brownout
        )
    }
}

/// System services.
pub trait System: Send + Sync {
    fn heap(&self) -> HeapStats;
    fn reset_reason(&self) -> ResetReason;
    /// Restart the device. Does not return on the device.
    fn restart(&self);
    /// Register the calling thread with the task watchdog.
    fn watchdog_register(&self, timeout: Duration) -> PlatformResult<()>;
    /// Tell the watchdog the calling thread is alive.
    fn watchdog_feed(&self) -> PlatformResult<()>;
    /// Read the block of memory that survives software resets (not power
    /// loss). Returns the number of bytes read; zero if it holds nothing valid.
    fn retained_read(&self, buf: &mut [u8]) -> PlatformResult<usize>;
    fn retained_write(&self, data: &[u8]) -> PlatformResult<()>;
}

// ---------------------------------------------------------------------------
// Storage
// ---------------------------------------------------------------------------

/// Small named blobs that survive power loss.
pub trait Storage: Send + Sync {
    /// Read `key` into `buf`. `Ok(None)` if the key is absent or unreadable.
    fn get(&self, key: &str, buf: &mut [u8]) -> PlatformResult<Option<usize>>;
    fn set(&self, key: &str, data: &[u8]) -> PlatformResult<()>;
}

#[cfg(any(test, feature = "testing"))]
pub mod fake;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instant_arithmetic() {
        let a = Instant::from_millis(1_000);
        let b = a + Duration::from_secs(2);
        assert_eq!(b.as_millis(), 3_000);
        assert_eq!(b - a, Duration::from_secs(2));
        assert_eq!(a - b, Duration::ZERO);
    }

    #[test]
    fn button_numbers_round_trip() {
        for b in Button::ALL {
            assert_eq!(Button::from_number(b.number()), Some(b));
        }
        assert_eq!(Button::from_number(0), None);
        assert_eq!(Button::from_number(5), None);
    }
}
