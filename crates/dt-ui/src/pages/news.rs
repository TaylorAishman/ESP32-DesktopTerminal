//! The News page: one panel per configured topic.

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, primitives::Rectangle};

use crate::layout::{titled_panel, RectExt, GAP};
use crate::panel::{draw_panel_state, stale_note};
use crate::text::{Align, BODY, SMALL};
use crate::view::{HeadlineView, Panel, TopicView};

const ON: BinaryColor = BinaryColor::On;
/// Height of one headline: two title lines, the source line and spacing.
const HEADLINE_HEIGHT: i32 = 2 * 21 + 18 + 14;

pub fn draw<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    area: Rectangle,
    topics: &[TopicView],
) {
    for (topic, cell) in topics.iter().zip(grid(area, topics.len())) {
        let body = titled_panel(target, cell, &topic.title);
        match &topic.headlines {
            Panel::Fresh(h) => headlines(target, body, h),
            Panel::Stale { data, age, .. } => {
                headlines(target, body, data);
                stale_note(target, body, age);
            }
            other => draw_panel_state(target, body, other),
        }
    }
}

/// Cells for `n` panels: two columns (one if there is a single topic), as
/// many rows as needed.
pub fn grid(area: Rectangle, n: usize) -> Vec<Rectangle> {
    if n == 0 {
        return Vec::new();
    }
    let cols = if n == 1 { 1 } else { 2 };
    let rows = n.div_ceil(cols);
    area.split_rows(rows, 2 * GAP)
        .into_iter()
        .flat_map(|r| r.split_columns(cols, 3 * GAP))
        .take(n)
        .collect()
}

fn headlines<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    area: Rectangle,
    items: &[HeadlineView],
) {
    let fit = ((area.size.height as i32 + 10) / HEADLINE_HEIGHT).max(1) as usize;
    let w = area.size.width as i32;
    let mut y = area.top_left.y + 16;
    for h in items.iter().take(fit) {
        let after = BODY.draw_wrapped(target, &h.title, Point::new(area.top_left.x, y), w, 2, ON);
        let meta = if h.age.is_empty() {
            h.source.clone()
        } else {
            format!("{}  ·  {}", h.source, h.age)
        };
        SMALL.draw(
            target,
            &SMALL.truncate(&meta, w),
            area.top_left.x,
            after - 2,
            Align::Left,
            ON,
        );
        y = after + 30;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_shapes() {
        let a = Rectangle::new(Point::zero(), Size::new(768, 368));
        assert_eq!(grid(a, 1).len(), 1);
        assert_eq!(grid(a, 1)[0].size.width, 768);
        let four = grid(a, 4);
        assert_eq!(four.len(), 4);
        assert_eq!(four[1].top_left.y, four[0].top_left.y);
        assert!(four[2].top_left.y > four[0].top_left.y);
        assert_eq!(grid(a, 5).len(), 5);
    }
}
