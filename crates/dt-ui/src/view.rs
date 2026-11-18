//! View models: everything a page shows, already formatted. Built from a
//! store snapshot; pages only place these values (REQ-DM-01).

use dt_core::api::market::is_us_market_open;
use dt_core::config::{Config, Units};
use dt_core::error::AppError;
use dt_core::model::{DailySummary, Headline, HourlyPoint, Quote};
use dt_core::platform::UnixTime;
use dt_core::store::{Page, QuoteSlot, Slot, Snapshot};

use crate::format;
use crate::icon::Icon;
use crate::weather_icon::condition_icon;

/// What the renderer needs to know besides the data.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Wall-clock time, if synchronised.
    pub now: Option<UnixTime>,
    /// Local offset from UTC, seconds.
    pub utc_offset: i32,
    pub units: Units,
    pub location: String,
    /// Wi-Fi signal strength; `None` when offline.
    pub wifi_rssi: Option<i8>,
}

impl Context {
    pub fn from_config(
        config: &Config,
        now: Option<UnixTime>,
        utc_offset: i32,
        wifi_rssi: Option<i8>,
    ) -> Self {
        Context {
            now,
            utc_offset,
            units: config.location.units,
            location: config.location.name.to_owned(),
            wifi_rssi,
        }
    }
}

/// The state of one block of data on a page.
#[derive(Clone, Debug, PartialEq)]
pub enum Panel<T> {
    /// No data yet and no error: waiting for the first fetch.
    Loading,
    /// No data, and the last attempt failed.
    Failed(ErrorView),
    /// Current data.
    Fresh(T),
    /// Data is shown but out of date: the last refresh failed or it is old.
    Stale {
        data: T,
        age: String,
        error: Option<ErrorView>,
    },
}

impl<T> Panel<T> {
    pub fn data(&self) -> Option<&T> {
        match self {
            Panel::Fresh(d) | Panel::Stale { data: d, .. } => Some(d),
            _ => None,
        }
    }

    pub fn is_problem(&self) -> bool {
        matches!(self, Panel::Failed(_) | Panel::Stale { .. })
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Panel<U> {
        match self {
            Panel::Loading => Panel::Loading,
            Panel::Failed(e) => Panel::Failed(e),
            Panel::Fresh(d) => Panel::Fresh(f(d)),
            Panel::Stale { data, age, error } => Panel::Stale {
                data: f(data),
                age,
                error,
            },
        }
    }
}

/// An error, ready to show (REQ-ERR-03).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorView {
    pub code: String,
    pub message: String,
    pub hint: String,
}

impl From<&AppError> for ErrorView {
    fn from(e: &AppError) -> Self {
        ErrorView {
            code: e.code().id(),
            message: e.message().to_owned(),
            hint: e.hint_text().to_owned(),
        }
    }
}

/// Data older than this is shown as stale even without an error.
pub mod max_age {
    pub const CURRENT: i64 = 2 * 3600;
    pub const FORECAST: i64 = 6 * 3600;
    pub const NEWS: i64 = 6 * 3600;
    /// While the market is open.
    pub const QUOTE_OPEN: i64 = 30 * 60;
}

fn panel<T, U>(slot: &Slot<T>, ctx: &Context, max_age: i64, f: impl FnOnce(&T) -> U) -> Panel<U> {
    let error = slot.last_error.as_ref().map(ErrorView::from);
    match &slot.data {
        None => match error {
            Some(e) => Panel::Failed(e),
            None => Panel::Loading,
        },
        Some(d) => {
            let age = ctx.now.and_then(|n| slot.age(n));
            let old = age.is_some_and(|a| a > max_age);
            if error.is_some() || old {
                Panel::Stale {
                    data: f(d),
                    age: age.map(format::age).unwrap_or_default(),
                    error,
                }
            } else {
                Panel::Fresh(f(d))
            }
        }
    }
}

// --------------------------------------------------------------------- Weather

