//! Layout geometry: splitting rectangles into rows and columns, dividers and
//! titled panels. Pages are built from these so spacing stays consistent.

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Rectangle},
};

use crate::text::{Align, SMALL_BOLD};

/// Standard spacing unit, in pixels.
pub const GAP: u32 = 8;

/// Rectangle helpers. All splits clamp instead of panicking, so a layout
/// that does not fit degrades rather than crashes.
pub trait RectExt: Sized {
    fn inset(&self, dx: u32, dy: u32) -> Self;
    /// Top `h` pixels, and the rest below.
    fn split_top(&self, h: u32) -> (Self, Self);
    /// Bottom `h` pixels, and the rest above.
    fn split_bottom(&self, h: u32) -> (Self, Self);
    /// Left `w` pixels, and the rest to the right.
    fn split_left(&self, w: u32) -> (Self, Self);
    /// Right `w` pixels, and the rest to the left.
    fn split_right(&self, w: u32) -> (Self, Self);
    /// `n` equal columns separated by `gap`.
    fn split_columns(&self, n: usize, gap: u32) -> Vec<Self>;
    /// `n` equal rows separated by `gap`.
    fn split_rows(&self, n: usize, gap: u32) -> Vec<Self>;
    /// Columns whose widths are proportional to `weights`.
    fn split_columns_weighted(&self, weights: &[u32], gap: u32) -> Vec<Self>;
    fn right(&self) -> i32;
    fn bottom(&self) -> i32;
}

impl RectExt for Rectangle {
    fn inset(&self, dx: u32, dy: u32) -> Self {
        let w = self.size.width.saturating_sub(2 * dx);
        let h = self.size.height.saturating_sub(2 * dy);
        Rectangle::new(
            self.top_left + Point::new(dx as i32, dy as i32),
            Size::new(w, h),
        )
    }

    fn split_top(&self, h: u32) -> (Self, Self) {
        let h = h.min(self.size.height);
        (
            Rectangle::new(self.top_left, Size::new(self.size.width, h)),
            Rectangle::new(
                self.top_left + Point::new(0, h as i32),
                Size::new(self.size.width, self.size.height - h),
            ),
        )
    }

    fn split_bottom(&self, h: u32) -> (Self, Self) {
        let h = h.min(self.size.height);
        let (rest, bottom) = self.split_top(self.size.height - h);
        (bottom, rest)
    }

    fn split_left(&self, w: u32) -> (Self, Self) {
        let w = w.min(self.size.width);
        (
            Rectangle::new(self.top_left, Size::new(w, self.size.height)),
            Rectangle::new(
                self.top_left + Point::new(w as i32, 0),
                Size::new(self.size.width - w, self.size.height),
            ),
        )
    }

    fn split_right(&self, w: u32) -> (Self, Self) {
        let w = w.min(self.size.width);
        let (rest, right) = self.split_left(self.size.width - w);
        (right, rest)
    }

    fn split_columns(&self, n: usize, gap: u32) -> Vec<Self> {
        self.split_columns_weighted(&vec![1; n], gap)
    }

    fn split_rows(&self, n: usize, gap: u32) -> Vec<Self> {
        if n == 0 {
            return Vec::new();
        }
        let total_gap = gap * (n as u32 - 1);
        let avail = self.size.height.saturating_sub(total_gap);
        let mut y = self.top_left.y;
        (0..n)
            .map(|i| {
                // Distribute the remainder so rows fill the height exactly.
                let h = avail * (i as u32 + 1) / n as u32 - avail * i as u32 / n as u32;
                let r = Rectangle::new(
                    Point::new(self.top_left.x, y),
                    Size::new(self.size.width, h),
                );
                y += (h + gap) as i32;
                r
            })
            .collect()
    }

