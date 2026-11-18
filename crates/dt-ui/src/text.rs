//! Fonts and text layout: measure, align, truncate with an ellipsis, wrap.
//!
//! All fonts are FreeUniversal (`fur` regular, `fub` bold) from u8g2 in their
//! `_tf` variants, which cover Latin-1. Provider text is transliterated to
//! Latin-1 before it gets here (`dt_core::parse::text`); the one exception,
//! "…", is drawn as three dots.

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};
use u8g2_fonts::{
    fonts,
    types::{FontColor, HorizontalAlignment, VerticalPosition},
    FontRenderer,
};

/// A font in the type scale.
pub struct Font {
    renderer: FontRenderer,
    /// Distance between baselines of wrapped lines, in pixels.
    pub line_height: i32,
    /// Height of capitals above the baseline, for vertical centring.
    pub cap_height: i32,
}

macro_rules! font {
    ($f:ty, $line:expr, $cap:expr) => {
        Font {
            renderer: FontRenderer::new::<$f>().with_ignore_unknown_chars(true),
            line_height: $line,
            cap_height: $cap,
        }
    };
}

/// Current temperature and other headline numbers.
pub static HERO: Font = font!(fonts::u8g2_font_fub42_tf, 60, 42);
/// Prices and large values.
pub static DISPLAY: Font = font!(fonts::u8g2_font_fub25_tf, 36, 25);
/// Page and section titles.
pub static TITLE: Font = font!(fonts::u8g2_font_fub17_tf, 25, 17);
/// Emphasised body text.
pub static BODY_BOLD: Font = font!(fonts::u8g2_font_fub14_tf, 21, 14);
/// Body text: headlines, values.
pub static BODY: Font = font!(fonts::u8g2_font_fur14_tf, 21, 14);
/// Captions: sources, times, units.
pub static SMALL: Font = font!(fonts::u8g2_font_fur11_tf, 16, 11);
/// Emphasised captions.
pub static SMALL_BOLD: Font = font!(fonts::u8g2_font_fub11_tf, 16, 11);

/// Horizontal alignment relative to the anchor point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

const ELLIPSIS: &str = "...";

/// Replace characters the fonts cannot draw but we still want to show.
fn drawable(text: &str) -> std::borrow::Cow<'_, str> {
    if text.contains('…') {
        text.replace('…', ELLIPSIS).into()
    } else {
        text.into()
    }
}

impl Font {
    /// Width of `text` in pixels (its advance).
    pub fn width(&self, text: &str) -> i32 {
        self.renderer
            .get_rendered_dimensions(
                drawable(text).as_ref(),
                Point::zero(),
                VerticalPosition::Baseline,
            )
            .map(|d| d.advance.x)
            .unwrap_or(0)
    }

    /// Draw `text` with its baseline at `y`, aligned at `x`.
    pub fn draw<D: DrawTarget<Color = BinaryColor>>(
        &self,
        target: &mut D,
        text: &str,
        x: i32,
        y: i32,
        align: Align,
        ink: BinaryColor,
    ) {
        let h = match align {
            Align::Left => HorizontalAlignment::Left,
            Align::Center => HorizontalAlignment::Center,
            Align::Right => HorizontalAlignment::Right,
        };
        // Drawing into a frame buffer cannot fail; a missing glyph is skipped.
        let _ = self.renderer.render_aligned(
            drawable(text).as_ref(),
            Point::new(x, y),
            VerticalPosition::Baseline,
            h,
            FontColor::Transparent(ink),
            target,
        );
    }

    /// `text` cut to fit `max_width`, ending in "..." if anything was cut.
    pub fn truncate(&self, text: &str, max_width: i32) -> String {
        let text = drawable(text);
        if self.width(&text) <= max_width {
            return text.into_owned();
        }
        let room = max_width - self.width(ELLIPSIS);
        let mut out = String::new();
        for c in text.chars() {
            out.push(c);
            if self.width(&out) > room {
                out.pop();
                break;
            }
        }
        format!("{}{ELLIPSIS}", out.trim_end())
    }