#[derive(Clone, Debug, PartialEq)]
pub struct CurrentView {
    pub icon: Icon,
    pub temperature: String,
    pub feels_like: String,
    pub condition: String,
    pub humidity: String,
    pub wind: String,
    pub sunrise: String,
    pub sunset: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HourView {
    pub time: String,
    pub icon: Icon,
    /// In display units, for the chart.
    pub temperature_value: f32,
    pub temperature: String,
    pub chance: f32,
    pub chance_text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DayView {
    /// "Today" or a weekday.
    pub name: String,
    pub icon: Icon,
    pub high: String,
    pub low: String,
    pub chance: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeatherView {
    pub location: String,
    pub current: Panel<CurrentView>,
    /// Today's high and low, e.g. "H 52°  L 41°", when the forecast has today.
    pub today: Option<String>,
    pub hourly: Panel<Vec<HourView>>,
    pub daily: Panel<Vec<DayView>>,
}

/// Forecast steps shown in the hourly strip (3 h each: a day).
pub const HOURLY_STEPS: usize = 8;
/// Days shown in the daily row.
pub const DAYS: usize = 5;

pub fn weather(s: &Snapshot, ctx: &Context) -> WeatherView {
    let u = ctx.units;
    let off = ctx.utc_offset;
    let current = panel(&s.current, ctx, max_age::CURRENT, |c| CurrentView {
        icon: condition_icon(c.condition, c.is_day),
        temperature: format::temperature(c.temperature_c, u),
        feels_like: format!("Feels like {}", format::temperature(c.feels_like_c, u)),
        condition: format::sentence(&c.description),
        humidity: format!("{}%", c.humidity_pct),
        wind: format::wind(c.wind_speed_ms, c.wind_deg, u),
        sunrise: format::clock(c.sunrise, off, u),
        sunset: format::clock(c.sunset, off, u),
    });
    let today_date = ctx
        .now
        .map(|n| dt_core::time::LocalDateTime::from_unix(n, off).date);
    let today = s.forecast.data.as_ref().and_then(|f| {
        let d = f.daily.iter().find(|d| Some(d.date) == today_date)?;
        Some(format!(
            "H {}  L {}",
            format::temperature(d.max_c, u),
            format::temperature(d.min_c, u)
        ))
    });
    let now = ctx.now.unwrap_or(UnixTime(i64::MIN));
    let hourly = panel(&s.forecast, ctx, max_age::FORECAST, |f| {
        f.hourly
            .iter()
            // Steps that end after now.
            .filter(|p| p.at.seconds() + 3 * 3600 > now.seconds())
            .take(HOURLY_STEPS)
            .map(|p| hour_view(p, ctx))
            .collect()
    });
    let daily = panel(&s.forecast, ctx, max_age::FORECAST, |f| {
        f.daily
            .iter()
            .filter(|d| today_date.is_none_or(|t| d.date >= t))
            .take(DAYS)
            .map(|d| day_view(d, today_date, u))
            .collect()
    });
    WeatherView {
        location: ctx.location.clone(),
        current,
        today,
        hourly,
        daily,
    }
}

fn hour_view(p: &HourlyPoint, ctx: &Context) -> HourView {
    HourView {
        time: format::hour(p.at, ctx.utc_offset, ctx.units),
        icon: condition_icon(p.condition, p.is_day),
        temperature_value: format::temperature_value(p.temperature_c, ctx.units),
        temperature: format::temperature(p.temperature_c, ctx.units),
        chance: p.precip_probability,
        chance_text: format::chance(p.precip_probability),
    }
}

fn day_view(d: &DailySummary, today: Option<dt_core::time::Date>, u: Units) -> DayView {
    DayView {
        name: if Some(d.date) == today {
            "Today".into()
        } else {
            d.date.weekday().short_name().into()
        },
        icon: condition_icon(d.condition, true),
        high: format::temperature(d.max_c, u),
        low: format::temperature(d.min_c, u),
        chance: format::chance(d.precip_probability),
    }
}

// ------------------------------------------------------------------------ News

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadlineView {
    pub title: String,
    pub source: String,
    /// "2 h ago"; empty when the time is unknown.
    pub age: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TopicView {
    pub title: String,
    pub headlines: Panel<Vec<HeadlineView>>,
}

pub fn news(s: &Snapshot, config: &Config, ctx: &Context) -> Vec<TopicView> {
    s.news
        .iter()
        .enumerate()
        .map(|(i, slot)| TopicView {
            title: config
                .topics
                .get(i)
                .map_or_else(String::new, |t| t.title.to_owned()),
            headlines: panel(slot, ctx, max_age::NEWS, |t| {
                t.headlines()
                    .iter()
                    .map(|h| headline_view(h, ctx))
                    .collect()
            }),
        })
        .collect()
}

fn headline_view(h: &Headline, ctx: &Context) -> HeadlineView {
    let age = match (h.published_at(), ctx.now) {
        (Some(p), Some(n)) => format::age(n.seconds() - p.seconds()),
        _ => String::new(),
    };
    HeadlineView {
        title: h.title().to_owned(),
        source: h.source().to_owned(),
        age,
    }
}

// --------------------------------------------------------------------- Finance

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Flat,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuoteView {
    pub price: String,
    pub change: String,
    pub change_pct: String,
    pub direction: Direction,
    /// "499.85 - 504.12".
    pub range: String,
    pub previous_close: f32,
    /// Prices sampled today, oldest first.
    pub history: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstrumentView {
    pub label: String,
    pub quote: Panel<QuoteView>,
}

pub fn finance(s: &Snapshot, config: &Config, ctx: &Context) -> Vec<InstrumentView> {
    let open = ctx.now.is_some_and(is_us_market_open);
    // Closed markets do not make quotes stale; errors still do.
    let limit = if open { max_age::QUOTE_OPEN } else { i64::MAX };
    s.quotes
        .iter()
        .enumerate()
        .map(|(i, q)| InstrumentView {
            label: config
                .instruments
                .get(i)
                .map_or_else(String::new, |x| x.label.to_owned()),
            quote: panel(&q.quote, ctx, limit, |quote| quote_view(quote, q)),
        })
        .collect()
}

fn quote_view(q: &Quote, slot: &QuoteSlot) -> QuoteView {
    QuoteView {
        price: format::price(q.price),
        change: format::change(q.change),
        change_pct: format::percent_change(q.change_pct),
        direction: if q.is_up() {
            Direction::Up
        } else if q.is_down() {
            Direction::Down
        } else {
            Direction::Flat
        },
        range: format!(
            "{} - {}",
            format::price(q.day_low),
            format::price(q.day_high)
        ),
        previous_close: q.previous_close,
        history: slot.history.iter().map(|p| p.price).collect(),
    }
}

// ------------------------------------------------------------------------ Home

#[derive(Clone, Debug, PartialEq)]
pub struct HomeView {
    pub weather: WeatherView,
    /// Top headline of each topic.
    pub headlines: Vec<(String, Panel<HeadlineView>)>,
    pub instruments: Vec<InstrumentView>,
}

pub fn home(s: &Snapshot, config: &Config, ctx: &Context) -> HomeView {
    HomeView {
        weather: weather(s, ctx),
        headlines: news(s, config, ctx)
            .into_iter()
            .map(|t| {
                let top = match t.headlines {
                    Panel::Fresh(v) => v.into_iter().next().map_or(Panel::Loading, Panel::Fresh),
                    Panel::Stale { data, age, error } => match data.into_iter().next() {
                        Some(h) => Panel::Stale {
                            data: h,
                            age,
                            error,
                        },
                        None => Panel::Loading,
                    },
                    Panel::Loading => Panel::Loading,
                    Panel::Failed(e) => Panel::Failed(e),
                };
                (t.title, top)
            })
            .collect(),
        instruments: finance(s, config, ctx),
    }
}

// ----------------------------------------------------------------------- Frame

/// Newest fetch time among the slots a page shows, as a clock time.
pub fn updated(s: &Snapshot, page: Page, ctx: &Context) -> Option<String> {
    let w = [s.current.fetched_at, s.forecast.fetched_at];
    let n = s.news.iter().map(|x| x.fetched_at);
    let q = s.quotes.iter().map(|x| x.quote.fetched_at);
    let newest = match page {
        Page::Weather => w.into_iter().flatten().max(),
        Page::News => n.flatten().max(),
        Page::Finance => q.flatten().max(),
        Page::Home => w.into_iter().chain(n).chain(q).flatten().max(),
    };
    newest.map(|t| format::clock(t, ctx.utc_offset, ctx.units))
}

/// Whether any data on a page is failing.
pub fn has_problem(s: &Snapshot, page: Page) -> bool {
    let w = s.current.has_error() || s.forecast.has_error();
    let n = s.news.iter().any(Slot::has_error);
    let q = s.quotes.iter().any(|x| x.quote.has_error());
    match page {
        Page::Weather => w,
        Page::News => n,
        Page::Finance => q,
        Page::Home => w || n || q,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dt_core::error::ErrorCode;
    use dt_core::model::NewsTopic;
    use dt_core::store::{Store, Update, UpdateKind};

    fn ctx(now: i64) -> Context {
        Context {
            now: Some(UnixTime(now)),
            utc_offset: -5 * 3600,
            units: Units::Imperial,
            location: "New York".into(),
            wifi_rssi: Some(-50),
        }
    }

    fn topic() -> NewsTopic {
        NewsTopic::new(
            "U.S.",
            [Headline::new("Headline", "AP", Some(UnixTime(1000)))],
        )
    }

    #[test]
    fn panel_states() {
        let mut s = Store::new(1, 0);
        let c = ctx(10_000);
        let p = |s: &Store| panel(&s.news[0], &c, max_age::NEWS, |t| t.headlines().len());
        assert_eq!(p(&s), Panel::Loading);

        s.apply(Update {
            at: UnixTime(9_000),
            kind: UpdateKind::News(0, Err(AppError::new(ErrorCode::ApiUnauthorized))),
        });
        assert!(matches!(p(&s), Panel::Failed(ErrorView { ref code, .. }) if code == "E-API-01"));

        s.apply(Update {
            at: UnixTime(9_500),
            kind: UpdateKind::News(0, Ok(topic())),
        });
        assert_eq!(p(&s), Panel::Fresh(1));

        s.apply(Update {
            at: UnixTime(9_900),
            kind: UpdateKind::News(0, Err(AppError::new(ErrorCode::NetTimeout))),
        });
        match p(&s) {
            Panel::Stale {
                data: 1,
                age,
                error: Some(e),
            } => {
                assert_eq!(age, "8 min ago");
                assert_eq!(e.code, "E-NET-04");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn old_data_is_stale_without_error() {
        let mut s = Store::new(1, 0);
        s.apply(Update {
            at: UnixTime(0),
            kind: UpdateKind::News(0, Ok(topic())),
        });
        let c = ctx(max_age::NEWS + 3600);
        assert!(matches!(
            panel(&s.news[0], &c, max_age::NEWS, |_| ()),
            Panel::Stale { error: None, .. }
        ));
    }

    #[test]
    fn headline_age_and_updated_time() {
        let mut s = Store::new(1, 0);
        s.apply(Update {
            at: UnixTime(1_794_924_309),
            kind: UpdateKind::News(0, Ok(topic())),
        });
        let c = ctx(1_794_924_309);
        let h = headline_view(&s.news[0].data.as_ref().unwrap().headlines()[0], &c);
        assert_eq!(h.age, "20774 d ago");
        assert_eq!(updated(&s, Page::News, &c).as_deref(), Some("9:05 AM"));
        assert_eq!(updated(&s, Page::Finance, &c), None);
        assert!(!has_problem(&s, Page::Home));
    }
}
