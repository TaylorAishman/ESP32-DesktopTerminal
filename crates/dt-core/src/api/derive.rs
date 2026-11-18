//! Deriving polling intervals from a provider's limits (Requirements §6.1).
//!
//! For each provider: take the tightest limit as a sustainable rate, keep
//! `utilisation` of it for scheduled calls, share that among the provider's
//! jobs by weight (budget a floored job cannot use goes to the others), round
//! intervals up to whole seconds, and stagger the jobs so the provider's calls
//! are evenly spaced. Evenly spaced calls at or below a limit's average rate
//! cannot exceed it in any window, calendar-aligned or rolling.

use core::time::Duration;

use crate::api::rate_limit::Limit;

/// What the derivation needs to know about one job.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JobSpec {
    /// Relative share of the provider's budget.
    pub weight: u32,
    /// Never run more often than this.
    pub min_interval: Duration,
}

/// When one job runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JobTiming {
    /// Time between runs.
    pub interval: Duration,
    /// Offset of the first run from the schedule's start.
    pub phase: Duration,
    /// True if the job runs at its `min_interval` rather than at its share
    /// of the limit (the limit would allow more).
    pub floored: bool,
}

impl JobTiming {
    pub fn calls_per_day(&self) -> f64 {
        86_400.0 / self.interval.as_secs_f64()
    }
}

/// The sustainable rate of the tightest limit, in calls per second, and the
/// limit it comes from. `None` if there are no limits.
pub fn binding_rate(limits: &[Limit]) -> Option<(f64, Limit)> {
    limits
        .iter()
        .map(|l| (f64::from(l.calls) / l.per.nominal().as_secs_f64(), *l))
        .min_by(|a, b| a.0.total_cmp(&b.0))
}

