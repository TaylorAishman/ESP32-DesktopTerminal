//! Render any screen to a PNG on the PC, using the same code as the device.
//!
//!     cargo run -p dt-sim -- <screen> [state] [output.png]
//!
//! screen: home | weather | news | finance | splash
//! state:  full (default) | loading | failed | stale

use std::path::PathBuf;
use std::process::ExitCode;

use dt_core::store::Page;
use dt_sim::config::CONFIG;
use dt_sim::fixtures::{context, empty_store, failed_store, full_store, stale_store};
use dt_sim::image::write_png;
use dt_ui::{render, FrameBuffer, Screen};

const USAGE: &str =
    "usage: dt-sim <home|weather|news|finance|splash> [full|loading|failed|stale] [output.png]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(which) = args.first() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let screen = match which.as_str() {
        "home" => Screen::Page(Page::Home),
        "weather" => Screen::Page(Page::Weather),
        "news" => Screen::Page(Page::News),
        "finance" => Screen::Page(Page::Finance),
        "splash" => Screen::Splash {
            version: env!("CARGO_PKG_VERSION").into(),
            status: "Starting...".into(),
        },
        other => {
            eprintln!("unknown screen '{other}'\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let state = args.get(1).map_or("full", String::as_str);
    let mut ctx = context();
    let store = match state {
        "full" => full_store(),
        "loading" => empty_store(),
        "failed" => failed_store(),
        "stale" => {
            ctx.wifi_rssi = None;
            stale_store()
        }
        other => {
            eprintln!("unknown state '{other}'\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let out = args.get(2).map_or_else(
        || PathBuf::from(format!("{which}-{state}.png")),
        PathBuf::from,
    );

    let mut fb = FrameBuffer::new();
    let started = std::time::Instant::now();
    render(&mut fb, &screen, &store, &CONFIG, &ctx);
    let took = started.elapsed();
    if let Err(e) = write_png(&fb, &out) {
        eprintln!("cannot write {}: {e}", out.display());
        return ExitCode::FAILURE;
    }
    println!("{} ({took:?} to render)", out.display());
    ExitCode::SUCCESS
}
