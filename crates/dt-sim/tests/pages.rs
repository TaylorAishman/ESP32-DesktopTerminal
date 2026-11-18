//! Golden images of the pages, rendered from the fixture store.

use dt_core::store::Page;
use dt_sim::config::CONFIG;
use dt_sim::fixtures::{context, full_store, NOW, OFFSET};
use dt_sim::image::assert_golden;
use dt_ui::chrome::{draw_chrome, Chrome};
use dt_ui::{format, view, FrameBuffer};

fn framed(
    page: Page,
    f: impl FnOnce(&mut FrameBuffer, embedded_graphics::primitives::Rectangle),
) -> FrameBuffer {
    let s = full_store();
    let ctx = context();
    let mut fb = FrameBuffer::new();
    let body = draw_chrome(
        &mut fb,
        &Chrome {
            page,
            date: Some(format::date(NOW, OFFSET)),
            wifi_rssi: ctx.wifi_rssi,
            warning: view::has_problem(&s, page),
            updated: view::updated(&s, page, &ctx),
        },
    );
    f(&mut fb, body);
    fb
}

#[test]
fn weather_page() {
    let fb = framed(Page::Weather, |fb, body| {
        dt_ui::pages::weather::draw(fb, body, &view::weather(&full_store(), &context()));
    });
    assert_golden("page_weather", &fb);
}

#[test]
fn news_page() {
    let fb = framed(Page::News, |fb, body| {
        dt_ui::pages::news::draw(fb, body, &view::news(&full_store(), &CONFIG, &context()));
    });
    assert_golden("page_news", &fb);
}

#[test]
fn finance_page() {
    let fb = framed(Page::Finance, |fb, body| {
        dt_ui::pages::finance::draw(fb, body, &view::finance(&full_store(), &CONFIG, &context()));
    });
    assert_golden("page_finance", &fb);
}

#[test]
fn home_page() {
    let fb = framed(Page::Home, |fb, body| {
        dt_ui::pages::home::draw(fb, body, &view::home(&full_store(), &CONFIG, &context()));
    });
    assert_golden("page_home", &fb);
}
