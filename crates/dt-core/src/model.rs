//! Provider-neutral data models. Parsers produce them; pages display them.

pub mod weather;

pub use weather::{Condition, Current, DailySummary, Forecast, HourlyPoint};
