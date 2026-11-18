//! Contact sheets of the icons.

use dt_sim::image::assert_golden;
use dt_sim::sheet::{all_sizes, contact_sheet};
use dt_ui::weather_icon::WEATHER_ICONS;
use dt_ui::Icon;

#[test]
fn icon_pipeline_sample() {
    assert_golden(
        "icons_sample",
        &contact_sheet(&[Icon::ClearDay], &all_sizes()),
    );
}

#[test]
fn weather_icons() {
    let sizes = [16, 24, 32, 48];
    assert_golden(
        "icons_weather_a",
        &contact_sheet(&WEATHER_ICONS[..9], &sizes),
    );
    assert_golden(
        "icons_weather_b",
        &contact_sheet(&WEATHER_ICONS[9..], &sizes),
    );
    let large = [64, 96];
    assert_golden(
        "icons_weather_large_a",
        &contact_sheet(&WEATHER_ICONS[..6], &large),
    );
    assert_golden(
        "icons_weather_large_b",
        &contact_sheet(&WEATHER_ICONS[6..12], &large),
    );
}
