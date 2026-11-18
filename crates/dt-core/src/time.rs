//! Calendar arithmetic on Unix time, without a time-zone database.
//!
//! The device knows its UTC offset from the platform `Clock`; these helpers
//! turn a UTC timestamp plus that offset into a local date and time.

use crate::platform::UnixTime;

/// Day of the week.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    pub fn short_name(self) -> &'static str {
        match self {
            Weekday::Mon => "Mon",
            Weekday::Tue => "Tue",
            Weekday::Wed => "Wed",
            Weekday::Thu => "Thu",
            Weekday::Fri => "Fri",
            Weekday::Sat => "Sat",
            Weekday::Sun => "Sun",
        }
    }

    pub fn is_weekend(self) -> bool {
        matches!(self, Weekday::Sat | Weekday::Sun)
    }
}

/// A calendar date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    /// 1 to 12.
    pub month: u8,
    /// 1 to 31.
    pub day: u8,
}

impl Date {
    /// Days since 1970-01-01.
    pub fn days_since_epoch(self) -> i64 {
        days_from_civil(self.year, self.month, self.day)
    }

    pub fn from_days_since_epoch(days: i64) -> Self {
        civil_from_days(days)
    }

    pub fn weekday(self) -> Weekday {
        // 1970-01-01 was a Thursday.
        const ORDER: [Weekday; 7] = [
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
            Weekday::Sun,
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
        ];
        ORDER[self.days_since_epoch().rem_euclid(7) as usize]
    }

    pub fn month_short_name(self) -> &'static str {
        const NAMES: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        NAMES[(self.month.clamp(1, 12) - 1) as usize]
    }
}

/// A local date and time of day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalDateTime {
    pub date: Date,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl LocalDateTime {
    /// Local time for `at`, `utc_offset` seconds ahead of UTC.
    pub fn from_unix(at: UnixTime, utc_offset: i32) -> Self {
        let local = at.seconds() + i64::from(utc_offset);
        let days = local.div_euclid(86_400);
        let secs = local.rem_euclid(86_400);
        LocalDateTime {
            date: civil_from_days(days),
            hour: (secs / 3600) as u8,
            minute: (secs % 3600 / 60) as u8,
            second: (secs % 60) as u8,
        }
    }

    /// Minutes since local midnight.
    pub fn minute_of_day(self) -> u16 {
        u16::from(self.hour) * 60 + u16::from(self.minute)
    }
}

// Howard Hinnant's algorithms: http://howardhinnant.github.io/date_algorithms.html
fn days_from_civil(y: i32, m: u8, d: u8) -> i64 {
    let y = i64::from(y) - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> Date {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
    Date { year, month, day }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_and_known_dates() {
        let t = LocalDateTime::from_unix(UnixTime(0), 0);
        assert_eq!(
            t.date,
            Date {
                year: 1970,
                month: 1,
                day: 1
            }
        );
        assert_eq!(t.date.weekday(), Weekday::Thu);

        // 2026-11-17 14:05:09 UTC
        let t = LocalDateTime::from_unix(UnixTime(1_794_924_309), 0);
        assert_eq!(
            t.date,
            Date {
                year: 2026,
                month: 11,
                day: 17
            }
        );
        assert_eq!((t.hour, t.minute, t.second), (14, 5, 9));
        assert_eq!(t.date.weekday(), Weekday::Tue);
    }

    #[test]
    fn offset_crosses_midnight() {
        // 2026-11-17 02:00 UTC is 2026-11-16 21:00 in UTC-5.
        let t = LocalDateTime::from_unix(UnixTime(1_794_880_800), -5 * 3600);
        assert_eq!(
            t.date,
            Date {
                year: 2026,
                month: 11,
                day: 16
            }
        );
        assert_eq!(t.hour, 21);
    }

    #[test]
    fn leap_day_round_trip() {
        let d = Date {
            year: 2028,
            month: 2,
            day: 29,
        };
        assert_eq!(Date::from_days_since_epoch(d.days_since_epoch()), d);
        for days in [-1_000_000i64, -1, 0, 59, 365, 20_000, 1_000_000] {
            let d = Date::from_days_since_epoch(days);
            assert_eq!(d.days_since_epoch(), days);
        }
    }
}
