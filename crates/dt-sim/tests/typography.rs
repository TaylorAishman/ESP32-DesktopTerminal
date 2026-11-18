//! Type specimen: every style, alignment, truncation and wrapping.

use dt_sim::image::assert_golden;
use dt_ui::text::{Align, Font, BODY, BODY_BOLD, DISPLAY, HERO, SMALL, SMALL_BOLD, TITLE};
use dt_ui::FrameBuffer;
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle},
};

const INK: BinaryColor = BinaryColor::On;

#[test]
fn typography_specimen() {
    let mut fb = FrameBuffer::new();
    let styles: [(&str, &Font); 7] = [
        ("HERO 42", &HERO),
        ("DISPLAY 25", &DISPLAY),
        ("TITLE 17", &TITLE),
        ("BODY BOLD 14", &BODY_BOLD),
        ("BODY 14", &BODY),
        ("SMALL BOLD 11", &SMALL_BOLD),
        ("SMALL 11", &SMALL),
    ];
    let mut y = 50;
    for (name, font) in styles {
        SMALL.draw(&mut fb, name, 16, y, Align::Left, INK);
        font.draw(
            &mut fb,
            "Weather 72°F Ünïcödé £€ ...",
            140,
            y,
            Align::Left,
            INK,
        );
        y += font.line_height + 4;
    }

    // Alignment against a guide line.
    let x = 600;
    Line::new(Point::new(x, 300), Point::new(x, 370))
        .into_styled(PrimitiveStyle::with_stroke(INK, 1))
        .draw(&mut fb)
        .unwrap();
    BODY.draw(&mut fb, "Left", x, 320, Align::Left, INK);
    BODY.draw(&mut fb, "Center", x, 343, Align::Center, INK);
    BODY.draw(&mut fb, "Right", x, 366, Align::Right, INK);

    // Truncation and wrapping inside a 300 px column.
    let headline = "Senate passes stopgap funding bill hours before shutdown deadline as talks continue into the night";
    Line::new(Point::new(316, 300), Point::new(316, 470))
        .into_styled(PrimitiveStyle::with_stroke(INK, 1))
        .draw(&mut fb)
        .unwrap();
    BODY_BOLD.draw(
        &mut fb,
        &BODY_BOLD.truncate(headline, 300),
        16,
        320,
        Align::Left,
        INK,
    );
    BODY.draw_wrapped(&mut fb, headline, Point::new(16, 350), 300, 3, INK);
    SMALL.draw_wrapped(&mut fb, headline, Point::new(16, 425), 300, 3, INK);

    assert_golden("typography_specimen", &fb);
}
