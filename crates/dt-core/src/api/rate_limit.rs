//! Hard per-provider call limits.
//!
//! The scheduler spaces calls evenly below every limit, so in normal operation
//! this limiter never refuses anything. It is the guard behind the schedule:
//! retries, manual refreshes and restart loops all have to get past it.
//!
//! Windows are fixed and aligned to UTC calendar boundaries (the minute, the
//! hour, the day, the calendar month), matching how providers reset their
//! quotas. The limiter works on wall-clock time so that its state stays valid
//! across restarts; API calls are only made once the clock is synchronised,
//! which TLS certificate checks require anyway.

use core::time::Duration;

use serde::{Deserialize, Serialize};

use crate::platform::UnixTime;
use crate::time::{Date, LocalDateTime};

/// Length of a limit window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    Minute,
    Hour,
    Day,
    Month,
}

impl Period {
    /// Start of the window containing `t`.
    pub fn window_start(self, t: UnixTime) -> UnixTime {
        let s = t.seconds();
        match self {
            Period::Minute => UnixTime(s - s.rem_euclid(60)),
            Period::Hour => UnixTime(s - s.rem_euclid(3600)),
            Period::Day => UnixTime(s - s.rem_euclid(86_400)),
            Period::Month => {
                let d = LocalDateTime::from_unix(t, 0).date;
                let first = Date { day: 1, ..d };
                UnixTime(first.days_since_epoch() * 86_400)
            }
        }
    }

    /// End of the window containing `t` (start of the next one).
    pub fn window_end(self, t: UnixTime) -> UnixTime {
        let start = self.window_start(t).seconds();
        match self {
            Period::Minute => UnixTime(start + 60),
            Period::Hour => UnixTime(start + 3600),
            Period::Day => UnixTime(start + 86_400),
            Period::Month => {
                let d = LocalDateTime::from_unix(UnixTime(start), 0).date;
                let (year, month) = if d.month == 12 {
                    (d.year + 1, 1)
                } else {
                    (d.year, d.month + 1)
                };
                UnixTime(
                    Date {
                        year,
                        month,
                        day: 1,
                    }
                    .days_since_epoch()
                        * 86_400,
                )
            }
        }
    }

    /// Nominal length, used for rate calculations. A month counts as 31
    /// days, so a monthly budget is spread conservatively.
    pub fn nominal(self) -> Duration {
        Duration::from_secs(match self {
            Period::Minute => 60,
            Period::Hour => 3600,
            Period::Day => 86_400,
            Period::Month => 31 * 86_400,
        })
    }
}

/// "At most `calls` per `period`".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Limit {
    pub calls: u32,
    pub per: Period,
}

/// Count within one window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub limit: Limit,
    /// Start of the window `count` belongs to.
    pub window_start: i64,
    pub count: u32,
}

/// Call counter for one provider, enforcing all of its limits at once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimiter {
    windows: Vec<WindowState>,
}

impl RateLimiter {
    pub fn new(limits: &[Limit]) -> Self {
        RateLimiter {
            windows: limits
                .iter()
                .map(|&limit| WindowState {
                    limit,
                    window_start: i64::MIN,
                    count: 0,
                })
                .collect(),
        }
    }

    /// Record one call if every limit allows it. Otherwise return how long to
    /// wait until all of them would.
    pub fn try_acquire(&mut self, now: UnixTime) -> Result<(), Duration> {
        self.roll(now);
        let wait = self.wait_time(now);
        if wait > Duration::ZERO {
            return Err(wait);
        }
        for w in &mut self.windows {
            w.count += 1;
        }
        Ok(())
    }

    /// How long until a call would be allowed; zero if it is allowed now.
    pub fn next_available(&self, now: UnixTime) -> Duration {
        let mut copy = self.clone();
        copy.roll(now);
        copy.wait_time(now)
    }

    /// Calls made in the current window of each limit, as (limit, used).
    pub fn usage(&self, now: UnixTime) -> Vec<(Limit, u32)> {
        let mut copy = self.clone();
        copy.roll(now);
        copy.windows.iter().map(|w| (w.limit, w.count)).collect()
    }

    /// Saved state, for persisting across restarts.
    pub fn state(&self) -> &[WindowState] {
        &self.windows
    }

    /// Restore counts saved by [`state`](Self::state). Counts for limits that
    /// no longer exist are dropped; new limits start from zero.
    pub fn restore(&mut self, saved: &[WindowState]) {
        for w in &mut self.windows {
            if let Some(s) = saved.iter().find(|s| s.limit.per == w.limit.per) {
                w.window_start = s.window_start;
                w.count = s.count;
            }
        }
    }

