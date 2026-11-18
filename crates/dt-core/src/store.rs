//! The data store: the latest data for every page, with freshness and errors.
//!
//! The API manager writes it with [`Store::apply`]; the UI renders from a
//! [`Snapshot`] taken under a short lock (REQ-DM-01, REQ-DM-02). A failed
//! refresh keeps the last good data and records the error (REQ-ERR-04).

use crate::error::AppError;
use crate::model::{Current, Forecast, NewsTopic, PriceHistory, PricePoint, Quote};
use crate::platform::UnixTime;

/// The four pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    Home,
    Weather,
    News,
    Finance,
}

impl Page {
    pub const ALL: [Page; 4] = [Page::Home, Page::Weather, Page::News, Page::Finance];

    fn bit(self) -> u8 {
        match self {
            Page::Home => 1,
            Page::Weather => 2,
            Page::News => 4,
            Page::Finance => 8,
        }
    }
}

/// A set of pages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PageSet(u8);

impl PageSet {
    pub const EMPTY: PageSet = PageSet(0);
    pub const ALL: PageSet = PageSet(0b1111);

    pub fn of(pages: &[Page]) -> Self {
        PageSet(pages.iter().fold(0, |acc, p| acc | p.bit()))
    }

    pub fn contains(self, page: Page) -> bool {
        self.0 & page.bit() != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn union(self, other: PageSet) -> PageSet {
        PageSet(self.0 | other.0)
    }

    pub fn iter(self) -> impl Iterator<Item = Page> {
        Page::ALL.into_iter().filter(move |p| self.contains(*p))
    }
}

/// One piece of data and what is known about its freshness.
#[derive(Clone, Debug, PartialEq)]
pub struct Slot<T> {
    /// The last good value. Kept when a later refresh fails.
    pub data: Option<T>,
    /// When `data` was fetched.
    pub fetched_at: Option<UnixTime>,
    /// The error from the most recent attempt, if it failed.
    pub last_error: Option<AppError>,
}

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Slot {
            data: None,
            fetched_at: None,
            last_error: None,
        }
    }
}

impl<T> Slot<T> {
    fn set(&mut self, at: UnixTime, result: Result<T, AppError>) {
        match result {
            Ok(value) => {
                self.data = Some(value);
                self.fetched_at = Some(at);
                self.last_error = None;
            }
            Err(e) => self.last_error = Some(e),
        }
    }

    /// Age of `data` at `now`, if there is any.
    pub fn age(&self, now: UnixTime) -> Option<i64> {
        self.fetched_at
            .map(|t| (now.seconds() - t.seconds()).max(0))
    }

    /// The most recent attempt failed (any data shown is stale).
    pub fn has_error(&self) -> bool {
        self.last_error.is_some()
    }
}

/// A quote and the prices sampled for its trend chart.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QuoteSlot {
    pub quote: Slot<Quote>,
    pub history: PriceHistory,
}

/// What a fetch produced.
#[derive(Clone, Debug, PartialEq)]
pub enum UpdateKind {
    Current(Result<Current, AppError>),
    Forecast(Result<Forecast, AppError>),
    /// Index into the configured topics.
    News(usize, Result<NewsTopic, AppError>),
    /// Index into the configured instruments.
    Quote(usize, Result<Quote, AppError>),
}

/// The outcome of one fetch, stamped with when it completed.
#[derive(Clone, Debug, PartialEq)]
pub struct Update {
    pub at: UnixTime,
    pub kind: UpdateKind,
}

/// Everything the pages show.
#[derive(Clone, Debug, PartialEq)]
pub struct Store {
    pub current: Slot<Current>,
    pub forecast: Slot<Forecast>,
    /// One per configured topic, in display order.
    pub news: Vec<Slot<NewsTopic>>,
    /// One per configured instrument, in display order.
    pub quotes: Vec<QuoteSlot>,
}

/// A copy of the store for rendering, taken without holding the lock while
/// drawing.
pub type Snapshot = Store;

impl Store {
    pub fn new(topics: usize, instruments: usize) -> Self {
        Store {
            current: Slot::default(),
            forecast: Slot::default(),
            news: (0..topics).map(|_| Slot::default()).collect(),
            quotes: (0..instruments).map(|_| QuoteSlot::default()).collect(),
        }
    }

