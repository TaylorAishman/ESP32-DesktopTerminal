//! The 1-bit frame buffer that pages draw into.

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

/// Panel width in pixels.
pub const WIDTH: u32 = 800;
/// Panel height in pixels.
pub const HEIGHT: u32 = 480;
/// Bytes per row: one bit per pixel.
pub const ROW_BYTES: usize = (WIDTH / 8) as usize;
/// Size of a full frame in bytes (48 000).
pub const FRAME_BYTES: usize = ROW_BYTES * HEIGHT as usize;

/// An 800×480 1-bit image in the panel's native layout.
///
/// Layout: rows top to bottom, 100 bytes per row, the most significant bit of
/// each byte is the leftmost pixel. A set bit is **black**
/// ([`BinaryColor::On`]); a clear bit is white. The display driver handles any
/// inversion the panel controller needs.
///
/// The buffer is allocated once and reused for every frame.
pub struct FrameBuffer {
    bytes: Box<[u8; FRAME_BYTES]>,
}

impl FrameBuffer {
    /// A new, all-white frame.
    pub fn new() -> Self {
        Self {
            bytes: Box::new([0; FRAME_BYTES]),
        }
    }

    /// Set every pixel to `color`.
    pub fn fill(&mut self, color: BinaryColor) {
        let byte = if color.is_on() { 0xFF } else { 0x00 };
        self.bytes.fill(byte);
    }

    /// The raw frame, ready to send to the display.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..]
    }

    /// Colour of one pixel. Out-of-range coordinates read as white.
    pub fn pixel(&self, x: u32, y: u32) -> BinaryColor {
        match Self::index(x, y) {
            Some((i, mask)) => BinaryColor::from(self.bytes[i] & mask != 0),
            None => BinaryColor::Off,
        }
    }

    /// A 64-bit FNV-1a hash of the frame, used to skip refreshes that would
    /// not change the panel.
    pub fn content_hash(&self) -> u64 {
        const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01b3;
        self.bytes
            .iter()
            .fold(OFFSET, |h, &b| (h ^ u64::from(b)).wrapping_mul(PRIME))
    }

    fn index(x: u32, y: u32) -> Option<(usize, u8)> {
        if x >= WIDTH || y >= HEIGHT {
            return None;
        }
        let i = y as usize * ROW_BYTES + (x / 8) as usize;
        Some((i, 0x80 >> (x % 8)))
    }

    fn set(&mut self, x: u32, y: u32, color: BinaryColor) {
        if let Some((i, mask)) = Self::index(x, y) {
            if color.is_on() {
                self.bytes[i] |= mask;
            } else {
                self.bytes[i] &= !mask;
            }
        }
    }
}

impl Default for FrameBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl OriginDimensions for FrameBuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for FrameBuffer {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(p, color) in pixels {
            if p.x >= 0 && p.y >= 0 {
                self.set(p.x as u32, p.y as u32, color);
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.fill(color);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};

    #[test]
    fn frame_is_48000_bytes() {
        assert_eq!(FrameBuffer::new().as_bytes().len(), 48_000);
    }

    #[test]
    fn corner_pixels_map_to_expected_bits() {
        let mut fb = FrameBuffer::new();
        for (x, y) in [(0, 0), (799, 0), (0, 479), (799, 479)] {
            Pixel(Point::new(x, y), BinaryColor::On)
                .draw(&mut fb)
                .unwrap();
        }
        let b = fb.as_bytes();
        assert_eq!(b[0], 0b1000_0000);
        assert_eq!(b[99], 0b0000_0001);
        assert_eq!(b[479 * 100], 0b1000_0000);
        assert_eq!(b[FRAME_BYTES - 1], 0b0000_0001);
        assert_eq!(b.iter().map(|x| x.count_ones()).sum::<u32>(), 4);
        assert_eq!(fb.pixel(799, 479), BinaryColor::On);
        assert_eq!(fb.pixel(798, 479), BinaryColor::Off);
    }

    #[test]
    fn out_of_range_drawing_is_ignored() {
        let mut fb = FrameBuffer::new();
        Rectangle::new(Point::new(-10, -10), Size::new(5, 5))
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(&mut fb)
            .unwrap();
        Pixel(Point::new(800, 0), BinaryColor::On)
            .draw(&mut fb)
            .unwrap();
        assert!(fb.as_bytes().iter().all(|&b| b == 0));
        assert_eq!(fb.pixel(9999, 0), BinaryColor::Off);
    }

    #[test]
    fn hash_changes_with_content() {
        let mut fb = FrameBuffer::new();
        let blank = fb.content_hash();
        Pixel(Point::new(400, 240), BinaryColor::On)
            .draw(&mut fb)
            .unwrap();
        let one = fb.content_hash();
        assert_ne!(blank, one);
        Pixel(Point::new(400, 240), BinaryColor::Off)
            .draw(&mut fb)
            .unwrap();
        assert_eq!(fb.content_hash(), blank);
    }

    #[test]
    fn fill_and_clear() {
        let mut fb = FrameBuffer::new();
        fb.clear(BinaryColor::On).unwrap();
        assert!(fb.as_bytes().iter().all(|&b| b == 0xFF));
        fb.fill(BinaryColor::Off);
        assert!(fb.as_bytes().iter().all(|&b| b == 0));
    }
}
