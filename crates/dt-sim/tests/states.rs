//! Every page with no data yet, with every source failing, and with stale
//! data while offline.

use dt_core::store::{Page, Store};
use dt_sim::config::CONFIG;
use dt_sim::fixtures::{context, empty_store, failed_store, stale_store, NOW, OFFSET};
use dt_sim::image::assert_golden;
use dt_ui::chrome::{draw_chrome, Chrome};
use dt_ui::{format, pages, view, FrameBuffer};

fn render(page: Page, s: &Store, wifi: Option<i8>) -> FrameBuffer {
    let mut ctx = context();
    ctx.wifi_rssi = wifi;
    let mut fb = FrameBuffer::new();
    let body = draw_chrome(
        &mut fb,
        &Chrome {
            page,
            date: Some(format::date(NOW, OFFSET)),
            wifi_rssi: wifi,
            warning: view::has_problem(s, page),
            updated: view::updated(s, page, &ctx),
        },
    );
    match page {
        Page::Home => pages::home::draw(&mut fb, body, &view::home(s, &CONFIG, &ctx)),
        Page::Weather => pages::weather::draw(&mut fb, body, &view::weather(s, &ctx)),
        Page::News => pages::news::draw(&mut fb, body, &view::news(s, &CONFIG, &ctx)),
        Page::Finance => pages::finance::draw(&mut fb, body, &view::finance(s, &CONFIG, &ctx)),
    }
    fb
}

fn name(page: Page) -> &'static str {
    match page {
        Page::Home => "home",
        Page::Weather => "weather",
        Page::News => "news",
        Page::Finance => "finance",
    }
}

#[test]
fn loading_states() {
    for page in Page::ALL {
        assert_golden(
            &format!("state_loading_{}", name(page)),
            &render(page, &empty_store(), Some(-60)),
        );
    }
}

#[test]
fn failed_states() {
    for page in Page::ALL {
        assert_golden(
            &format!("state_failed_{}", name(page)),
            &render(page, &failed_store(), Some(-60)),
        );
    }
}

#[test]
fn stale_states() {
    for page in Page::ALL {
        assert_golden(
            &format!("state_stale_{}", name(page)),
            &render(page, &stale_store(), None),
        );
    }
}
