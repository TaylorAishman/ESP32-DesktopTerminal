//! The page frame for each page.

use dt_core::store::Page;
use dt_sim::image::assert_golden;
use dt_ui::chrome::{draw_chrome, Chrome};
use dt_ui::FrameBuffer;

#[test]
fn chrome_for_each_page() {
    for (page, name, rssi, warning) in [
        (Page::Home, "chrome_home", Some(-50), false),
        (Page::Weather, "chrome_weather", Some(-70), false),
        (Page::News, "chrome_news", Some(-80), true),
        (Page::Finance, "chrome_finance", None, true),
    ] {
        let mut fb = FrameBuffer::new();
        draw_chrome(
            &mut fb,
            &Chrome {
                page,
                date: Some("Tue 17 Nov".into()),
                wifi_rssi: rssi,
                warning,
                updated: Some("14:05".into()),
            },
        );
        assert_golden(name, &fb);
    }
}
