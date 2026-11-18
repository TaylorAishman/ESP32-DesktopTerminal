//! The API manager: scheduling, rate limiting and fetching.

pub mod rate_limit;

pub use rate_limit::{Limit, Period, RateLimiter};
