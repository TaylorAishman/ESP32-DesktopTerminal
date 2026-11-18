//! Retry timing after a failed fetch (REQ-API-05).
//!
//! Exponential backoff from [`BASE_DELAY`], with ±25 % jitter so jobs that
//! failed together do not retry together, capped at the job's normal interval
//! (retrying later than the next scheduled run is pointless). A provider's
//! `Retry-After` wins. Errors that will not fix themselves (rejected key,
//! unknown symbol) go straight to the cap.

use core::time::Duration;

use crate::api_error::is_permanent;
use crate::error::AppError;

/// Delay after the first failure.
pub const BASE_DELAY: Duration = Duration::from_secs(30);

/// Consecutive-failure counter for one job.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Backoff {
    failures: u32,
}

impl Backoff {
    pub fn failures(&self) -> u32 {
        self.failures
    }

    pub fn on_success(&mut self) {
        self.failures = 0;
    }

    /// Record a failure and return how long to wait before retrying.
    ///
    /// `jitter` is a random number in `[0, 1)`; `interval` is the job's
    /// normal interval.
    pub fn on_failure(
        &mut self,
        error: &AppError,
        retry_after: Option<Duration>,
        interval: Duration,
        jitter: f64,
    ) -> Duration {
        self.failures = self.failures.saturating_add(1);
        if let Some(after) = retry_after {
            return after.max(Duration::from_secs(1));
        }
        if is_permanent(error.code()) {
            return interval;
        }
        let exp = BASE_DELAY.saturating_mul(1u32 << (self.failures - 1).min(16));
        let factor = 0.75 + 0.5 * jitter.clamp(0.0, 1.0);
        exp.mul_f64(factor)
            .min(interval)
            .max(Duration::from_secs(1))
    }
}

/// Small deterministic PRNG (xorshift64*) for jitter; no external crates and
/// reproducible in tests.
#[derive(Clone, Debug)]
pub struct Jitter(u64);

impl Jitter {
    pub fn new(seed: u64) -> Self {
        Jitter(seed | 1)
    }

    /// A number in `[0, 1)`.
    pub fn sample(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;

    const HOUR: Duration = Duration::from_secs(3600);

    fn timeout() -> AppError {
        AppError::new(ErrorCode::NetTimeout)
    }

    #[test]
    fn grows_exponentially_and_caps() {
        let mut b = Backoff::default();
        let delays: Vec<u64> = (0..10)
            .map(|_| b.on_failure(&timeout(), None, HOUR, 0.5).as_secs())
            .collect();
        assert_eq!(delays, [30, 60, 120, 240, 480, 960, 1920, 3600, 3600, 3600]);
        assert_eq!(b.failures(), 10);
    }

    #[test]
    fn jitter_stays_within_25_percent() {
        let mut lo = Backoff::default();
        let mut hi = Backoff::default();
        assert_eq!(
            lo.on_failure(&timeout(), None, HOUR, 0.0),
            Duration::from_millis(22_500)
        );
        assert_eq!(
            hi.on_failure(&timeout(), None, HOUR, 0.999_999).as_secs(),
            37
        );
    }

    #[test]
    fn retry_after_overrides() {
        let mut b = Backoff::default();
        let e = AppError::new(ErrorCode::ApiRateLimited);
        assert_eq!(
            b.on_failure(&e, Some(Duration::from_secs(7200)), HOUR, 0.5),
            Duration::from_secs(7200)
        );
    }

    #[test]
    fn permanent_errors_go_to_the_cap() {
        let mut b = Backoff::default();
        let e = AppError::new(ErrorCode::ApiUnauthorized);
        assert_eq!(b.on_failure(&e, None, HOUR, 0.5), HOUR);
    }

    #[test]
    fn success_resets() {
        let mut b = Backoff::default();
        b.on_failure(&timeout(), None, HOUR, 0.5);
        b.on_failure(&timeout(), None, HOUR, 0.5);
        b.on_success();
        assert_eq!(b.on_failure(&timeout(), None, HOUR, 0.5), BASE_DELAY);
    }

    #[test]
    fn jitter_is_uniformish() {
        let mut j = Jitter::new(42);
        let xs: Vec<f64> = (0..10_000).map(|_| j.sample()).collect();
        assert!(xs.iter().all(|&x| (0.0..1.0).contains(&x)));
        let mean = xs.iter().sum::<f64>() / xs.len() as f64;
        assert!((mean - 0.5).abs() < 0.02, "{mean}");
    }
}
