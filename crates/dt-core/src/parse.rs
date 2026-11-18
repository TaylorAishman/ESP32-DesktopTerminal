//! Provider response parsers. Each turns one endpoint's JSON into models and
//! returns a short reason on failure; the fetch executor turns that into an
//! `AppError` naming the provider and endpoint.

pub mod owm;
