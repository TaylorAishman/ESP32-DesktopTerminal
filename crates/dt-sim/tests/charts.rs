//! Chart widgets with normal, flat and too-short data.

use dt_sim::image::assert_golden;
use dt_ui::chart::{bars, line_chart, sparkline};
use dt_ui::layout::RectExt;
use dt_ui::text::{Align, SMALL};
use dt_ui::FrameBuffer;
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};

fn boxed(fb: &mut FrameBuffer, x: i32, y: i32, w: u32, h: u32, label: &str) -> Rectangle {
    let r = Rectangle::new(Point::new(x, y), Size::new(w, h));
    r.into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(fb)
        .unwrap();
    SMALL.draw(fb, label, x, y - 4, Align::Left, BinaryColor::On);
    r.inset(6, 6)
}

#[test]
fn charts() {
    let mut fb = FrameBuffer::new();
    let day: Vec<f32> = (0..120)
        .map(|i| 500.0 + (i as f32 / 9.0).sin() * 2.0 + i as f32 * 0.02)
        .collect();

    let a = boxed(&mut fb, 20, 30, 220, 70, "sparkline, previous close 499");
    sparkline(&mut fb, a, &day, Some(499.0));
    let a = boxed(&mut fb, 280, 30, 220, 70, "sparkline, flat");
    sparkline(&mut fb, a, &[3.0; 20], None);
    let a = boxed(&mut fb, 540, 30, 220, 70, "sparkline, one point");
    sparkline(&mut fb, a, &[3.0], None);

    let temps = [8.4, 9.1, 7.0, 5.2, 4.8, 6.6, 10.3, 11.0];
    let a = boxed(
        &mut fb,
        20,
        150,
        740,
        140,
        "hourly temperatures with labels",
    );
    let pts = line_chart(&mut fb, a, &temps, 24);
    for (p, t) in pts.iter().zip(temps) {
        SMALL.draw(
            &mut fb,
            &format!("{t:.0}°"),
            p.x,
            p.y - 8,
            Align::Center,
            BinaryColor::On,
        );
    }
    let a = boxed(&mut fb, 20, 340, 740, 60, "precipitation probability");
    bars(&mut fb, a, &[0.0, 0.1, 0.4, 0.8, 1.0, 0.6, 0.2, 0.0], 20);
    let a = boxed(&mut fb, 20, 430, 220, 40, "line chart, one point");
    line_chart(&mut fb, a, &[1.0], 4);

    assert_golden("charts", &fb);
}
