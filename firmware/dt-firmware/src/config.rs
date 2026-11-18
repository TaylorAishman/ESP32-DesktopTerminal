//! The configuration generated at build time from `config/*.toml`.

include!(concat!(env!("OUT_DIR"), "/config.rs"));
