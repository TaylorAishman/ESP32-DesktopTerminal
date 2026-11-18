//! The Weather page: current conditions, the next 24 hours, five days.

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, primitives::Rectangle};

use crate::chart::line_chart;
use crate::icon::Icon;
use crate::layout::{titled_panel, vline, RectExt, GAP};
use crate::panel::{draw_panel_state, stale_note};
use crate::text::{Align, BODY, BODY_BOLD, HERO, SMALL, TITLE};
use crate::view::{CurrentView, DayView, HourView, Panel, WeatherView};

const ON: BinaryColor = BinaryColor::On;

pub fn draw<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, v: &WeatherView) {
    let cols = area.split_columns_weighted(&[5, 8], 3 * GAP);
    let (left, right) = (cols[0], cols[1]);
    vline(
        target,
        left.right() + 3 * GAP as i32 / 2,
        left.top_left.y,
        left.bottom(),
        1,
    );

    let body = titled_panel(target, left, &v.location);
    match &v.current {
        Panel::Fresh(c) => current(target, body, c, v.today.as_deref()),
        Panel::Stale { data, age, .. } => {
            current(target, body, data, v.today.as_deref());
            stale_note(target, body, age);
        }
        other => draw_panel_state(target, body, other),
    }

    let (top, bottom) = right.split_top(right.size.height * 11 / 20);
    let body = titled_panel(target, top, "Next 24 hours");
    match &v.hourly {
        Panel::Fresh(h) => hourly(target, body, h),
        Panel::Stale { data, age, .. } => {
            hourly(target, body, data);
            stale_note(target, body, age);
        }
        other => draw_panel_state(target, body, other),
    }
    let body = titled_panel(target, bottom.split_top(GAP).1, "5-day forecast");
    match &v.daily {
        Panel::Fresh(d) => daily(target, body, d),
        Panel::Stale { data, .. } => daily(target, body, data),
        other => draw_panel_state(target, body, other),
    }
}

fn current<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    area: Rectangle,
    c: &CurrentView,
    today: Option<&str>,
) {
    let x = area.top_left.x;
    let y = area.top_left.y;
    c.icon.draw(target, Point::new(x, y + 4), 96, ON);
    HERO.draw(target, &c.temperature, x + 112, y + 58, Align::Left, ON);
    BODY.draw(target, &c.feels_like, x + 114, y + 86, Align::Left, ON);

    let mut line = y + 132;
    TITLE.draw(
        target,
        &TITLE.truncate(&c.condition, area.size.width as i32),
        x,
        line,
        Align::Left,
        ON,
    );
    if let Some(t) = today {
        line += 28;
        BODY_BOLD.draw(target, t, x, line, Align::Left, ON);
    }

    // Details: two columns of icon + value.
    let grid = Rectangle::new(
        Point::new(x, line + 18),
        Size::new(area.size.width, (area.bottom() - line - 18).max(0) as u32),
    );
    let rows = grid.split_rows(2, GAP);
    let items = [
        (Icon::Humidity, c.humidity.as_str(), "Humidity"),
        (Icon::Wind, c.wind.as_str(), "Wind"),
        (Icon::Sunrise, c.sunrise.as_str(), "Sunrise"),
        (Icon::Sunset, c.sunset.as_str(), "Sunset"),
    ];
    for (i, (icon, value, label)) in items.iter().enumerate() {
        let cell = rows[i / 2].split_columns(2, GAP)[i % 2];
        icon.draw(target, cell.top_left + Point::new(0, 2), 32, ON);
        let tx = cell.top_left.x + 40;
        let w = cell.right() - tx;
        BODY_BOLD.draw(
            target,
            &BODY_BOLD.truncate(value, w),
            tx,
            cell.top_left.y + 16,
            Align::Left,
            ON,
        );
        SMALL.draw(target, label, tx, cell.top_left.y + 33, Align::Left, ON);
    }
}

fn hourly<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, hours: &[HourView]) {
    if hours.is_empty() {
        return;
    }
    let cols = area.split_columns(hours.len(), 0);
    let col_w = cols[0].size.width;
    for (h, col) in hours.iter().zip(&cols) {
        let cx = col.top_left.x + col.size.width as i32 / 2;
        SMALL.draw(target, &h.time, cx, col.top_left.y + 12, Align::Center, ON);
        h.icon
            .draw_centered(target, Point::new(cx, col.top_left.y + 34), 32, ON);
        SMALL.draw(
            target,
            &h.chance_text,
            cx,
            col.bottom() - 2,
            Align::Center,
            ON,
        );
    }
    // Chart points line up with the column centres.
    let chart = Rectangle::new(
        Point::new(area.top_left.x + col_w as i32 / 2, area.top_left.y + 56),
        Size::new(
            area.size.width - col_w,
            area.size.height.saturating_sub(56 + 22),
        ),
    );
    let temps: Vec<f32> = hours.iter().map(|h| h.temperature_value).collect();
    let pts = line_chart(target, chart, &temps, 20);
    for (p, h) in pts.iter().zip(hours) {
        SMALL.draw(target, &h.temperature, p.x, p.y - 8, Align::Center, ON);
    }
}

fn daily<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, days: &[DayView]) {
    if days.is_empty() {
        return;
    }
    for (d, col) in days.iter().zip(area.split_columns(days.len(), GAP)) {
        let cx = col.top_left.x + col.size.width as i32 / 2;
        let y = col.top_left.y;
        BODY_BOLD.draw(target, &d.name, cx, y + 16, Align::Center, ON);
        d.icon.draw_centered(target, Point::new(cx, y + 46), 48, ON);
        BODY.draw(
            target,
            &format!("{} / {}", d.high, d.low),
            cx,
            y + 88,
            Align::Center,
            ON,
        );
        SMALL.draw(target, &d.chance, cx, y + 106, Align::Center, ON);
    }
}
