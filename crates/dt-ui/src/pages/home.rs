//! The Home page: highlights from the other three (REQ-HOME-01/02).

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, primitives::Rectangle};

use crate::chart::sparkline;
use crate::layout::{titled_panel, vline, RectExt, GAP};
use crate::pages::finance::direction_icon;
use crate::panel::draw_panel;
use crate::text::{Align, BODY, BODY_BOLD, DISPLAY, HERO, SMALL, SMALL_BOLD};
use crate::view::{CurrentView, HomeView, Panel, QuoteView, WeatherView};

const ON: BinaryColor = BinaryColor::On;

pub fn draw<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, v: &HomeView) {
    let (bottom, top) = area.split_bottom(112);
    let cols = top.split_columns_weighted(&[5, 8], 3 * GAP);
    vline(
        target,
        cols[0].right() + 3 * GAP as i32 / 2,
        cols[0].top_left.y,
        cols[0].bottom() - GAP as i32,
        1,
    );
    weather(target, cols[0], &v.weather);
    headlines(target, cols[1], v);
    markets(target, bottom, v);
}

fn weather<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, w: &WeatherView) {
    let body = titled_panel(target, area, &w.location);
    draw_panel(target, body, &w.current, |t, a, c| now(t, a, c, w));
}

fn now<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    body: Rectangle,
    c: &CurrentView,
    w: &WeatherView,
) {
    let (x, y) = (body.top_left.x, body.top_left.y);
    c.icon.draw(target, Point::new(x, y + 2), 64, ON);
    HERO.draw(target, &c.temperature, x + 76, y + 50, Align::Left, ON);
    let condition = BODY_BOLD.truncate(&c.condition, body.size.width as i32);
    BODY_BOLD.draw(target, &condition, x, y + 92, Align::Left, ON);
    if let Some(t) = &w.today {
        BODY.draw(target, t, x, y + 116, Align::Left, ON);
    }
    // The next few hours as a compact row.
    if let Some(hours) = w.hourly.data() {
        let row = Rectangle::new(Point::new(x, y + 128), Size::new(body.size.width, 64));
        for (h, col) in hours.iter().take(4).zip(row.split_columns(4, GAP)) {
            let cx = col.top_left.x + col.size.width as i32 / 2;
            SMALL.draw(target, &h.time, cx, col.top_left.y + 12, Align::Center, ON);
            h.icon
                .draw_centered(target, Point::new(cx, col.top_left.y + 30), 24, ON);
            SMALL_BOLD.draw(
                target,
                &h.temperature,
                cx,
                col.top_left.y + 58,
                Align::Center,
                ON,
            );
        }
    }
}

fn headlines<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, v: &HomeView) {
    let body = titled_panel(target, area, "Top headlines");
    let w = body.size.width as i32;
    let n = v.headlines.len().max(1) as i32;
    let step = (body.size.height as i32 / n).clamp(40, 80);
    let mut y = body.top_left.y + 14;
    for (topic, panel) in &v.headlines {
        SMALL_BOLD.draw(
            target,
            &topic.to_uppercase(),
            body.top_left.x,
            y,
            Align::Left,
            ON,
        );
        let text = match panel {
            Panel::Fresh(h) | Panel::Stale { data: h, .. } => h.title.clone(),
            Panel::Loading => "Waiting for data...".into(),
            Panel::Failed(e) => format!("{}: {}", e.code, e.message),
        };
        let lines = if step >= 72 { 2 } else { 1 };
        BODY.draw_wrapped(
            target,
            &text,
            Point::new(body.top_left.x, y + 20),
            w,
            lines,
            ON,
        );
        y += step;
    }
}

fn markets<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, v: &HomeView) {
    let body = titled_panel(target, area, "Markets");
    let n = v.instruments.len().max(1);
    for (inst, col) in v.instruments.iter().zip(body.split_columns(n, 2 * GAP)) {
        let label = SMALL_BOLD.truncate(&inst.label, col.size.width as i32);
        SMALL_BOLD.draw(
            target,
            &label,
            col.top_left.x,
            col.top_left.y + 12,
            Align::Left,
            ON,
        );
        // Content starts below the label row, where a stale marker can sit.
        draw_panel(target, col.split_top(30).1, &inst.quote, |t, a, q| {
            market(t, a, q)
        });
    }
}

fn market<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, q: &QuoteView) {
    let (x, y) = (area.top_left.x, area.top_left.y);
    let (chart, text) = area.split_right(area.size.width / 3);
    let price = DISPLAY.truncate(&q.price, text.size.width as i32);
    DISPLAY.draw(target, &price, x, y + 16, Align::Left, ON);
    direction_icon(q.direction).draw(target, Point::new(x, y + 26), 16, ON);
    SMALL_BOLD.draw(target, &q.change_pct, x + 20, y + 40, Align::Left, ON);
    let chart = Rectangle::new(
        chart.top_left + Point::new(GAP as i32 + 4, -8),
        Size::new(chart.size.width.saturating_sub(GAP + 4), 44),
    );
    sparkline(target, chart, &q.history, Some(q.previous_close));
}
