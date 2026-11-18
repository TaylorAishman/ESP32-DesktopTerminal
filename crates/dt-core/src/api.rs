//! The API manager: scheduling, rate limiting and fetching.

pub mod backoff;
pub mod derive;
pub mod fetch;
pub mod jobs;
pub mod manager;
pub mod market;
pub mod persist;
pub mod rate_limit;
pub mod request;
pub mod scheduler;

pub use backoff::{Backoff, Jitter};
pub use derive::{derive, JobSpec, JobTiming};
pub use fetch::{execute, FetchReport};
pub use jobs::{build_jobs, Job, JobKind};
pub use manager::{ApiManager, Step};
pub use rate_limit::{Limit, Period, RateLimiter};
pub use request::{build_request, ApiRequest};
pub use scheduler::{Action, Outcome, Scheduler};
