//! Drawing panels that have no data yet, failed, or are out of date.

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, primitives::Rectangle};

use crate::icon::Icon;
use crate::layout::RectExt;
use crate::text::{Align, SMALL};
use crate::view::Panel;

const ON: BinaryColor = BinaryColor::On;

/// Placeholder for a panel without data.
pub fn draw_panel_state<D: DrawTarget<Color = BinaryColor>, T>(
    target: &mut D,
    area: Rectangle,
    panel: &Panel<T>,
) {
    if let Panel::Loading | Panel::Failed(_) = panel {
        SMALL.draw(
            target,
            "Waiting for data...",
            area.top_left.x,
            area.top_left.y + 16,
            Align::Left,
            ON,
        );
    }
}

/// Small "stale" marker with the data's age in the panel's top-right corner.
pub fn stale_note<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, age: &str) {
    let x = area.right();
    let w = SMALL.width(age);
    SMALL.draw(target, age, x, area.top_left.y - 10, Align::Right, ON);
    Icon::Stale.draw(target, Point::new(x - w - 20, area.top_left.y - 24), 16, ON);
}