/// Intervals and phases for a provider's jobs, in the order given.
///
/// With no limits, every job runs at its `min_interval`.
pub fn derive(limits: &[Limit], utilisation: f64, jobs: &[JobSpec]) -> Vec<JobTiming> {
    let budget = binding_rate(limits).map(|(rate, _)| rate * utilisation.clamp(0.0, 1.0));

    // Water-filling: a job whose fair share would run it faster than its
    // floor is fixed at the floor, which frees budget for the rest.
    let mut floored = vec![budget.is_none(); jobs.len()];
    let mut rates = vec![0.0_f64; jobs.len()];
    if let Some(budget) = budget {
        loop {
            let used: f64 = (0..jobs.len())
                .filter(|&i| floored[i])
                .map(|i| 1.0 / jobs[i].min_interval.as_secs_f64())
                .sum();
            let free_weight: u32 = (0..jobs.len())
                .filter(|&i| !floored[i])
                .map(|i| jobs[i].weight.max(1))
                .sum();
            let left = (budget - used).max(0.0);
            let mut changed = false;
            let free: Vec<usize> = (0..jobs.len()).filter(|&i| !floored[i]).collect();
            for i in free {
                let share = left * f64::from(jobs[i].weight.max(1)) / f64::from(free_weight);
                if share * jobs[i].min_interval.as_secs_f64() >= 1.0 {
                    floored[i] = true;
                    changed = true;
                } else {
                    rates[i] = share;
                }
            }
            if !changed {
                break;
            }
        }
    }

    let intervals: Vec<Duration> = (0..jobs.len())
        .map(|i| {
            if floored[i] || rates[i] <= 0.0 {
                // Floored, or no budget at all: fall back to the floor; the
                // rate limiter still guards the provider.
                jobs[i].min_interval.max(Duration::from_secs(1))
            } else {
                let secs = (1.0 / rates[i]).ceil();
                Duration::from_secs(secs as u64).max(jobs[i].min_interval)
            }
        })
        .collect();

    // Stagger: job j starts j call-gaps after job 0, where a call gap is the
    // provider's average time between calls.
    let total_rate: f64 = intervals.iter().map(|d| 1.0 / d.as_secs_f64()).sum();
    let gap = if total_rate > 0.0 {
        1.0 / total_rate
    } else {
        0.0
    };
    (0..jobs.len())
        .map(|j| {
            let interval = intervals[j];
            let phase = Duration::from_secs_f64((j as f64 * gap) % interval.as_secs_f64());
            JobTiming {
                interval,
                phase: Duration::from_secs(phase.as_secs()),
                floored: floored[j] || budget.is_none(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::rate_limit::Period;

    fn lim(calls: u32, per: Period) -> Limit {
        Limit { calls, per }
    }

    fn job(weight: u32, min_s: u64) -> JobSpec {
        JobSpec {
            weight,
            min_interval: Duration::from_secs(min_s),
        }
    }

    const NEWS: [Limit; 1] = [Limit {
        calls: 100,
        per: Period::Day,
    }];

    #[test]
    fn newsapi_four_topics_every_64_minutes_staggered_16() {
        let t = derive(&NEWS, 0.9, &[job(1, 900); 4]);
        for (i, timing) in t.iter().enumerate() {
            assert_eq!(timing.interval, Duration::from_secs(64 * 60), "job {i}");
            assert!(!timing.floored);
            assert_eq!(timing.phase, Duration::from_secs(16 * 60 * i as u64));
        }
        let per_day: f64 = t.iter().map(JobTiming::calls_per_day).sum();
        assert!((per_day - 90.0).abs() < 0.01, "{per_day}");
    }

    #[test]
    fn adding_a_job_lengthens_every_interval() {
        let four = derive(&NEWS, 0.9, &[job(1, 900); 4]);
        let five = derive(&NEWS, 0.9, &[job(1, 900); 5]);
        assert_eq!(five[0].interval, Duration::from_secs(80 * 60));
        assert!(five.iter().all(|t| t.interval > four[0].interval));
    }

    #[test]
    fn floor_overrides_a_generous_limit() {
        let owm = [lim(60, Period::Minute), lim(1_000_000, Period::Month)];
        let t = derive(&owm, 0.9, &[job(1, 600), job(1, 1800)]);
        assert_eq!(t[0].interval, Duration::from_secs(600));
        assert_eq!(t[1].interval, Duration::from_secs(1800));
        assert!(t[0].floored && t[1].floored);
    }

    #[test]
    fn slowest_window_binds() {
        let owm = [lim(60, Period::Minute), lim(1_000_000, Period::Month)];
        let (rate, l) = binding_rate(&owm).unwrap();
        assert_eq!(l.per, Period::Month);
        assert!((rate - 1_000_000.0 / (31.0 * 86_400.0)).abs() < 1e-9);
        // With tiny floors the month limit decides: ~6 s per job for two jobs.
        let t = derive(&owm, 0.9, &[job(1, 1), job(1, 1)]);
        assert_eq!(t[0].interval, Duration::from_secs(6));
    }

    #[test]
    fn weights_skew_the_split() {
        let t = derive(&NEWS, 0.9, &[job(2, 60), job(1, 60)]);
        // 90/day split 60/30: every 24 min and every 48 min.
        assert_eq!(t[0].interval, Duration::from_secs(1440));
        assert_eq!(t[1].interval, Duration::from_secs(2880));
    }

    #[test]
    fn freed_budget_goes_to_unfloored_jobs() {
        // One job floored at 1 h uses 24/day; the other gets the remaining 66.
        let t = derive(&NEWS, 0.9, &[job(1, 3600), job(1, 60)]);
        assert!(t[0].floored);
        assert_eq!(t[0].interval, Duration::from_secs(3600));
        assert_eq!(
            t[1].interval,
            Duration::from_secs((86_400.0_f64 / 66.0).ceil() as u64)
        );
    }

    #[test]
    fn schedule_never_exceeds_limits() {
        // Simulate a week of the derived schedule against the limits.
        for n in 1..=8 {
            let t = derive(&NEWS, 0.9, &vec![job(1, 900); n]);
            let mut per_day = std::collections::HashMap::<u64, u32>::new();
            for timing in &t {
                let mut at = timing.phase.as_secs();
                while at < 7 * 86_400 {
                    *per_day.entry(at / 86_400).or_default() += 1;
                    at += timing.interval.as_secs();
                }
            }
            assert!(per_day.values().all(|&c| c <= 100), "n={n}: {per_day:?}");
        }
    }

    #[test]
    fn no_limits_means_floors() {
        let t = derive(&[], 0.9, &[job(1, 60), job(1, 120)]);
        assert_eq!(t[0].interval, Duration::from_secs(60));
        assert_eq!(t[1].interval, Duration::from_secs(120));
    }
}
