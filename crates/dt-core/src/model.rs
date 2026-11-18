//! Provider-neutral data models. Parsers produce them; pages display them.

pub mod news;
pub mod weather;

pub use news::{Headline, NewsTopic};
pub use weather::{Condition, Current, DailySummary, Forecast, HourlyPoint};
