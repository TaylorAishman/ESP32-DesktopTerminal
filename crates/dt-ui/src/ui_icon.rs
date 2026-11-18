//! Interface icons: connectivity, status, trends and page glyphs.

use crate::icon::Icon;

/// Signal strength icon for an RSSI in dBm, or Wi-Fi off.
pub fn wifi_icon(rssi: Option<i8>) -> Icon {
    match rssi {
        None => Icon::WifiOff,
        Some(r) if r >= -55 => Icon::Wifi4,
        Some(r) if r >= -67 => Icon::Wifi3,
        Some(r) if r >= -78 => Icon::Wifi2,
        Some(_) => Icon::Wifi1,
    }
}

/// Up, down or flat marker for a change.
pub fn trend_icon(change: f32) -> Icon {
    if change > 0.0 {
        Icon::TrendUp
    } else if change < 0.0 {
        Icon::TrendDown
    } else {
        Icon::TrendFlat
    }
}

/// Interface icons in review order.
pub const UI_ICONS: [Icon; 15] = [
    Icon::Wifi4,
    Icon::Wifi3,
    Icon::Wifi2,
    Icon::Wifi1,
    Icon::WifiOff,
    Icon::Warning,
    Icon::Stale,
    Icon::TrendUp,
    Icon::TrendDown,
    Icon::TrendFlat,
    Icon::PageHome,
    Icon::PageWeather,
    Icon::PageNews,
    Icon::PageFinance,
    Icon::Unknown,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wifi_levels() {
        assert_eq!(wifi_icon(None), Icon::WifiOff);
        assert_eq!(wifi_icon(Some(-40)), Icon::Wifi4);
        assert_eq!(wifi_icon(Some(-60)), Icon::Wifi3);
        assert_eq!(wifi_icon(Some(-75)), Icon::Wifi2);
        assert_eq!(wifi_icon(Some(-90)), Icon::Wifi1);
    }

    #[test]
    fn trends() {
        assert_eq!(trend_icon(0.5), Icon::TrendUp);
        assert_eq!(trend_icon(-0.01), Icon::TrendDown);
        assert_eq!(trend_icon(0.0), Icon::TrendFlat);
    }
}
