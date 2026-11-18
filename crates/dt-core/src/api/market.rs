//! US equity market hours, for slowing quote polling when prices cannot move.
//!
//! Regular session: Monday to Friday, 09:30 to 16:00 New York time. Exchange
//! holidays are not modelled (Requirements §14); on a holiday quotes are
//! simply polled at the normal rate.

use crate::platform::UnixTime;
use crate::time::{Date, LocalDateTime, Weekday};

/// Polling starts this long before the open and continues this long after
/// the close, to catch the opening and closing prints.
const MARGIN_MINUTES: u16 = 5;
const OPEN_MINUTE: u16 = 9 * 60 + 30;
const CLOSE_MINUTE: u16 = 16 * 60;

/// New York's offset from UTC at `t`: -4 h during daylight saving time
/// (second Sunday in March, 02:00 local, to first Sunday in November, 02:00
/// local), otherwise -5 h.
pub fn new_york_offset(t: UnixTime) -> i32 {
    const EST: i32 = -5 * 3600;
    const EDT: i32 = -4 * 3600;
    let year = LocalDateTime::from_unix(t, EST).date.year;
    // 02:00 EST on the second Sunday of March = 07:00 UTC.
    let start = nth_sunday(year, 3, 2).days_since_epoch() * 86_400 + 7 * 3600;
    // 02:00 EDT on the first Sunday of November = 06:00 UTC.
    let end = nth_sunday(year, 11, 1).days_since_epoch() * 86_400 + 6 * 3600;
    if (start..end).contains(&t.seconds()) {
        EDT
    } else {
        EST
    }
}

fn nth_sunday(year: i32, month: u8, n: u8) -> Date {
    let first = Date {
        year,
        month,
        day: 1,
    };
    let mut day = 1;
    let mut d = first;
    while d.weekday() != Weekday::Sun {
        day += 1;
        d = Date { day, ..first };
    }
    Date {
        day: day + 7 * (n - 1),
        ..first
    }
}

/// Whether US equities may be trading at `t` (with a small margin).
pub fn is_us_market_open(t: UnixTime) -> bool {
    let local = LocalDateTime::from_unix(t, new_york_offset(t));
    if local.date.weekday().is_weekend() {
        return false;
    }
    let m = local.minute_of_day();
    (OPEN_MINUTE - MARGIN_MINUTES..CLOSE_MINUTE + MARGIN_MINUTES).contains(&m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(y: i32, mo: u8, d: u8, h: i64, mi: i64) -> UnixTime {
        UnixTime(
            Date {
                year: y,
                month: mo,
                day: d,
            }
            .days_since_epoch()
                * 86_400
                + h * 3600
                + mi * 60,
        )
    }

    #[test]
    fn dst_boundaries_2026() {
        // DST 2026: 8 March to 1 November.
        assert_eq!(new_york_offset(utc(2026, 3, 8, 6, 59)), -5 * 3600);
        assert_eq!(new_york_offset(utc(2026, 3, 8, 7, 0)), -4 * 3600);
        assert_eq!(new_york_offset(utc(2026, 11, 1, 5, 59)), -4 * 3600);
        assert_eq!(new_york_offset(utc(2026, 11, 1, 6, 0)), -5 * 3600);
        assert_eq!(new_york_offset(utc(2026, 7, 4, 12, 0)), -4 * 3600);
        assert_eq!(new_york_offset(utc(2026, 1, 15, 12, 0)), -5 * 3600);
    }

    #[test]
    fn session_in_winter() {
        // Tue 17 Nov 2026, EST: 09:30 = 14:30 UTC, 16:00 = 21:00 UTC.
        assert!(!is_us_market_open(utc(2026, 11, 17, 14, 24)));
        assert!(is_us_market_open(utc(2026, 11, 17, 14, 25)));
        assert!(is_us_market_open(utc(2026, 11, 17, 21, 4)));
        assert!(!is_us_market_open(utc(2026, 11, 17, 21, 5)));
    }

    #[test]
    fn session_in_summer() {
        // Wed 15 Jul 2026, EDT: 09:30 = 13:30 UTC.
        assert!(!is_us_market_open(utc(2026, 7, 15, 13, 24)));
        assert!(is_us_market_open(utc(2026, 7, 15, 13, 30)));
        assert!(!is_us_market_open(utc(2026, 7, 15, 20, 10)));
    }

    #[test]
    fn closed_at_weekends() {
        // Sat 21 Nov 2026, midday New York.
        assert!(!is_us_market_open(utc(2026, 11, 21, 17, 0)));
        assert!(!is_us_market_open(utc(2026, 11, 22, 17, 0)));
        // Mon 23 Nov 2026.
        assert!(is_us_market_open(utc(2026, 11, 23, 17, 0)));
    }
}
