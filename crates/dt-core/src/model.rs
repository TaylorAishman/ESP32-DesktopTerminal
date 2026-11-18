//! Provider-neutral data models. Parsers produce them; pages display them.

pub mod finance;
pub mod news;
pub mod weather;

pub use finance::{PriceHistory, PricePoint, Quote};
pub use news::{Headline, NewsTopic};
pub use weather::{Condition, Current, DailySummary, Forecast, HourlyPoint};
