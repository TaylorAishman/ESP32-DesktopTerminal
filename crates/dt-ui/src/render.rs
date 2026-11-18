//! The one entry point the firmware calls to draw a frame.

use dt_core::config::Config;
use dt_core::platform::Button;
use dt_core::store::{Page, Snapshot};
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

use crate::chrome::{draw_chrome, Chrome};
use crate::framebuffer::FrameBuffer;
use crate::system::{DiagnosticsView, SafeModeView};
use crate::view::{self, Context};
use crate::{format, pages, system};

/// The page each front-panel button opens (REQ-UX-02).
pub fn page_for_button(button: Button) -> Page {
    match button {
        Button::One => Page::Home,
        Button::Two => Page::Weather,
        Button::Three => Page::News,
        Button::Four => Page::Finance,
    }
}

/// Anything that can fill the screen.
#[derive(Clone, Debug, PartialEq)]
pub enum Screen {
    Page(Page),
    Splash { version: String, status: String },
    SafeMode(SafeModeView),
    Diagnostics(DiagnosticsView),
}

/// Draw `screen` into `fb`, replacing its contents.
pub fn render(
    fb: &mut FrameBuffer,
    screen: &Screen,
    snapshot: &Snapshot,
    config: &Config,
    ctx: &Context,
) {
    fb.fill(BinaryColor::Off);
    match screen {
        Screen::Page(page) => render_page(fb, *page, snapshot, config, ctx),
        Screen::Splash { version, status } => system::splash(fb, version, status),
        Screen::SafeMode(v) => system::safe_mode(fb, v),
        Screen::Diagnostics(v) => system::diagnostics(fb, v),
    }
}

fn render_page<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    page: Page,
    s: &Snapshot,
    config: &Config,
    ctx: &Context,
) {
    let chrome = Chrome {
        page,
        date: ctx.now.map(|n| format::date(n, ctx.utc_offset)),
        wifi_rssi: ctx.wifi_rssi,
        warning: view::has_problem(s, page),
        updated: view::updated(s, page, ctx),
    };
    let body = draw_chrome(target, &chrome);
    match page {
        Page::Home => pages::home::draw(target, body, &view::home(s, config, ctx)),
        Page::Weather => pages::weather::draw(target, body, &view::weather(s, ctx)),
        Page::News => pages::news::draw(target, body, &view::news(s, config, ctx)),
        Page::Finance => pages::finance::draw(target, body, &view::finance(s, config, ctx)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_map_to_pages_in_order() {
        let pages: Vec<Page> = Button::ALL.iter().map(|&b| page_for_button(b)).collect();
        assert_eq!(pages, Page::ALL);
    }
}
