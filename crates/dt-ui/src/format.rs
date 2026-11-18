//! Formatting values for display, in the user's units.

use dt_core::config::Units;
use dt_core::platform::UnixTime;
use dt_core::time::LocalDateTime;

/// "72°" (Fahrenheit for imperial, Celsius for metric).
pub fn temperature(celsius: f32, units: Units) -> String {
    format!("{}°", temperature_value(celsius, units).round() as i32)
}

pub fn temperature_value(celsius: f32, units: Units) -> f32 {
    match units {
        Units::Metric => celsius,
        Units::Imperial => celsius * 9.0 / 5.0 + 32.0,
    }
}

/// "12 mph SW" or "19 km/h SW".
pub fn wind(speed_ms: f32, deg: Option<u16>, units: Units) -> String {
    let (v, unit) = match units {
        Units::Metric => (speed_ms * 3.6, "km/h"),
        Units::Imperial => (speed_ms * 2.236_94, "mph"),
    };
    match deg {
        Some(d) => format!("{} {unit} {}", v.round() as i32, compass(d)),
        None => format!("{} {unit}", v.round() as i32),
    }
}

/// Eight-point compass direction the wind comes from.
pub fn compass(deg: u16) -> &'static str {
    const P: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    P[((u32::from(deg % 360) * 2 + 45) / 90 % 8) as usize]
}

/// Clock time: "6:42 AM" for imperial (US convention), "06:42" for metric.
pub fn clock(at: UnixTime, offset: i32, units: Units) -> String {
    let t = LocalDateTime::from_unix(at, offset);
    match units {
        Units::Metric => format!("{:02}:{:02}", t.hour, t.minute),
        Units::Imperial => {
            let (h, ampm) = twelve_hour(t.hour);
            format!("{h}:{:02} {ampm}", t.minute)
        }
    }
}

/// Hour label for charts: "3 PM" or "15:00".
pub fn hour(at: UnixTime, offset: i32, units: Units) -> String {
    let t = LocalDateTime::from_unix(at, offset);
    match units {
        Units::Metric => format!("{:02}:00", t.hour),
        Units::Imperial => {
            let (h, ampm) = twelve_hour(t.hour);
            format!("{h} {ampm}")
        }
    }
}

fn twelve_hour(h: u8) -> (u8, &'static str) {
    let ampm = if h < 12 { "AM" } else { "PM" };
    let h12 = match h % 12 {
        0 => 12,
        x => x,
    };
    (h12, ampm)
}

/// "Tue 17 Nov".
pub fn date(at: UnixTime, offset: i32) -> String {
    let d = LocalDateTime::from_unix(at, offset).date;
    format!(
        "{} {} {}",
        d.weekday().short_name(),
        d.day,
        d.month_short_name()
    )
}

/// How long ago: "just now", "5 min ago", "3 h ago", "2 d ago".
pub fn age(seconds: i64) -> String {
    match seconds.max(0) {
        0..=59 => "just now".into(),
        s @ 60..=3599 => format!("{} min ago", s / 60),
        s @ 3600..=86_399 => format!("{} h ago", s / 3600),
        s => format!("{} d ago", s / 86_400),
    }
}

/// A price with two decimals and thousands separators: "1,234.56".
pub fn price(v: f32) -> String {
    let s = format!("{:.2}", v.abs());
    let (int, frac) = s.split_once('.').unwrap_or((&s, "00"));
    let mut grouped = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    format!("{}{grouped}.{frac}", if v < 0.0 { "-" } else { "" })
}

/// A signed change: "+1.23" / "-1.23".
pub fn change(v: f32) -> String {
    let p = price(v.abs());
    if v < 0.0 {
        format!("-{p}")
    } else {
        format!("+{p}")
    }
}

/// A signed percentage: "+0.43%".
pub fn percent_change(v: f32) -> String {
    format!("{}{:.2}%", if v < 0.0 { "-" } else { "+" }, v.abs())
}

/// A percentage of a 0-1 fraction, rounded to tens: "40%".
pub fn chance(f: f32) -> String {
    format!("{}%", ((f.clamp(0.0, 1.0) * 10.0).round() * 10.0) as i32)
}

/// First letter upper-case: "light rain" -> "Light rain".
pub fn sentence(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-11-17 14:05:09 UTC
    const T: UnixTime = UnixTime(1_794_924_309);
    const NY: i32 = -5 * 3600;

    #[test]
    fn temperatures() {
        assert_eq!(temperature(8.42, Units::Metric), "8°");
        assert_eq!(temperature(8.42, Units::Imperial), "47°");
        assert_eq!(temperature(-12.6, Units::Metric), "-13°");
        assert_eq!(temperature(-40.0, Units::Imperial), "-40°");
    }

    #[test]
    fn winds() {
        assert_eq!(wind(4.63, Some(250), Units::Imperial), "10 mph W");
        assert_eq!(wind(4.63, Some(225), Units::Metric), "17 km/h SW");
        assert_eq!(wind(0.0, None, Units::Metric), "0 km/h");
        assert_eq!(compass(0), "N");
        assert_eq!(compass(359), "N");
        assert_eq!(compass(22), "N");
        assert_eq!(compass(23), "NE");
        assert_eq!(compass(180), "S");
    }

    #[test]
    fn times() {
        assert_eq!(clock(T, NY, Units::Imperial), "9:05 AM");
        assert_eq!(clock(T, NY, Units::Metric), "09:05");
        assert_eq!(clock(T, 0, Units::Imperial), "2:05 PM");
        assert_eq!(hour(T, NY, Units::Imperial), "9 AM");
        assert_eq!(hour(UnixTime(T.0 - 14 * 3600), 0, Units::Imperial), "12 AM");
        assert_eq!(hour(T, 0, Units::Metric), "14:00");
        assert_eq!(date(T, NY), "Tue 17 Nov");
    }

    #[test]
    fn ages() {
        assert_eq!(age(5), "just now");
        assert_eq!(age(300), "5 min ago");
        assert_eq!(age(3 * 3600 + 5), "3 h ago");
        assert_eq!(age(2 * 86_400), "2 d ago");
        assert_eq!(age(-10), "just now");
    }

    #[test]
    fn money() {
        assert_eq!(price(501.367), "501.37");
        assert_eq!(price(44_123.4), "44,123.40");
        assert_eq!(price(1_000_000.0), "1,000,000.00");
        assert_eq!(price(-2.5), "-2.50");
        assert_eq!(change(-2.18), "-2.18");
        assert_eq!(change(0.5), "+0.50");
        assert_eq!(percent_change(-0.4329), "-0.43%");
        assert_eq!(percent_change(1.0), "+1.00%");
        assert_eq!(chance(0.37), "40%");
        assert_eq!(sentence("light rain"), "Light rain");
    }
}
