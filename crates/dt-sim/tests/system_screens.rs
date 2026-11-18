//! Splash, safe mode and diagnostics screens.

use dt_sim::image::assert_golden;
use dt_ui::system::{diagnostics, safe_mode, splash, DiagnosticsView, JobRow, SafeModeView};
use dt_ui::view::ErrorView;
use dt_ui::FrameBuffer;

fn f(k: &str, v: &str) -> (String, String) {
    (k.into(), v.into())
}

#[test]
fn splash_screen() {
    let mut fb = FrameBuffer::new();
    splash(&mut fb, "0.1.0", "Connecting to Wi-Fi 'HomeNet'...");
    assert_golden("system_splash", &fb);
}

#[test]
fn safe_mode_screen() {
    let mut fb = FrameBuffer::new();
    safe_mode(
        &mut fb,
        &SafeModeView {
            restarts: 3,
            errors: vec![
                ErrorView {
                    code: "E-SYS-01".into(),
                    message: "Restarted after an internal error: panicked at dt-core/src/store.rs:88".into(),
                    hint: "Details are on the diagnostics screen (hold button 1). Please report repeated crashes.".into(),
                },
                ErrorView {
                    code: "E-SYS-02".into(),
                    message: "Restarted after the system stopped responding (network thread).".into(),
                    hint: "Please report repeated restarts.".into(),
                },
            ],
            retry_at: Some("10:35 AM".into()),
        },
    );
    assert_golden("system_safe_mode", &fb);
}

#[test]
fn diagnostics_screen() {
    let job = |n: &str, s: &str, e: &str, x: &str| JobRow {
        name: n.into(),
        last_success: s.into(),
        last_error: e.into(),
        next_run: x.into(),
    };
    let mut fb = FrameBuffer::new();
    diagnostics(
        &mut fb,
        &DiagnosticsView {
            system: vec![
                f("Firmware", "0.1.0"),
                f("Uptime", "3 d 4 h"),
                f("Last reset", "Power on"),
                f("Clock", "synced"),
            ],
            network: vec![
                f("Wi-Fi", "HomeNet"),
                f("Signal", "-58 dBm"),
                f("IP", "192.168.1.50"),
            ],
            memory: vec![f("Free heap", "96 KB"), f("Lowest", "52 KB")],
            jobs: vec![
                job("weather.current", "4 min ago", "", "in 6 min"),
                job("weather.forecast", "12 min ago", "", "in 18 min"),
                job("news[0]", "20 min ago", "", "in 44 min"),
                job("news[1]", "4 min ago", "", "in 60 min"),
                job("news[2]", "52 min ago", "E-NET-04", "in 1 min"),
                job("news[3]", "36 min ago", "", "in 28 min"),
                job("quote[0]", "never", "E-API-10", "in 60 min"),
                job("quote[1]", "1 min ago", "", "in 1 min"),
                job("quote[2]", "1 min ago", "", "in 1 min"),
                job("quote[3]", "1 min ago", "", "in 1 min"),
            ],
            budgets: vec![
                f("NewsAPI", "37 / 100 today"),
                f("OpenWeatherMap", "8,412 / 1,000,000 this month"),
                f("Finnhub", "4 / 60 this minute"),
            ],
        },
    );
    assert_golden("system_diagnostics", &fb);
}
