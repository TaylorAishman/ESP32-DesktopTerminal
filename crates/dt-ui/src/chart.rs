//! Small charts: a sparkline for price history, a line chart for the hourly
//! forecast, and bars for precipitation probability.

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Line, Polyline, PrimitiveStyle, Rectangle},
};

use crate::layout::RectExt;

const ON: BinaryColor = BinaryColor::On;

/// Map `values` to points inside `area`, scaled to their own range (or to
/// `range` if given). Flat data sits on the middle line.
fn plot(area: Rectangle, values: &[f32], range: Option<(f32, f32)>) -> Vec<Point> {
    let n = values.len();
    let (lo, hi) = range.unwrap_or_else(|| {
        values
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &v| {
                (a.min(v), b.max(v))
            })
    });
    let span = hi - lo;
    let w = area.size.width.saturating_sub(1) as f32;
    let h = area.size.height.saturating_sub(1) as f32;
    values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = if n > 1 {
                i as f32 * w / (n - 1) as f32
            } else {
                w / 2.0
            };
            let t = if span > f32::EPSILON {
                (v - lo) / span
            } else {
                0.5
            };
            Point::new(
                area.top_left.x + x.round() as i32,
                area.top_left.y + (h - t * h).round() as i32,
            )
        })
        .collect()
}

/// A line through `values`, scaled to fill `area`. With `reference`, a
/// dashed line marks that value (e.g. the previous close) and the scale
/// includes it. Fewer than two values draw a short dash in the middle.
pub fn sparkline<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    area: Rectangle,
    values: &[f32],
    reference: Option<f32>,
) {
    if values.len() < 2 {
        let c = area.center();
        let _ = Line::new(c - Point::new(6, 0), c + Point::new(6, 0))
            .into_styled(PrimitiveStyle::with_stroke(ON, 1))
            .draw(target);
        return;
    }
    let (mut lo, mut hi) = values
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &v| {
            (a.min(v), b.max(v))
        });
    if let Some(r) = reference {
        lo = lo.min(r);
        hi = hi.max(r);
    }
    let pts = plot(area, values, Some((lo, hi)));
    if let Some(r) = reference {
        let y = plot(area, &[r], Some((lo, hi)))[0].y;
        dashed_hline(target, area.top_left.x, area.right() - 1, y);
    }
    let _ = Polyline::new(&pts)
        .into_styled(PrimitiveStyle::with_stroke(ON, 2))
        .draw(target);
    // Mark the latest value.
    if let Some(&last) = pts.last() {
        let _ = Circle::with_center(last, 5)
            .into_styled(PrimitiveStyle::with_fill(ON))
            .draw(target);
    }
}

/// A line chart with a dot on each value, scaled with `padding` pixels of
/// headroom above and below so labels can go there. Returns the points, so
/// callers can place labels.
pub fn line_chart<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    area: Rectangle,
    values: &[f32],
    padding: u32,
) -> Vec<Point> {
    let inner = area.inset(0, padding);
    let pts = plot(inner, values, None);
    if pts.len() >= 2 {
        let _ = Polyline::new(&pts)
            .into_styled(PrimitiveStyle::with_stroke(ON, 2))
            .draw(target);
    }
    for &p in &pts {
        let _ = Circle::with_center(p, 6)
            .into_styled(PrimitiveStyle::with_fill(ON))
            .draw(target);
    }
    pts
}

/// Vertical bars for fractions 0..=1 (e.g. chance of rain), one per slot
/// across `area`, growing up from its bottom edge. Zero draws nothing.
pub fn bars<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    area: Rectangle,
    fractions: &[f32],
    bar_width: u32,
) {
    let n = fractions.len();
    if n == 0 {
        return;
    }
    let slot = area.size.width as f32 / n as f32;
    for (i, &f) in fractions.iter().enumerate() {
        let h = (f.clamp(0.0, 1.0) * area.size.height as f32).round() as u32;
        if h == 0 {
            continue;
        }
        let cx = area.top_left.x + (slot * (i as f32 + 0.5)) as i32;
        let r = Rectangle::new(
            Point::new(cx - bar_width as i32 / 2, area.bottom() - h as i32),
            Size::new(bar_width, h),
        );
        let _ = r.into_styled(PrimitiveStyle::with_fill(ON)).draw(target);
    }
}

fn dashed_hline<D: DrawTarget<Color = BinaryColor>>(target: &mut D, x0: i32, x1: i32, y: i32) {
    let pixels = (x0..=x1)
        .filter(|x| (x - x0) % 6 < 3)
        .map(|x| Pixel(Point::new(x, y), ON));
    let _ = target.draw_iter(pixels);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r() -> Rectangle {
        Rectangle::new(Point::new(10, 10), Size::new(101, 51))
    }

    #[test]
    fn plot_scales_to_the_box() {
        let p = plot(r(), &[1.0, 3.0, 2.0], None);
        assert_eq!(
            p,
            [Point::new(10, 60), Point::new(60, 10), Point::new(110, 35)]
        );
    }

    #[test]
    fn flat_data_is_centred() {
        let p = plot(r(), &[5.0, 5.0], None);
        assert!(p.iter().all(|q| q.y == 35));
    }
}
