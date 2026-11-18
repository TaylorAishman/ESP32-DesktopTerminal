//! The Finance page: one card per configured instrument.

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, primitives::Rectangle};

use crate::chart::sparkline;
use crate::icon::Icon;
use crate::layout::{titled_panel, RectExt, GAP};
use crate::pages::news::grid;
use crate::panel::{draw_panel_state, stale_note};
use crate::text::{Align, BODY_BOLD, DISPLAY, SMALL};
use crate::view::{Direction, InstrumentView, Panel, QuoteView};

const ON: BinaryColor = BinaryColor::On;

pub fn draw<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    area: Rectangle,
    instruments: &[InstrumentView],
) {
    for (inst, cell) in instruments.iter().zip(grid(area, instruments.len())) {
        let body = titled_panel(target, cell, &inst.label);
        match &inst.quote {
            Panel::Fresh(q) => card(target, body, q),
            Panel::Stale { data, age, .. } => {
                card(target, body, data);
                stale_note(target, body, age);
            }
            other => draw_panel_state(target, body, other),
        }
    }
}

pub fn direction_icon(d: Direction) -> Icon {
    match d {
        Direction::Up => Icon::TrendUp,
        Direction::Down => Icon::TrendDown,
        Direction::Flat => Icon::TrendFlat,
    }
}

fn card<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, q: &QuoteView) {
    let cols = area.split_columns_weighted(&[9, 10], 2 * GAP);
    let (text, chart) = (cols[0], cols[1]);
    let x = text.top_left.x;
    let y = text.top_left.y;
    DISPLAY.draw(
        target,
        &DISPLAY.truncate(&q.price, text.size.width as i32),
        x,
        y + 30,
        Align::Left,
        ON,
    );
    direction_icon(q.direction).draw(target, Point::new(x, y + 44), 16, ON);
    BODY_BOLD.draw(
        target,
        &format!("{}  {}", q.change, q.change_pct),
        x + 22,
        y + 59,
        Align::Left,
        ON,
    );
    SMALL.draw(target, "Day range", x, y + 86, Align::Left, ON);
    SMALL.draw(target, &q.range, x, y + 102, Align::Left, ON);

    let chart = Rectangle::new(
        chart.top_left + Point::new(0, 6),
        Size::new(
            chart.size.width,
            96.min(chart.size.height.saturating_sub(12)),
        ),
    );
    sparkline(target, chart, &q.history, Some(q.previous_close));
    if q.history.len() >= 2 {
        SMALL.draw(
            target,
            "Today",
            chart.top_left.x,
            chart.bottom() + 14,
            Align::Left,
            ON,
        );
        SMALL.draw(
            target,
            "- - prev. close",
            chart.right(),
            chart.bottom() + 14,
            Align::Right,
            ON,
        );
    }
}
