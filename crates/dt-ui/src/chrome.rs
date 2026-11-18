//! The frame shared by every page (REQ-UX-06): a header with the page title,
//! date and status icons, and a footer with the navigation strip, laid out
//! in the same order as the physical buttons, and the last-update time.

use dt_core::store::Page;
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::framebuffer::{HEIGHT, WIDTH};
use crate::icon::Icon;
use crate::layout::{hline, vline, RectExt, GAP};
use crate::text::{Align, SMALL, SMALL_BOLD, TITLE};
use crate::ui_icon::wifi_icon;

pub const HEADER_HEIGHT: u32 = 48;
pub const FOOTER_HEIGHT: u32 = 40;
/// Margin around the page content.
pub const MARGIN: u32 = 16;

const ON: BinaryColor = BinaryColor::On;
const OFF: BinaryColor = BinaryColor::Off;

/// What the frame shows besides the page itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chrome {
    pub page: Page,
    /// e.g. "Tue 17 Nov"; `None` before the clock is set.
    pub date: Option<String>,
    /// Wi-Fi signal strength in dBm; `None` when not connected.
    pub wifi_rssi: Option<i8>,
    /// Some data on this page failed to refresh.
    pub warning: bool,
    /// e.g. "14:05"; time of the newest data on this page.
    pub updated: Option<String>,
}

pub fn page_title(page: Page) -> &'static str {
    match page {
        Page::Home => "Home",
        Page::Weather => "Weather",
        Page::News => "News",
        Page::Finance => "Finance",
    }
}

pub fn page_icon(page: Page) -> Icon {
    match page {
        Page::Home => Icon::PageHome,
        Page::Weather => Icon::PageWeather,
        Page::News => Icon::PageNews,
        Page::Finance => Icon::PageFinance,
    }
}

/// The whole screen.
pub fn screen() -> Rectangle {
    Rectangle::new(Point::zero(), Size::new(WIDTH, HEIGHT))
}

/// Draw header and footer; return the content area between them.
pub fn draw_chrome<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    chrome: &Chrome,
) -> Rectangle {
    let (header, rest) = screen().split_top(HEADER_HEIGHT);
    let (footer, body) = rest.split_bottom(FOOTER_HEIGHT);
    draw_header(target, header, chrome);
    draw_footer(target, footer, chrome);
    body.inset(MARGIN, GAP + 4)
}

fn draw_header<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, c: &Chrome) {
    let inner = area.inset(MARGIN, 0);
    let mid = inner.top_left.y + 32;
    page_icon(c.page).draw(
        target,
        Point::new(inner.top_left.x, inner.top_left.y + 10),
        24,
        ON,
    );
    TITLE.draw(
        target,
        &page_title(c.page).to_uppercase(),
        inner.top_left.x + 34,
        mid,
        Align::Left,
        ON,
    );

    if let Some(date) = &c.date {
        TITLE.draw(target, date, WIDTH as i32 / 2, mid, Align::Center, ON);
    }

    let mut x = inner.right() - 24;
    wifi_icon(c.wifi_rssi).draw(target, Point::new(x, inner.top_left.y + 10), 24, ON);
    if c.warning {
        x -= 32;
        Icon::Warning.draw(target, Point::new(x, inner.top_left.y + 10), 24, ON);
    }
    hline(target, 0, WIDTH as i32 - 1, area.bottom() - 2, 3);
}

fn draw_footer<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, c: &Chrome) {
    hline(target, 0, WIDTH as i32 - 1, area.top_left.y, 1);
    let (tabs_area, info) = area.split_left(4 * 140);
    let tabs = tabs_area.inset(GAP, 5).split_columns(4, 4);
    for (i, (page, tab)) in Page::ALL.iter().zip(tabs).enumerate() {
        let active = *page == c.page;
        let ink = if active { OFF } else { ON };
        if active {
            let _ = tab.into_styled(PrimitiveStyle::with_fill(ON)).draw(target);
        } else {
            let _ = tab
                .into_styled(PrimitiveStyle::with_stroke(ON, 1))
                .draw(target);
        }
        let cy = tab.top_left.y + tab.size.height as i32 / 2;
        let label = format!("{}  {}", i + 1, page_title(*page));
        SMALL_BOLD.draw(
            target,
            &label,
            tab.top_left.x + tab.size.width as i32 / 2,
            cy + 5,
            Align::Center,
            ink,
        );
    }
    vline(
        target,
        info.top_left.x,
        info.top_left.y + 6,
        info.bottom() - 6,
        1,
    );
    if let Some(t) = &c.updated {
        SMALL.draw(
            target,
            &format!("Updated {t}"),
            info.right() - MARGIN as i32,
            info.top_left.y + 25,
            Align::Right,
            ON,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FrameBuffer;

    #[test]
    fn content_area_sits_between_header_and_footer() {
        let mut fb = FrameBuffer::new();
        let body = draw_chrome(
            &mut fb,
            &Chrome {
                page: Page::News,
                date: None,
                wifi_rssi: None,
                warning: false,
                updated: None,
            },
        );
        assert_eq!(body.top_left, Point::new(16, 48 + 12));
        assert_eq!(body.bottom(), 480 - 40 - 12);
        assert_eq!(body.right(), 800 - 16);
    }
}
