//! Screens outside the four pages: start-up splash, safe mode and
//! diagnostics (REQ-UX-07, REQ-ERR-06, REQ-ERR-07).

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, primitives::Rectangle};

use crate::chrome::screen;
use crate::icon::Icon;
use crate::layout::{hline, titled_panel, RectExt, GAP};
use crate::text::{Align, BODY, BODY_BOLD, DISPLAY, HERO, SMALL, SMALL_BOLD, TITLE};
use crate::view::ErrorView;

const ON: BinaryColor = BinaryColor::On;

/// Start-up screen: name, version and what is happening.
pub fn splash<D: DrawTarget<Color = BinaryColor>>(target: &mut D, version: &str, status: &str) {
    let c = screen().center();
    Icon::PartlyCloudyDay.draw_centered(target, Point::new(c.x, c.y - 110), 96, ON);
    HERO.draw(target, "Desktop Terminal", c.x, c.y + 10, Align::Center, ON);
    BODY.draw(
        target,
        &format!("Version {version}"),
        c.x,
        c.y + 44,
        Align::Center,
        ON,
    );
    hline(target, c.x - 160, c.x + 160, c.y + 70, 1);
    BODY_BOLD.draw(
        target,
        &BODY_BOLD.truncate(status, 700),
        c.x,
        c.y + 104,
        Align::Center,
        ON,
    );
    SMALL.draw(
        target,
        "1 Home    2 Weather    3 News    4 Finance",
        c.x,
        456,
        Align::Center,
        ON,
    );
}

/// What the safe-mode screen shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafeModeView {
    pub restarts: u32,
    /// Most recent first.
    pub errors: Vec<ErrorView>,
    /// e.g. "10:35 AM"; when a normal start will be tried automatically.
    pub retry_at: Option<String>,
}

/// Shown after repeated crashes: the network is off and the reason is shown.
pub fn safe_mode<D: DrawTarget<Color = BinaryColor>>(target: &mut D, v: &SafeModeView) {
    let area = screen().inset(32, 24);
    Icon::Warning.draw(target, area.top_left, 48, ON);
    DISPLAY.draw(
        target,
        "Safe mode",
        area.top_left.x + 64,
        area.top_left.y + 36,
        Align::Left,
        ON,
    );
    let mut y = area.top_left.y + 84;
    let w = area.size.width as i32;
    let why = format!(
        "The terminal restarted {} times in a row because of errors, so it has started without Wi-Fi or data to stay usable.",
        v.restarts
    );
    y = BODY.draw_wrapped(target, &why, Point::new(area.top_left.x, y), w, 2, ON) + 8;

    let errors = Rectangle::new(
        Point::new(area.top_left.x, y),
        Size::new(area.size.width, 190),
    );
    let body = titled_panel(target, errors, "Recent errors");
    let mut ey = body.top_left.y + 14;
    if v.errors.is_empty() {
        SMALL.draw(
            target,
            "None recorded.",
            body.top_left.x,
            ey,
            Align::Left,
            ON,
        );
    }
    for e in v.errors.iter().take(3) {
        SMALL_BOLD.draw(target, &e.code, body.top_left.x, ey, Align::Left, ON);
        let tx = body.top_left.x + 90;
        SMALL.draw(
            target,
            &SMALL.truncate(&e.message, body.right() - tx),
            tx,
            ey,
            Align::Left,
            ON,
        );
        SMALL.draw(
            target,
            &SMALL.truncate(&e.hint, body.right() - tx),
            tx,
            ey + 17,
            Align::Left,
            ON,
        );
        ey += 46;
    }

    let mut y = area.bottom() - 42;
    TITLE.draw(
        target,
        "Press any button to try a normal start.",
        area.top_left.x,
        y,
        Align::Left,
        ON,
    );
    y += 26;
    let auto = match &v.retry_at {
        Some(t) => {
            format!("A normal start is tried automatically at {t}. Details are in the serial log.")
        }
        None => "Details are in the serial log.".into(),
    };
    SMALL.draw(target, &auto, area.top_left.x, y, Align::Left, ON);
}

/// One labelled value.
pub type Field = (String, String);

/// One job's status row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobRow {
    pub name: String,
    /// e.g. "4 min ago" or "never".
    pub last_success: String,
    /// Error code of the last failure, or empty.
    pub last_error: String,
    /// e.g. "in 12 min".
    pub next_run: String,
}

/// Everything on the diagnostics screen, formatted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticsView {
    /// Firmware version, uptime, reset reason, time synced...
    pub system: Vec<Field>,
    /// SSID, IP, signal...
    pub network: Vec<Field>,
    /// Free heap, minimum free heap...
    pub memory: Vec<Field>,
    pub jobs: Vec<JobRow>,
    /// Per provider: "NewsAPI" -> "37 / 100 today".
    pub budgets: Vec<Field>,
}

/// Diagnostics: system, network and memory facts, job status and API budget.
pub fn diagnostics<D: DrawTarget<Color = BinaryColor>>(target: &mut D, v: &DiagnosticsView) {
    let area = screen().inset(16, 12);
    TITLE.draw(
        target,
        "DIAGNOSTICS",
        area.top_left.x,
        area.top_left.y + 18,
        Align::Left,
        ON,
    );
    SMALL.draw(
        target,
        "Press any button to return",
        area.right(),
        area.top_left.y + 18,
        Align::Right,
        ON,
    );
    hline(
        target,
        area.top_left.x,
        area.right(),
        area.top_left.y + 28,
        2,
    );
    let rest = area.split_top(36).1;
    let cols = rest.split_columns_weighted(&[2, 3], 3 * GAP);

    let left = cols[0].split_rows(3, GAP);
    fields(target, left[0], "System", &v.system);
    fields(target, left[1], "Network", &v.network);
    fields(target, left[2], "Memory", &v.memory);

    let (budget_area, jobs_area) = cols[1].split_bottom(96);
    let body = titled_panel(target, jobs_area, "Jobs");
    let c = body.split_columns_weighted(&[5, 4, 3, 4], GAP);
    let mut y = body.top_left.y + 12;
    for (i, h) in ["job", "last success", "error", "next run"]
        .iter()
        .enumerate()
    {
        SMALL_BOLD.draw(target, h, c[i].top_left.x, y, Align::Left, ON);
    }
    for row in &v.jobs {
        y += 17;
        if y > body.bottom() {
            break;
        }
        for (i, s) in [&row.name, &row.last_success, &row.last_error, &row.next_run]
            .iter()
            .enumerate()
        {
            SMALL.draw(
                target,
                &SMALL.truncate(s, c[i].size.width as i32),
                c[i].top_left.x,
                y,
                Align::Left,
                ON,
            );
        }
    }
    fields(
        target,
        budget_area.split_top(GAP).1,
        "API budget",
        &v.budgets,
    );
}

fn fields<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    area: Rectangle,
    title: &str,
    items: &[Field],
) {
    let body = titled_panel(target, area, title);
    let label_w = items.iter().map(|(k, _)| SMALL.width(k)).max().unwrap_or(0) + 12;
    let mut y = body.top_left.y + 12;
    for (k, v) in items {
        if y > body.bottom() {
            break;
        }
        SMALL.draw(target, k, body.top_left.x, y, Align::Left, ON);
        let x = body.top_left.x + label_w;
        SMALL_BOLD.draw(
            target,
            &SMALL_BOLD.truncate(v, body.right() - x),
            x,
            y,
            Align::Left,
            ON,
        );
        y += 17;
    }
}
