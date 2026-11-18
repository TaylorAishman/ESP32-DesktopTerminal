//! Configuration schema, validation and code generation (runs on the build host).

#![forbid(unsafe_code)]

pub mod api;
pub mod error;
pub mod secrets;
pub mod user;

pub use api::{parse_api, ApiConfig};
pub use error::{ConfigError, ConfigErrors};
pub use secrets::{parse_secrets, SecretsConfig};
pub use user::{parse_user, UserConfig};
