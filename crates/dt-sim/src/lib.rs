//! Host simulator: renders pages to PNG and runs golden-image tests.

#![forbid(unsafe_code)]

pub mod image;
pub mod sheet;

/// Configuration generated from `config/api.toml` and the example user and
/// secrets files.
pub mod config {
    include!(concat!(env!("OUT_DIR"), "/config.rs"));
}
