//! Drawing a panel in any of its states (REQ-ERR-03, REQ-ERR-04):
//! loading, failed (code, message and what to do), fresh, or stale (the data
//! with a marker showing its age and the error code, if any).

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, primitives::Rectangle};

use crate::icon::Icon;
use crate::layout::RectExt;
use crate::text::{Align, BODY, SMALL, SMALL_BOLD};
use crate::view::{ErrorView, Panel};

const ON: BinaryColor = BinaryColor::On;

/// Draw `panel` into `area`, calling `content` for its data when it has any.
pub fn draw_panel<D: DrawTarget<Color = BinaryColor>, T>(
    target: &mut D,
    area: Rectangle,
    panel: &Panel<T>,
    content: impl FnOnce(&mut D, Rectangle, &T),
) {
    match panel {
        Panel::Fresh(data) => content(target, area, data),
        Panel::Stale { data, age, error } => {
            content(target, area, data);
            stale_marker(target, area, age, error.as_ref());
        }
        Panel::Loading => loading(target, area),
        Panel::Failed(e) => failed(target, area, e),
    }
}

fn loading<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle) {
    let (x, y) = (area.top_left.x, area.top_left.y);
    Icon::Stale.draw(target, Point::new(x, y + 2), 16, ON);
    SMALL.draw(
        target,
        "Waiting for data...",
        x + 22,
        y + 15,
        Align::Left,
        ON,
    );
}

fn failed<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, e: &ErrorView) {
    let (x, y) = (area.top_left.x, area.top_left.y);
    let w = area.size.width as i32;
    if area.size.height < 72 {
        // Small panel: one line with the code, a second with the hint.
        Icon::Warning.draw(target, Point::new(x, y + 2), 16, ON);
        SMALL_BOLD.draw(target, &e.code, x + 22, y + 15, Align::Left, ON);
        SMALL.draw(
            target,
            &SMALL.truncate(&e.message, w),
            x,
            y + 33,
            Align::Left,
            ON,
        );
        if area.size.height >= 50 {
            SMALL.draw(
                target,
                &SMALL.truncate(&e.hint, w),
                x,
                y + 49,
                Align::Left,
                ON,
            );
        }
        return;
    }
    Icon::Warning.draw(target, Point::new(x, y + 2), 24, ON);
    SMALL_BOLD.draw(target, &e.code, x + 32, y + 19, Align::Left, ON);
    let lines_left = ((area.bottom() - y - 40) / 21).clamp(1, 3) as usize;
    let after = BODY.draw_wrapped(
        target,
        &e.message,
        Point::new(x, y + 46),
        w,
        lines_left.min(3),
        ON,
    );
    let hint_lines = ((area.bottom() - after) / 16).clamp(0, 3) as usize;
    if hint_lines > 0 {
        SMALL.draw_wrapped(target, &e.hint, Point::new(x, after), w, hint_lines, ON);
    }
}

/// Marker in the panel's title row, at the right: stale icon, error code if
/// any, and the data's age.
fn stale_marker<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    area: Rectangle,
    age: &str,
    error: Option<&ErrorView>,
) {
    let text = match error {
        Some(e) if age.is_empty() => e.code.clone(),
        Some(e) => format!("{}  {age}", e.code),
        None => age.to_owned(),
    };
    let right = area.right();
    let baseline = area.top_left.y - 14;
    let w = SMALL.width(&text);
    // Clear behind the marker so it reads over the panel rule.
    let bg = Rectangle::new(
        Point::new(right - w - 26, baseline - 13),
        Size::new((w + 26) as u32, 17),
    );
    let _ = embedded_graphics::primitives::Primitive::into_styled(
        bg,
        embedded_graphics::primitives::PrimitiveStyle::with_fill(BinaryColor::Off),
    )
    .draw(target);
    Icon::Stale.draw(target, Point::new(right - w - 22, baseline - 12), 16, ON);
    SMALL.draw(target, &text, right, baseline, Align::Right, ON);
}