    fn split_columns_weighted(&self, weights: &[u32], gap: u32) -> Vec<Self> {
        let n = weights.len();
        if n == 0 {
            return Vec::new();
        }
        let total: u32 = weights.iter().sum::<u32>().max(1);
        let avail = self.size.width.saturating_sub(gap * (n as u32 - 1));
        let mut x = self.top_left.x;
        let mut acc = 0;
        weights
            .iter()
            .map(|&w| {
                let start = avail * acc / total;
                acc += w;
                let width = avail * acc / total - start;
                let r = Rectangle::new(
                    Point::new(x, self.top_left.y),
                    Size::new(width, self.size.height),
                );
                x += (width + gap) as i32;
                r
            })
            .collect()
    }

    fn right(&self) -> i32 {
        self.top_left.x + self.size.width as i32
    }

    fn bottom(&self) -> i32 {
        self.top_left.y + self.size.height as i32
    }
}

/// A horizontal line across `r` at `y`.
pub fn hline<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    x0: i32,
    x1: i32,
    y: i32,
    width: u32,
) {
    let _ = Line::new(Point::new(x0, y), Point::new(x1, y))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, width))
        .draw(target);
}

/// A vertical line at `x`.
pub fn vline<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    x: i32,
    y0: i32,
    y1: i32,
    width: u32,
) {
    let _ = Line::new(Point::new(x, y0), Point::new(x, y1))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, width))
        .draw(target);
}

/// Height of a panel title row.
pub const PANEL_TITLE_HEIGHT: u32 = 24;

/// Draw a panel title (small bold capitals over a rule) at the top of
/// `area`; returns the area left for the panel's content.
pub fn titled_panel<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    area: Rectangle,
    title: &str,
) -> Rectangle {
    let (head, body) = area.split_top(PANEL_TITLE_HEIGHT);
    let caps = title.to_uppercase();
    let label = SMALL_BOLD.truncate(&caps, head.size.width as i32);
    SMALL_BOLD.draw(
        target,
        &label,
        head.top_left.x,
        head.top_left.y + 14,
        Align::Left,
        BinaryColor::On,
    );
    hline(
        target,
        head.top_left.x,
        head.right() - 1,
        head.top_left.y + 19,
        1,
    );
    body.split_top(GAP / 2).1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: i32, y: i32, w: u32, h: u32) -> Rectangle {
        Rectangle::new(Point::new(x, y), Size::new(w, h))
    }

    #[test]
    fn splits() {
        let a = r(10, 20, 100, 50);
        assert_eq!(a.split_top(10), (r(10, 20, 100, 10), r(10, 30, 100, 40)));
        assert_eq!(a.split_bottom(10), (r(10, 60, 100, 10), r(10, 20, 100, 40)));
        assert_eq!(a.split_left(30), (r(10, 20, 30, 50), r(40, 20, 70, 50)));
        assert_eq!(a.split_right(30), (r(80, 20, 30, 50), r(10, 20, 70, 50)));
        assert_eq!(a.inset(5, 2), r(15, 22, 90, 46));
        // Over-large splits clamp.
        assert_eq!(a.split_top(500).1.size.height, 0);
        assert_eq!(a.inset(80, 80).size, Size::zero());
    }

    #[test]
    fn columns_fill_exactly() {
        let a = r(0, 0, 800, 10);
        let cols = a.split_columns(3, 8);
        assert_eq!(cols.len(), 3);
        let widths: u32 = cols.iter().map(|c| c.size.width).sum();
        assert_eq!(widths + 16, 800);
        assert_eq!(cols[2].right(), 800);
        assert_eq!(cols[1].top_left.x, cols[0].right() + 8);
    }

    #[test]
    fn weighted_columns() {
        let cols = r(0, 0, 308, 10).split_columns_weighted(&[2, 1], 8);
        assert_eq!(cols[0].size.width, 200);
        assert_eq!(cols[1].size.width, 100);
        assert_eq!(cols[1].top_left.x, 208);
    }

    #[test]
    fn rows_fill_exactly() {
        let rows = r(0, 0, 10, 101).split_rows(4, 3);
        assert_eq!(rows.last().unwrap().bottom(), 101);
        assert!(rows
            .windows(2)
            .all(|w| w[1].top_left.y == w[0].bottom() + 3));
        assert!(r(0, 0, 1, 1).split_rows(0, 3).is_empty());
    }
}
