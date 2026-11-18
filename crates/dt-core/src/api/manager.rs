//! The API manager: the only component that makes network requests
//! (REQ-API-01). It runs on its own thread so network activity never blocks
//! buttons or rendering (REQ-API-06).

use core::time::Duration;
use std::sync::{Arc, Mutex};

use super::fetch::{execute, FetchReport};
use super::jobs::{build_jobs, Job};
use super::market::is_us_market_open;
use super::scheduler::{Action, Outcome, Scheduler};
use crate::config::Config;
use crate::platform::{Clock, HttpClient, Wifi};
use crate::store::{PageSet, Store};

/// Longest single wait, so the loop notices connectivity changes promptly.
pub const MAX_WAIT: Duration = Duration::from_secs(5);

/// What one step of the loop did.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// No network; nothing was attempted.
    Offline,
    /// Connected but the clock is not synchronised yet (TLS needs it).
    WaitingForTime,
    /// Nothing due for this long.
    Idle(Duration),
    /// A job ran.
    Fetched {
        job: usize,
        pages: PageSet,
        report: Box<FetchReport>,
    },
}

/// Called with the pages whose data changed after each fetch.
pub type Notify = Box<dyn FnMut(PageSet) + Send>;

pub struct ApiManager<H: HttpClient> {
    config: &'static Config,
    jobs: Vec<Job>,
    scheduler: Scheduler,
    http: H,
    buf: Vec<u8>,
    store: Arc<Mutex<Store>>,
    clock: Arc<dyn Clock>,
    wifi: Arc<dyn Wifi>,
    notify: Notify,
}

impl<H: HttpClient> ApiManager<H> {
    pub fn new(
        config: &'static Config,
        http: H,
        store: Arc<Mutex<Store>>,
        clock: Arc<dyn Clock>,
        wifi: Arc<dyn Wifi>,
        notify: Notify,
    ) -> Self {
        let jobs = build_jobs(config);
        let seed = clock.monotonic().as_millis() ^ 0x5DEE_CE66;
        let scheduler = Scheduler::new(&jobs, clock.monotonic(), seed);
        // One receive buffer, sized for the largest endpoint and reused.
        let size = jobs
            .iter()
            .map(|j| j.endpoint.max_response_bytes as usize)
            .max()
            .unwrap_or(1024);
        ApiManager {
            config,
            jobs,
            scheduler,
            http,
            buf: vec![0; size],
            store,
            clock,
            wifi,
            notify,
        }
    }

    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }

    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    pub fn scheduler_mut(&mut self) -> &mut Scheduler {
        &mut self.scheduler
    }

    /// Do at most one thing and say what it was. The caller sleeps after
    /// `Offline`, `WaitingForTime` and `Idle`.
    pub fn step(&mut self) -> Step {
        if !self.wifi.status().is_connected() {
            return Step::Offline;
        }
        let Some(wall) = self.clock.now() else {
            return Step::WaitingForTime;
        };
        self.apply_market_hours(wall);
        match self.scheduler.next(self.clock.monotonic(), wall) {
            Action::Wait(d) => Step::Idle(d),
            Action::Run(i) => {
                let job = &self.jobs[i];
                let report = execute(
                    job,
                    self.config,
                    &mut self.http,
                    &mut self.buf,
                    self.clock.as_ref(),
                );
                log_fetch(&job.name(), &report);
                let pages = match self.store.lock() {
                    Ok(mut s) => s.apply(report.update.clone()),
                    Err(poisoned) => poisoned.into_inner().apply(report.update.clone()),
                };
                self.scheduler
                    .report(i, &report.outcome, self.clock.monotonic());
                if !pages.is_empty() {
                    (self.notify)(pages);
                }
                Step::Fetched {
                    job: i,
                    pages,
                    report: Box::new(report),
                }
            }
        }
    }

    /// Poll quotes at their off-hours interval while US markets are closed.
    fn apply_market_hours(&mut self, wall: crate::platform::UnixTime) {
        let open = is_us_market_open(wall);
        let now = self.clock.monotonic();
        for (i, job) in self.jobs.iter().enumerate() {
            if let Some(off) = job.off_hours_interval {
                let interval = if open { job.timing.interval } else { off };
                self.scheduler.set_interval(i, interval, now);
            }
        }
    }

    /// Run forever, sleeping with `sleep` between steps.
    pub fn run(&mut self, mut sleep: impl FnMut(Duration)) -> ! {
        loop {
            match self.step() {
                Step::Offline | Step::WaitingForTime => sleep(Duration::from_secs(1)),
                Step::Idle(d) => sleep(d.min(MAX_WAIT)),
                Step::Fetched { .. } => {}
            }
        }
    }
}

fn log_fetch(name: &str, r: &FetchReport) {
    let status = r.status.map_or("-".to_owned(), |s| s.to_string());
    match &r.outcome {
        Outcome::Success => log::info!("{name}: HTTP {status}, {} B, {}", r.bytes, r.redacted_url),
        Outcome::Failure { error, .. } => log::warn!("{name}: {error} [{}]", r.redacted_url),
    }
}
