//! A `log` backend that writes ESP-IDF-style lines to the serial console:
//! `I (12345) dt_core::api::manager: message`.

use std::io::Write;
use std::time::Instant;

use log::{Level, LevelFilter, Log, Metadata, Record};

struct SerialLogger {
    start: Instant,
}

impl Log for SerialLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let letter = match record.level() {
            Level::Error => 'E',
            Level::Warn => 'W',
            Level::Info => 'I',
            Level::Debug => 'D',
            Level::Trace => 'V',
        };
        let ms = self.start.elapsed().as_millis();
        // One write per line so lines from different threads do not interleave.
        let line = format!("{letter} ({ms}) {}: {}\n", record.target(), record.args());
        let _ = std::io::stdout().lock().write_all(line.as_bytes());
    }

    fn flush(&self) {
        let _ = std::io::stdout().flush();
    }
}

/// Install the logger. Call once, first thing in `main`.
pub fn init() {
    let logger = Box::leak(Box::new(SerialLogger { start: Instant::now() }));
    if log::set_logger(logger).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}
