//! The API manager: scheduling, rate limiting and fetching.

pub mod backoff;
pub mod derive;
pub mod jobs;
pub mod rate_limit;
pub mod request;

pub use backoff::{Backoff, Jitter};
pub use derive::{derive, JobSpec, JobTiming};
pub use jobs::{build_jobs, Job, JobKind};
pub use rate_limit::{Limit, Period, RateLimiter};
pub use request::{build_request, ApiRequest};
