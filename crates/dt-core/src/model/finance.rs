//! Market data models.

use crate::platform::UnixTime;

/// A price snapshot for one instrument.
#[derive(Clone, Debug, PartialEq)]
pub struct Quote {
    /// Name shown to the user, e.g. "DJIA (DIA)".
    pub label: String,
    /// Ticker sent to the provider, e.g. "DIA".
    pub symbol: String,
    pub price: f32,
    /// Change from the previous close.
    pub change: f32,
    pub change_pct: f32,
    pub day_low: f32,
    pub day_high: f32,
    pub open: f32,
    pub previous_close: f32,
    /// Time of the last trade, as reported by the provider.
    pub at: UnixTime,
}

impl Quote {
    pub fn is_up(&self) -> bool {
        self.change > 0.0
    }

    pub fn is_down(&self) -> bool {
        self.change < 0.0
    }
}

/// One sampled price.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PricePoint {
    pub at: UnixTime,
    pub price: f32,
}

/// Number of samples kept per instrument for the trend chart.
pub const HISTORY_CAPACITY: usize = 256;

/// Fixed-capacity ring buffer of prices sampled on the device. When full, the
/// oldest sample is overwritten. Free market data plans do not include
/// historical candles, so the chart is built from what the device saw.
#[derive(Clone, Debug, PartialEq)]
pub struct PriceHistory {
    samples: Box<[PricePoint]>,
    /// Index the next sample is written to.
    head: usize,
    len: usize,
}

impl PriceHistory {
    pub fn new() -> Self {
        Self::with_capacity(HISTORY_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0, "history capacity must be positive");
        PriceHistory {
            samples: vec![
                PricePoint {
                    at: UnixTime(0),
                    price: 0.0
                };
                capacity
            ]
            .into_boxed_slice(),
            head: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, point: PricePoint) {
        self.samples[self.head] = point;
        self.head = (self.head + 1) % self.samples.len();
        self.len = (self.len + 1).min(self.samples.len());
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.samples.len()
    }

    pub fn last(&self) -> Option<PricePoint> {
        (self.len > 0).then(|| {
            let cap = self.samples.len();
            self.samples[(self.head + cap - 1) % cap]
        })
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }

    /// Samples from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = PricePoint> + '_ {
        let cap = self.samples.len();
        let start = (self.head + cap - self.len) % cap;
        (0..self.len).map(move |i| self.samples[(start + i) % cap])
    }
}

impl Default for PriceHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(t: i64) -> PricePoint {
        PricePoint {
            at: UnixTime(t),
            price: t as f32,
        }
    }

    #[test]
    fn iterates_oldest_first_before_wrapping() {
        let mut h = PriceHistory::with_capacity(4);
        assert!(h.is_empty());
        assert_eq!(h.last(), None);
        h.push(p(1));
        h.push(p(2));
        let v: Vec<i64> = h.iter().map(|x| x.at.0).collect();
        assert_eq!(v, [1, 2]);
        assert_eq!(h.last(), Some(p(2)));
    }

    #[test]
    fn overwrites_oldest_when_full() {
        let mut h = PriceHistory::with_capacity(3);
        for t in 1..=5 {
            h.push(p(t));
        }
        assert_eq!(h.len(), 3);
        let v: Vec<i64> = h.iter().map(|x| x.at.0).collect();
        assert_eq!(v, [3, 4, 5]);
        assert_eq!(h.last(), Some(p(5)));
        h.clear();
        assert_eq!(h.iter().count(), 0);
    }

    #[test]
    fn direction() {
        let q = Quote {
            label: "VOO".into(),
            symbol: "VOO".into(),
            price: 500.0,
            change: -1.0,
            change_pct: -0.2,
            day_low: 498.0,
            day_high: 503.0,
            open: 501.0,
            previous_close: 501.0,
            at: UnixTime(0),
        };
        assert!(q.is_down() && !q.is_up());
    }
}