    fn roll(&mut self, now: UnixTime) {
        for w in &mut self.windows {
            let start = w.limit.per.window_start(now).seconds();
            if start != w.window_start {
                w.window_start = start;
                w.count = 0;
            }
        }
    }

    fn wait_time(&self, now: UnixTime) -> Duration {
        self.windows
            .iter()
            .filter(|w| w.count >= w.limit.calls)
            .map(|w| {
                let end = w.limit.per.window_end(now).seconds();
                Duration::from_secs((end - now.seconds()).max(1) as u64)
            })
            .max()
            .unwrap_or(Duration::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-11-17 14:05:09 UTC
    const T: i64 = 1_794_924_309;

    fn lim(calls: u32, per: Period) -> Limit {
        Limit { calls, per }
    }

    #[test]
    fn window_boundaries() {
        let t = UnixTime(T);
        assert_eq!(Period::Minute.window_start(t).0, T - 9);
        assert_eq!(Period::Hour.window_end(t).0, T - 5 * 60 - 9 + 3600);
        assert_eq!(Period::Day.window_start(t).0 % 86_400, 0);
        // November 2026 starts 2026-11-01 and ends 2026-12-01.
        let nov1 = Date {
            year: 2026,
            month: 11,
            day: 1,
        }
        .days_since_epoch()
            * 86_400;
        let dec1 = Date {
            year: 2026,
            month: 12,
            day: 1,
        }
        .days_since_epoch()
            * 86_400;
        assert_eq!(Period::Month.window_start(t).0, nov1);
        assert_eq!(Period::Month.window_end(t).0, dec1);
        // December rolls into January.
        let jan1 = Date {
            year: 2027,
            month: 1,
            day: 1,
        }
        .days_since_epoch()
            * 86_400;
        assert_eq!(Period::Month.window_end(UnixTime(dec1 + 5)).0, jan1);
    }

    #[test]
    fn never_exceeds_per_minute_limit() {
        let mut r = RateLimiter::new(&[lim(3, Period::Minute)]);
        let start = Period::Minute.window_start(UnixTime(T)).0;
        let mut granted_per_minute = std::collections::HashMap::<i64, u32>::new();
        // Try every second for ten minutes.
        for s in 0..600 {
            let now = UnixTime(start + s);
            if r.try_acquire(now).is_ok() {
                *granted_per_minute.entry(s / 60).or_default() += 1;
            }
        }
        assert_eq!(granted_per_minute.len(), 10);
        assert!(granted_per_minute.values().all(|&n| n == 3));
    }

    #[test]
    fn wait_points_to_window_end() {
        let mut r = RateLimiter::new(&[lim(1, Period::Day)]);
        let now = UnixTime(T);
        assert!(r.try_acquire(now).is_ok());
        let end = Period::Day.window_end(now).0;
        assert_eq!(
            r.try_acquire(now),
            Err(Duration::from_secs((end - T) as u64))
        );
        assert_eq!(r.next_available(UnixTime(end)), Duration::ZERO);
        assert!(r.try_acquire(UnixTime(end)).is_ok());
    }

    #[test]
    fn all_limits_apply_together() {
        let mut r = RateLimiter::new(&[lim(60, Period::Minute), lim(100, Period::Day)]);
        let day = Period::Day.window_start(UnixTime(T)).0;
        let mut granted = 0;
        for s in 0..86_400 {
            if r.try_acquire(UnixTime(day + s)).is_ok() {
                granted += 1;
            }
        }
        assert_eq!(granted, 100);
        let usage = r.usage(UnixTime(day + 86_399));
        assert_eq!(usage[1], (lim(100, Period::Day), 100));
    }

    #[test]
    fn state_round_trips_through_json() {
        let limits = [lim(100, Period::Day)];
        let mut r = RateLimiter::new(&limits);
        for _ in 0..40 {
            r.try_acquire(UnixTime(T)).unwrap();
        }
        let json = serde_json::to_string(r.state()).unwrap();
        let saved: Vec<WindowState> = serde_json::from_str(&json).unwrap();
        let mut restored = RateLimiter::new(&limits);
        restored.restore(&saved);
        assert_eq!(restored.usage(UnixTime(T + 60))[0].1, 40);
        // Next day the count resets.
        assert_eq!(restored.usage(UnixTime(T + 86_400))[0].1, 0);
    }
}
