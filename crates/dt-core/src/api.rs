//! The API manager: scheduling, rate limiting and fetching.

pub mod derive;
pub mod rate_limit;

pub use derive::{derive, JobSpec, JobTiming};
pub use rate_limit::{Limit, Period, RateLimiter};
