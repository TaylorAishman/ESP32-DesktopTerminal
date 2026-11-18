//! Deciding which job runs next (REQ-API-01, REQ-API-02, REQ-API-09).
//!
//! A pure state machine: the caller supplies the time, runs the job it is
//! told to, and reports the outcome. No I/O, so it is tested over simulated
//! weeks on the host.
//!
//! Each job runs on a grid: its phase plus whole multiples of its interval.
//! After a success the next run is the next grid point, so a late run does
//! not drift the schedule and missed slots are not replayed in a burst.
//! After a failure the job retries on its backoff delay instead. Every run,
//! including retries, must get past the provider's rate limiter.

use core::time::Duration;

use super::backoff::{Backoff, Jitter};
use super::jobs::Job;
use super::rate_limit::RateLimiter;
use crate::error::AppError;
use crate::platform::{Instant, UnixTime};

/// What to do next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Run the job with this index now.
    Run(usize),
    /// Nothing is due; ask again after this long.
    Wait(Duration),
}

/// How a run ended.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Success,
    Failure {
        error: AppError,
        retry_after: Option<Duration>,
    },
}

#[derive(Clone, Debug)]
struct JobState {
    provider: usize,
    /// First grid point.
    anchor: Instant,
    interval: Duration,
    due: Instant,
    backoff: Backoff,
}

/// Schedules a fixed set of jobs.
#[derive(Clone, Debug)]
pub struct Scheduler {
    jobs: Vec<JobState>,
    /// Provider ids, parallel to `limiters`.
    providers: Vec<&'static str>,
    limiters: Vec<RateLimiter>,
    jitter: Jitter,
}

impl Scheduler {
    /// Schedule `jobs` starting at `start`; each job's first run is at its
    /// phase. `seed` seeds the retry jitter.
    pub fn new(jobs: &[Job], start: Instant, seed: u64) -> Self {
        let mut providers: Vec<&'static str> = Vec::new();
        let mut limiters = Vec::new();
        let states = jobs
            .iter()
            .map(|j| {
                let provider = match providers.iter().position(|p| *p == j.provider.id) {
                    Some(i) => i,
                    None => {
                        providers.push(j.provider.id);
                        limiters.push(RateLimiter::new(j.provider.limits));
                        providers.len() - 1
                    }
                };
                let anchor = start + j.timing.phase;
                JobState {
                    provider,
                    anchor,
                    interval: j.timing.interval,
                    due: anchor,
                    backoff: Backoff::default(),
                }
            })
            .collect();
        Scheduler {
            jobs: states,
            providers,
            limiters,
            jitter: Jitter::new(seed),
        }
    }

    /// The next action at monotonic time `now` and wall-clock time `wall`.
    /// A `Run` has already been counted against the provider's limits.
    pub fn next(&mut self, now: Instant, wall: UnixTime) -> Action {
        let mut due: Vec<usize> = (0..self.jobs.len())
            .filter(|&i| self.jobs[i].due <= now)
            .collect();
        // Most overdue first; ties by job order.
        due.sort_by_key(|&i| (self.jobs[i].due, i));
        for i in due {
            let p = self.jobs[i].provider;
            match self.limiters[p].try_acquire(wall) {
                Ok(()) => return Action::Run(i),
                // Over a limit: wait for the window to reset.
                Err(wait) => self.jobs[i].due = now + wait,
            }
        }
        let next = self.jobs.iter().map(|j| j.due).min();
        match next {
            Some(t) => Action::Wait((t - now).max(Duration::from_millis(1))),
            None => Action::Wait(Duration::from_secs(3600)),
        }
    }

    /// Record how job `index`, started by the last `Run`, ended at `now`.
    pub fn report(&mut self, index: usize, outcome: &Outcome, now: Instant) {
        let jitter = self.jitter.sample();
        let job = &mut self.jobs[index];
        match outcome {
            Outcome::Success => {
                job.backoff.on_success();
                job.due = next_grid_point(job.anchor, job.interval, now);
            }
            Outcome::Failure { error, retry_after } => {
                let delay = job
                    .backoff
                    .on_failure(error, *retry_after, job.interval, jitter);
                job.due = now + delay;
            }
        }
    }

    /// Change a job's interval (e.g. at the market open or close), at time
    /// `now`. When the interval shrinks, a run already booked further out than
    /// one new interval is pulled in, keeping its offset modulo the new
    /// interval so staggered jobs stay staggered. The grid is re-anchored at
    /// the job's next due time.
    pub fn set_interval(&mut self, index: usize, interval: Duration, now: Instant) {
        let job = &mut self.jobs[index];
        if job.interval == interval {
            return;
        }
        job.interval = interval;
        if job.due > now + interval {
            let step = interval.as_millis().max(1) as u64;
            let ahead = job.due.as_millis() - now.as_millis();
            job.due = Instant::from_millis(now.as_millis() + ahead % step);
        }
        job.anchor = job.due;
    }

