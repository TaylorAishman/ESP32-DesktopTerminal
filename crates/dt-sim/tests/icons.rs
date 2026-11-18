//! Contact sheets of the icons.

use dt_sim::image::assert_golden;
use dt_sim::sheet::{all_sizes, contact_sheet};
use dt_ui::Icon;

#[test]
fn icon_pipeline_sample() {
    assert_golden("icons_sample", &contact_sheet(&[Icon::Sun], &all_sizes()));
}
