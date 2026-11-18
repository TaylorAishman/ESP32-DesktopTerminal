//! Icons rasterised at build time from `assets/icons/*.svg`.

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

/// One icon at one size: `size`×`size` pixels, rows padded to whole bytes,
/// most significant bit first, set bit = ink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bitmap {
    pub size: u32,
    pub bits: &'static [u8],
}

impl Bitmap {
    fn is_set(&self, x: u32, y: u32) -> bool {
        let row = self.size.div_ceil(8);
        self.bits[(y * row + x / 8) as usize] & (0x80 >> (x % 8)) != 0
    }
}

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

impl Icon {
    /// The largest rendered size not bigger than `size` (or the smallest).
    pub fn bitmap(self, size: u32) -> Bitmap {
        let all = self.bitmaps();
        all.iter()
            .rev()
            .find(|b| b.size <= size)
            .copied()
            .unwrap_or(all[0])
    }

    /// Draw with the top-left corner at `top_left`. Only ink pixels are
    /// drawn; the background is left as it is.
    pub fn draw<D: DrawTarget<Color = BinaryColor>>(
        self,
        target: &mut D,
        top_left: Point,
        size: u32,
        ink: BinaryColor,
    ) {
        let b = self.bitmap(size);
        let pixels = (0..b.size)
            .flat_map(move |y| (0..b.size).map(move |x| (x, y)))
            .filter(|&(x, y)| b.is_set(x, y))
            .map(|(x, y)| Pixel(top_left + Point::new(x as i32, y as i32), ink));
        let _ = target.draw_iter(pixels);
    }

    /// Draw centred on `center`.
    pub fn draw_centered<D: DrawTarget<Color = BinaryColor>>(
        self,
        target: &mut D,
        center: Point,
        size: u32,
        ink: BinaryColor,
    ) {
        let s = self.bitmap(size).size as i32;
        self.draw(target, center - Point::new(s / 2, s / 2), size, ink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_has_every_size_with_ink() {
        for icon in Icon::ALL {
            for size in SIZES {
                let b = icon.bitmap(size);
                assert_eq!(b.size, size);
                assert_eq!(b.bits.len(), (size.div_ceil(8) * size) as usize);
                assert!(
                    b.bits.iter().any(|&x| x != 0),
                    "{icon:?} at {size} px is blank"
                );
            }
        }
    }

    #[test]
    fn nearest_smaller_size() {
        assert_eq!(Icon::ClearDay.bitmap(40).size, 32);
        assert_eq!(Icon::ClearDay.bitmap(500).size, 96);
        assert_eq!(Icon::ClearDay.bitmap(4).size, 16);
    }
}
