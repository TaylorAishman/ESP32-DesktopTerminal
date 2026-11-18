//! Models, data store, parsers, API manager and platform traits.

#![forbid(unsafe_code)]

pub mod api;
pub mod api_error;
pub mod error;
pub mod model;
pub mod platform;
pub mod secret;
pub mod time;

pub use error::{AppError, Category, ErrorCode};
pub use secret::Secret;
