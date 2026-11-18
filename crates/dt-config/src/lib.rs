//! Configuration schema, validation and code generation (runs on the build host).

#![forbid(unsafe_code)]

pub mod api;
pub mod error;

pub use api::{parse_api, ApiConfig};
pub use error::{ConfigError, ConfigErrors};
