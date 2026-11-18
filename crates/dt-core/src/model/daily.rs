//! Per-day summaries computed from 3-hour forecast steps (REQ-WX-06).

use super::weather::{Condition, DailySummary, Forecast, HourlyPoint};
use crate::platform::UnixTime;
use crate::time::{Date, LocalDateTime};

/// Group forecast steps by local calendar day. `utc_offset` gives the local
/// offset at a given time, so a daylight-saving change inside the forecast is
/// handled. The first day may be partial.
pub fn summarise_days(
    points: &[HourlyPoint],
    utc_offset: impl Fn(UnixTime) -> i32,
) -> Vec<DailySummary> {
    let mut days: Vec<(Date, Vec<&HourlyPoint>)> = Vec::new();
    for p in points {
        let date = LocalDateTime::from_unix(p.at, utc_offset(p.at)).date;
        match days.last_mut() {
            Some((d, v)) if *d == date => v.push(p),
            _ => days.push((date, vec![p])),
        }
    }
    days.into_iter()
        .map(|(date, steps)| DailySummary {
            date,
            min_c: steps
                .iter()
                .map(|p| p.temperature_c)
                .fold(f32::INFINITY, f32::min),
            max_c: steps
                .iter()
                .map(|p| p.temperature_c)
                .fold(f32::NEG_INFINITY, f32::max),
            condition: dominant(&steps),
            precip_probability: steps
                .iter()
                .map(|p| p.precip_probability)
                .fold(0.0, f32::max),
        })
        .collect()
}

/// The condition to show for a day: the most significant one that occurs in
/// at least two steps (a single stray shower does not make a rainy day), or
/// failing that the most frequent one (ties go to the more significant).
fn dominant(steps: &[&HourlyPoint]) -> Condition {
    let count = |c: Condition| steps.iter().filter(|p| p.condition == c).count();
    let mut present: Vec<Condition> = steps.iter().map(|p| p.condition).collect();
    present.sort_by_key(|c| std::cmp::Reverse(c.severity()));
    present.dedup();
    if let Some(c) = present.iter().copied().find(|&c| count(c) >= 2) {
        return c;
    }
    present
        .iter()
        .copied()
        .max_by_key(|&c| (count(c), c.severity()))
        .unwrap_or(Condition::Unknown)
}

impl Forecast {
    /// Steps as given plus their daily summaries.
    pub fn from_points(hourly: Vec<HourlyPoint>, utc_offset: impl Fn(UnixTime) -> i32) -> Self {
        let daily = summarise_days(&hourly, utc_offset);
        Forecast { hourly, daily }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(at: i64, t: f32, c: Condition, pop: f32) -> HourlyPoint {
        HourlyPoint {
            at: UnixTime(at),
            temperature_c: t,
            condition: c,
            is_day: true,
            precip_probability: pop,
            precip_mm: 0.0,
        }
    }

    // 2026-11-17 00:00 UTC
    const DAY: i64 = 1_794_873_600;
    const H: i64 = 3600;

    #[test]
    fn groups_by_local_day_with_partial_first_day() {
        // Steps every 3 h from 15:00 UTC on the 17th for 2 days, in UTC-5.
        let pts: Vec<_> = (0..16)
            .map(|i| step(DAY + 15 * H + i * 3 * H, i as f32, Condition::Clear, 0.0))
            .collect();
        let days = summarise_days(&pts, |_| -5 * 3600);
        // Local 10:00 on the 17th to 07:00 on the 19th: three local days.
        assert_eq!(days.len(), 3);
        assert_eq!(
            days[0].date,
            Date {
                year: 2026,
                month: 11,
                day: 17
            }
        );
        // 10:00, 13:00, 16:00, 19:00, 22:00 local = 5 steps on day one.
        assert_eq!(days[0].min_c, 0.0);
        assert_eq!(days[0].max_c, 4.0);
        assert_eq!(days[1].min_c, 5.0);
        assert_eq!(days[1].max_c, 12.0);
        assert_eq!(
            days[2].date,
            Date {
                year: 2026,
                month: 11,
                day: 19
            }
        );
    }

    #[test]
    fn utc_boundary_differs_from_local() {
        let pts = [
            step(DAY + 21 * H, 1.0, Condition::Clear, 0.0),
            step(DAY + 24 * H, 2.0, Condition::Clear, 0.0),
        ];
        assert_eq!(summarise_days(&pts, |_| 0).len(), 2);
        assert_eq!(summarise_days(&pts, |_| -5 * 3600).len(), 1);
    }

    #[test]
    fn dominant_condition_rules() {
        let c = |cs: &[Condition]| {
            let pts: Vec<_> = cs
                .iter()
                .enumerate()
                .map(|(i, &c)| step(DAY + i as i64 * 3 * H, 0.0, c, 0.0))
                .collect();
            summarise_days(&pts, |_| 0)[0].condition
        };
        use Condition::*;
        // One stray shower among clouds: cloudy.
        assert_eq!(c(&[Cloudy, Cloudy, Rain, Cloudy]), Cloudy);
        // Two rainy steps: rain wins over more frequent clouds.
        assert_eq!(c(&[Cloudy, Rain, Rain, Cloudy, Cloudy]), Rain);
        // All different: tie on count goes to the more significant.
        assert_eq!(c(&[Clear, PartlyCloudy, Snow]), Snow);
    }

    #[test]
    fn max_precip_probability() {
        let pts = [
            step(DAY, 0.0, Condition::Rain, 0.2),
            step(DAY + 3 * H, 0.0, Condition::Rain, 0.7),
        ];
        let d = summarise_days(&pts, |_| 0);
        assert!((d[0].precip_probability - 0.7).abs() < 1e-6);
    }

    #[test]
    fn fixture_gives_six_days() {
        let pts = crate::parse::owm::parse_forecast(include_bytes!(
            "../../tests/fixtures/owm_forecast.json"
        ))
        .unwrap();
        let f = Forecast::from_points(pts, |_| -5 * 3600);
        assert_eq!(f.hourly.len(), 40);
        assert_eq!(f.daily.len(), 6);
        assert!(f.daily.iter().all(|d| d.min_c <= d.max_c));
    }
}
