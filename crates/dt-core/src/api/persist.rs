//! Keeping rate-limiter counts across restarts and power cycles
//! (REQ-API-10), so rebooting cannot reset a daily or monthly quota.

use super::rate_limit::{Period, RateLimiter, WindowState};
use crate::platform::{Storage, UnixTime};

/// Largest saved record.
const MAX_RECORD: usize = 512;

/// Storage key for a provider. NVS keys are at most 15 bytes.
pub fn key_for(provider_id: &str) -> String {
    let short: String = provider_id.chars().take(12).collect();
    format!("rl_{short}")
}

/// Whether a limiter needs saving: only windows longer than a restart matter.
pub fn worth_persisting(limiter: &RateLimiter) -> bool {
    limiter
        .state()
        .iter()
        .any(|w| matches!(w.limit.per, Period::Day | Period::Month))
}

/// Save a provider's counts. Failures are logged, not fatal: the schedule
/// alone keeps calls under the limits.
pub fn save(storage: &dyn Storage, provider_id: &str, limiter: &RateLimiter) {
    match serde_json::to_vec(limiter.state()) {
        Ok(bytes) => {
            if let Err(e) = storage.set(&key_for(provider_id), &bytes) {
                log::warn!("could not save call counts for {provider_id}: {e}");
            }
        }
        Err(e) => log::warn!("could not encode call counts for {provider_id}: {e}"),
    }
}

/// What restoring found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Restored {
    /// Saved counts were loaded.
    Loaded,
    /// Nothing saved (first boot): counts start at zero.
    Fresh,
    /// The record was unreadable: counts were set conservatively.
    Conservative,
}

/// Restore a provider's counts. `now` is the wall-clock time if known; it is
/// only used to size the conservative estimate for a corrupt record.
pub fn restore(
    storage: &dyn Storage,
    provider_id: &str,
    limiter: &mut RateLimiter,
    now: Option<UnixTime>,
) -> Restored {
    let mut buf = [0u8; MAX_RECORD];
    match storage.get(&key_for(provider_id), &mut buf) {
        Ok(None) => Restored::Fresh,
        Ok(Some(n)) => match serde_json::from_slice::<Vec<WindowState>>(&buf[..n]) {
            Ok(saved) => {
                limiter.restore(&saved);
                Restored::Loaded
            }
            Err(_) => {
                log::warn!("saved call counts for {provider_id} are unreadable; assuming the quota is partly used");
                assume_used(limiter, now);
                Restored::Conservative
            }
        },
        Err(e) => {
            log::warn!("could not read call counts for {provider_id}: {e}; assuming the quota is partly used");
            assume_used(limiter, now);
            Restored::Conservative
        }
    }
}

/// Assume each window is already used in proportion to how much of it has
/// passed, i.e. as much as an evenly spread schedule would have used. With no
/// clock, assume half.
fn assume_used(limiter: &mut RateLimiter, now: Option<UnixTime>) {
    let estimated: Vec<WindowState> = limiter
        .state()
        .iter()
        .map(|w| {
            let (start, fraction) = match now {
                Some(t) => {
                    let start = w.limit.per.window_start(t);
                    let end = w.limit.per.window_end(t);
                    let f = (t.seconds() - start.seconds()) as f64
                        / (end.seconds() - start.seconds()) as f64;
                    (start.seconds(), f)
                }
                None => (i64::MIN, 0.5),
            };
            WindowState {
                limit: w.limit,
                window_start: start,
                count: (f64::from(w.limit.calls) * fraction).ceil() as u32,
            }
        })
        .collect();
    limiter.restore(&estimated);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Limit;
    use crate::platform::fake::FakeStorage;

    const DAY: [Limit; 1] = [Limit {
        calls: 100,
        per: Period::Day,
    }];
    // 2026-11-17 12:00 UTC: half the day gone.
    const NOON: UnixTime = UnixTime(1_794_873_600 + 43_200);

    #[test]
    fn restart_mid_day_keeps_the_count() {
        let storage = FakeStorage::new();
        let mut before = RateLimiter::new(&DAY);
        for _ in 0..40 {
            before.try_acquire(NOON).unwrap();
        }
        save(&storage, "newsapi", &before);

        let mut after = RateLimiter::new(&DAY);
        assert_eq!(
            restore(&storage, "newsapi", &mut after, None),
            Restored::Loaded
        );
        assert_eq!(after.usage(NOON)[0].1, 40);
        // 60 more allowed today, then refused.
        for _ in 0..60 {
            after.try_acquire(NOON).unwrap();
        }
        assert!(after.try_acquire(NOON).is_err());
    }

    #[test]
    fn first_boot_starts_at_zero() {
        let mut l = RateLimiter::new(&DAY);
        assert_eq!(
            restore(&FakeStorage::new(), "newsapi", &mut l, Some(NOON)),
            Restored::Fresh
        );
        assert_eq!(l.usage(NOON)[0].1, 0);
    }

    #[test]
    fn corrupt_record_is_conservative() {
        let storage = FakeStorage::new();
        storage.set_raw(&key_for("newsapi"), b"\xff\x00garbage");
        let mut l = RateLimiter::new(&DAY);
        assert_eq!(
            restore(&storage, "newsapi", &mut l, Some(NOON)),
            Restored::Conservative
        );
        assert_eq!(l.usage(NOON)[0].1, 50);
    }

    #[test]
    fn keys_fit_nvs() {
        assert_eq!(key_for("openweathermap"), "rl_openweatherm");
        assert!(key_for("openweathermap").len() <= 15);
    }

    #[test]
    fn only_long_windows_are_persisted() {
        assert!(worth_persisting(&RateLimiter::new(&DAY)));
        assert!(!worth_persisting(&RateLimiter::new(&[Limit {
            calls: 60,
            per: Period::Minute
        }])));
    }
}