    /// Record a fetch result. Returns the pages whose content changed.
    pub fn apply(&mut self, update: Update) -> PageSet {
        let at = update.at;
        match update.kind {
            UpdateKind::Current(r) => {
                self.current.set(at, r);
                PageSet::of(&[Page::Weather, Page::Home])
            }
            UpdateKind::Forecast(r) => {
                self.forecast.set(at, r);
                PageSet::of(&[Page::Weather, Page::Home])
            }
            UpdateKind::News(i, r) => match self.news.get_mut(i) {
                Some(slot) => {
                    slot.set(at, r);
                    PageSet::of(&[Page::News, Page::Home])
                }
                None => PageSet::EMPTY,
            },
            UpdateKind::Quote(i, r) => match self.quotes.get_mut(i) {
                Some(slot) => {
                    if let Ok(q) = &r {
                        record_sample(&mut slot.history, q);
                    }
                    slot.quote.set(at, r);
                    PageSet::of(&[Page::Finance, Page::Home])
                }
                None => PageSet::EMPTY,
            },
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        self.clone()
    }
}

/// Add a quote's price to the chart history: once per trade time (no
/// duplicates while the market is closed), restarting each trading day.
fn record_sample(history: &mut PriceHistory, q: &Quote) {
    if let Some(last) = history.last() {
        if last.at == q.at {
            return;
        }
        // UTC day boundaries fall after the US close, so this starts a new
        // chart each trading day.
        if last.at.seconds().div_euclid(86_400) != q.at.seconds().div_euclid(86_400) {
            history.clear();
        }
    }
    history.push(PricePoint {
        at: q.at,
        price: q.price,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::model::Headline;

    fn quote(at: i64, price: f32) -> Quote {
        Quote {
            label: "VOO".into(),
            symbol: "VOO".into(),
            price,
            change: 0.0,
            change_pct: 0.0,
            day_low: price,
            day_high: price,
            open: price,
            previous_close: price,
            at: UnixTime(at),
        }
    }

    fn topic(title: &str) -> NewsTopic {
        NewsTopic::new("Tech", [Headline::new(title, "src", None)])
    }

    #[test]
    fn success_replaces_and_failure_keeps_data() {
        let mut s = Store::new(2, 1);
        let pages = s.apply(Update {
            at: UnixTime(100),
            kind: UpdateKind::News(1, Ok(topic("first"))),
        });
        assert_eq!(pages, PageSet::of(&[Page::News, Page::Home]));
        assert_eq!(s.news[1].fetched_at, Some(UnixTime(100)));

        s.apply(Update {
            at: UnixTime(200),
            kind: UpdateKind::News(1, Err(AppError::new(ErrorCode::NetTimeout))),
        });
        let slot = &s.news[1];
        assert_eq!(slot.data.as_ref().unwrap().headlines()[0].title(), "first");
        assert_eq!(slot.fetched_at, Some(UnixTime(100)));
        assert!(slot.has_error());
        assert_eq!(slot.age(UnixTime(250)), Some(150));

        s.apply(Update {
            at: UnixTime(300),
            kind: UpdateKind::News(1, Ok(topic("second"))),
        });
        assert!(!s.news[1].has_error());
        assert!(s.news[0].data.is_none());
    }

    #[test]
    fn affected_pages() {
        let mut s = Store::new(1, 1);
        let w = s.apply(Update {
            at: UnixTime(1),
            kind: UpdateKind::Forecast(Ok(Forecast::default())),
        });
        assert!(w.contains(Page::Weather) && w.contains(Page::Home) && !w.contains(Page::News));
        let f = s.apply(Update {
            at: UnixTime(1),
            kind: UpdateKind::Quote(0, Ok(quote(1, 1.0))),
        });
        assert_eq!(f.iter().collect::<Vec<_>>(), [Page::Home, Page::Finance]);
        let none = s.apply(Update {
            at: UnixTime(1),
            kind: UpdateKind::Quote(5, Ok(quote(1, 1.0))),
        });
        assert!(none.is_empty());
    }

    #[test]
    fn quote_history_skips_duplicates_and_resets_daily() {
        let mut s = Store::new(0, 1);
        let day = 20_000 * 86_400;
        for (at, price) in [(day + 100, 1.0), (day + 100, 1.0), (day + 160, 2.0)] {
            s.apply(Update {
                at: UnixTime(at),
                kind: UpdateKind::Quote(0, Ok(quote(at, price))),
            });
        }
        assert_eq!(s.quotes[0].history.len(), 2);
        let next = day + 86_400 + 50;
        s.apply(Update {
            at: UnixTime(next),
            kind: UpdateKind::Quote(0, Ok(quote(next, 3.0))),
        });
        assert_eq!(s.quotes[0].history.len(), 1);
    }

    #[test]
    fn snapshot_is_independent() {
        let mut s = Store::new(1, 0);
        let snap = s.snapshot();
        s.apply(Update {
            at: UnixTime(1),
            kind: UpdateKind::News(0, Ok(topic("x"))),
        });
        assert!(snap.news[0].data.is_none());
    }
}
