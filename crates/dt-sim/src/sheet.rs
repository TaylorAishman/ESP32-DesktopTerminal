//! Contact sheets for reviewing icons.

use dt_ui::icon::{Icon, SIZES};
use dt_ui::text::{Align, SMALL};
use dt_ui::FrameBuffer;
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

/// Every size of each icon on one row, labelled; continues in a second
/// column when the first is full. Fits eight icons.
pub fn contact_sheet(icons: &[Icon], sizes: &[u32]) -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    let row_height = sizes.iter().copied().max().unwrap_or(48) as i32 + 8;
    let (mut x, mut y) = (8, 8);
    for icon in icons {
        let mut cx = x + 110;
        for &size in sizes {
            icon.draw(&mut fb, Point::new(cx, y), size, BinaryColor::On);
            cx += size as i32 + 8;
        }
        SMALL.draw(
            &mut fb,
            &format!("{icon:?}"),
            x,
            y + 20,
            Align::Left,
            BinaryColor::On,
        );
        y += row_height;
        if y + row_height > 480 {
            y = 8;
            x += 400;
        }
    }
    fb
}

/// All sizes an icon is available in.
pub fn all_sizes() -> Vec<u32> {
    SIZES.to_vec()
}