    pub fn interval(&self, index: usize) -> Duration {
        self.jobs[index].interval
    }

    /// When job `index` will next be due.
    pub fn due(&self, index: usize) -> Instant {
        self.jobs[index].due
    }

    /// Consecutive failures of job `index`.
    pub fn failures(&self, index: usize) -> u32 {
        self.jobs[index].backoff.failures()
    }

    /// Make job `index` due now, if its interval allows it at all (used for
    /// manual refreshes; the rate limiter still applies).
    pub fn make_due(&mut self, index: usize, now: Instant) {
        let job = &mut self.jobs[index];
        if job.due > now {
            job.due = now;
        }
    }

    /// Rate limiter of a provider, for persistence and diagnostics.
    pub fn limiter(&self, provider_id: &str) -> Option<&RateLimiter> {
        self.providers
            .iter()
            .position(|p| *p == provider_id)
            .map(|i| &self.limiters[i])
    }

    pub fn limiter_mut(&mut self, provider_id: &str) -> Option<&mut RateLimiter> {
        self.providers
            .iter()
            .position(|p| *p == provider_id)
            .map(move |i| &mut self.limiters[i])
    }

    pub fn provider_ids(&self) -> &[&'static str] {
        &self.providers
    }
}

/// The first point `anchor + k * interval` (k >= 0) strictly after `now`.
fn next_grid_point(anchor: Instant, interval: Duration, now: Instant) -> Instant {
    let step = interval.as_millis().max(1) as u64;
    if now < anchor {
        return anchor;
    }
    let elapsed = now.as_millis() - anchor.as_millis();
    Instant::from_millis(anchor.as_millis() + (elapsed / step + 1) * step)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::derive::JobTiming;
    use crate::api::jobs::JobKind;
    use crate::api::{Limit, Period};
    use crate::config::{AuthConfig, AuthLocation, EndpointConfig, ProviderConfig};
    use crate::error::ErrorCode;
    use crate::Secret;
    use std::collections::HashMap;

    const EP: EndpointConfig = EndpointConfig {
        id: "e",
        path: "/e",
        params: &[],
        weight: 1,
        min_interval_s: 1,
        max_staleness_s: 100_000,
        max_response_bytes: 1024,
        off_hours_interval_s: None,
    };

    const fn provider(id: &'static str, limits: &'static [Limit]) -> ProviderConfig {
        ProviderConfig {
            id,
            name: id,
            base_url: "https://x",
            host: "x",
            auth: AuthConfig {
                location: AuthLocation::Header,
                name: "k",
                secret_name: "k",
                key: Secret::new("k"),
            },
            limits,
            utilisation: 0.9,
            timeout_ms: 1000,
            endpoints: &[EP],
        }
    }

    static NEWS: ProviderConfig = provider(
        "news",
        &[Limit {
            calls: 100,
            per: Period::Day,
        }],
    );
    static QUOTES: ProviderConfig = provider(
        "quotes",
        &[Limit {
            calls: 60,
            per: Period::Minute,
        }],
    );

    fn job(p: &'static ProviderConfig, kind: JobKind, interval: u64, phase: u64) -> Job {
        Job {
            kind,
            provider: p,
            endpoint: &p.endpoints[0],
            params: vec![],
            timing: JobTiming {
                interval: Duration::from_secs(interval),
                phase: Duration::from_secs(phase),
                floored: false,
            },
            off_hours_interval: None,
        }
    }

    fn news_jobs() -> Vec<Job> {
        (0..4)
            .map(|i| job(&NEWS, JobKind::News(i), 3840, 960 * i as u64))
            .collect()
    }

    /// Run the scheduler for `secs`, with `fail(job, call_number)` deciding
    /// failures. Returns (job, wall second) for every run.
    fn simulate(
        jobs: &[Job],
        secs: u64,
        mut fail: impl FnMut(usize, usize) -> bool,
    ) -> Vec<(usize, i64)> {
        let wall0 = 1_794_873_600; // midnight UTC
        let mut s = Scheduler::new(jobs, Instant::ZERO, 7);
        let mut now = Instant::ZERO;
        let mut runs = Vec::new();
        while now.as_millis() < secs * 1000 {
            let wall = UnixTime(wall0 + (now.as_millis() / 1000) as i64);
            match s.next(now, wall) {
                Action::Run(i) => {
                    runs.push((i, wall.0));
                    let outcome = if fail(i, runs.len()) {
                        Outcome::Failure {
                            error: AppError::new(ErrorCode::NetTimeout),
                            retry_after: None,
                        }
                    } else {
                        Outcome::Success
                    };
                    // Each fetch takes 2 s.
                    now = now + Duration::from_secs(2);
                    s.report(i, &outcome, now);
                }
                Action::Wait(d) => now = now + d,
            }
        }
        runs
    }

    #[test]
    fn a_week_of_news_is_evenly_spaced_and_within_limits() {
        let runs = simulate(&news_jobs(), 7 * 86_400, |_, _| false);
        let mut per_day: HashMap<i64, u32> = HashMap::new();
        for (_, t) in &runs {
            *per_day.entry(t.div_euclid(86_400)).or_default() += 1;
        }
        assert!(per_day.values().all(|&n| n <= 100), "{per_day:?}");
        // Calls are 16 minutes apart throughout.
        let gaps: Vec<i64> = runs.windows(2).map(|w| w[1].1 - w[0].1).collect();
        assert!(gaps.iter().all(|&g| g == 960), "{:?}", &gaps[..10]);
        // Each topic every 64 minutes.
        let topic0: Vec<i64> = runs.iter().filter(|r| r.0 == 0).map(|r| r.1).collect();
        assert!(topic0.windows(2).all(|w| w[1] - w[0] == 3840));
        assert_eq!(runs.len(), 7 * 86_400 / 960);
    }

    #[test]
    fn failed_job_backs_off_and_recovers() {
        // Job 0 fails its first three attempts.
        let mut failures = 0;
        let runs = simulate(&news_jobs(), 6 * 3600, |i, _| {
            if i == 0 && failures < 3 {
                failures += 1;
                true
            } else {
                false
            }
        });
        let job0: Vec<i64> = runs
            .iter()
            .filter(|r| r.0 == 0)
            .map(|r| r.1 - runs[0].1)
            .collect();
        // First at 0 s, retries after ~30 s, ~60 s, ~120 s (±25 %), then back
        // on the 3840 s grid.
        assert!((22..=40).contains(&(job0[1] - job0[0])), "{job0:?}");
        assert!((45..=77).contains(&(job0[2] - job0[1])), "{job0:?}");
        assert!((90..=152).contains(&(job0[3] - job0[2])), "{job0:?}");
        assert_eq!(job0[4], 3840);
        assert_eq!(job0[5], 7680);
    }

    #[test]
    fn limiter_blocks_a_too_fast_schedule() {
        // Misconfigured: 4 jobs every 30 s against 100/day.
        let jobs: Vec<Job> = (0..4)
            .map(|i| job(&NEWS, JobKind::News(i), 30, 0))
            .collect();
        let runs = simulate(&jobs, 2 * 86_400, |_, _| false);
        let mut per_day: HashMap<i64, u32> = HashMap::new();
        for (_, t) in &runs {
            *per_day.entry(t.div_euclid(86_400)).or_default() += 1;
        }
        assert_eq!(per_day.values().copied().collect::<Vec<_>>(), [100, 100]);
    }

    #[test]
    fn providers_are_independent_and_nobody_starves() {
        let mut jobs = news_jobs();
        jobs.extend((0..4).map(|i| job(&QUOTES, JobKind::Quote(i), 60, 15 * i as u64)));
        let runs = simulate(&jobs, 3 * 3600, |_, _| false);
        for i in 0..8 {
            let n = runs.iter().filter(|r| r.0 == i).count();
            let expected = if i < 4 { 3 } else { 3 * 60 };
            assert!(
                n.abs_diff(expected) <= 1,
                "job {i}: {n} runs, expected ~{expected}"
            );
        }
    }

    #[test]
    fn shortening_an_interval_pulls_runs_in_and_keeps_stagger() {
        let jobs: Vec<Job> = (0..4)
            .map(|i| job(&QUOTES, JobKind::Quote(i), 3600, 15 * i as u64))
            .collect();
        let mut s = Scheduler::new(&jobs, Instant::ZERO, 1);
        let now = Instant::from_millis(1_000);
        for i in 0..4 {
            s.set_interval(i, Duration::from_secs(60), now);
        }
        let dues: Vec<u64> = (0..4).map(|i| s.due(i).as_millis()).collect();
        // Already due (job 0 at 0 s) stays; the others keep their 15 s spacing.
        assert_eq!(dues, [0, 15_000, 30_000, 45_000]);
        // Lengthening never pulls a run in.
        s.set_interval(1, Duration::from_secs(3600), now);
        assert_eq!(s.due(1).as_millis(), 15_000);
    }

    #[test]
    fn grid_points() {
        let a = Instant::from_millis(1000);
        let i = Duration::from_secs(10);
        assert_eq!(next_grid_point(a, i, Instant::from_millis(500)), a);
        assert_eq!(
            next_grid_point(a, i, Instant::from_millis(1000)).as_millis(),
            11_000
        );
        assert_eq!(
            next_grid_point(a, i, Instant::from_millis(35_000)).as_millis(),
            41_000
        );
    }
}