    /// Break `text` into lines no wider than `max_width`, at spaces where
    /// possible (a word wider than a line is split). At most `max_lines`
    /// lines; if the text does not fit, the last line ends in "...".
    pub fn wrap(&self, text: &str, max_width: i32, max_lines: usize) -> Vec<String> {
        let text = drawable(text);
        let mut words: std::collections::VecDeque<String> =
            text.split_whitespace().map(str::to_owned).collect();
        let mut lines: Vec<String> = Vec::new();
        let mut current = String::new();
        while lines.len() < max_lines {
            let Some(word) = words.pop_front() else { break };
            let candidate = if current.is_empty() {
                word.clone()
            } else {
                format!("{current} {word}")
            };
            if self.width(&candidate) <= max_width {
                current = candidate;
            } else if !current.is_empty() {
                // Start a new line and try the word again there.
                lines.push(std::mem::take(&mut current));
                words.push_front(word);
            } else {
                // The word alone is too wide: split it.
                let cut = self.fit_prefix(&word, max_width);
                lines.push(word[..cut].to_owned());
                words.push_front(word[cut..].to_owned());
            }
        }
        if !current.is_empty() {
            if lines.len() < max_lines {
                lines.push(current);
            } else {
                words.push_front(current);
            }
        }
        if !words.is_empty() {
            if let Some(last) = lines.last_mut() {
                *last = self.truncate(&format!("{last}{ELLIPSIS}{ELLIPSIS}"), max_width);
            }
        }
        lines
    }

    /// Byte length of the longest prefix of `word` that fits (at least one
    /// character, so progress is always made).
    fn fit_prefix(&self, word: &str, max_width: i32) -> usize {
        let mut cut = 0;
        for (i, c) in word.char_indices() {
            if self.width(&word[..i + c.len_utf8()]) > max_width {
                break;
            }
            cut = i + c.len_utf8();
        }
        cut.max(word.chars().next().map_or(0, char::len_utf8))
    }

    /// Draw wrapped text with the first baseline at `origin`. Returns the
    /// baseline after the last line drawn.
    pub fn draw_wrapped<D: DrawTarget<Color = BinaryColor>>(
        &self,
        target: &mut D,
        text: &str,
        origin: Point,
        max_width: i32,
        max_lines: usize,
        ink: BinaryColor,
    ) -> i32 {
        let mut baseline = origin.y;
        for line in self.wrap(text, max_width, max_lines) {
            self.draw(target, &line, origin.x, baseline, Align::Left, ink);
            baseline += self.line_height;
        }
        baseline
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_are_positive_and_additive() {
        let a = BODY.width("Hello");
        let b = BODY.width(" world");
        assert!(a > 0);
        assert!((BODY.width("Hello world") - (a + b)).abs() <= 1);
        assert!(HERO.width("8") > BODY.width("8"));
        assert_eq!(BODY.width(""), 0);
    }

    #[test]
    fn truncate_fits_and_marks() {
        let s = BODY.truncate("A rather long headline that will not fit", 150);
        assert!(BODY.width(&s) <= 150, "{s}");
        assert!(s.ends_with("..."));
        assert_eq!(BODY.truncate("Short", 150), "Short");
    }

    #[test]
    fn wrap_respects_width_and_lines() {
        let text =
            "Senate passes stopgap funding bill hours before shutdown deadline as talks continue";
        let lines = BODY.wrap(text, 300, 2);
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|l| BODY.width(l) <= 300), "{lines:?}");
        assert!(lines[1].ends_with("..."), "{lines:?}");

        let all = BODY.wrap(text, 300, 10);
        assert!(all.len() >= 3);
        assert!(!all.last().unwrap().ends_with("..."));
        assert_eq!(all.join(" "), text);
    }

    #[test]
    fn wrap_breaks_long_words() {
        let lines = BODY.wrap(&"x".repeat(80), 200, 5);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|l| BODY.width(l) <= 200), "{lines:?}");
    }

    #[test]
    fn ellipsis_character_is_drawable() {
        assert_eq!(BODY.width("a…"), BODY.width("a..."));
    }

    #[test]
    fn latin1_is_covered() {
        // A glyph present in the font has a positive advance.
        for c in "éüñçÀÖß°±£¥".chars() {
            assert!(BODY.width(&c.to_string()) > 0, "{c}");
        }
    }
}
