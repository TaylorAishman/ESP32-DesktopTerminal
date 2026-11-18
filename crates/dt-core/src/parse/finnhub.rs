//! Finnhub `/quote`. https://finnhub.io/docs/api/quote

use serde::Deserialize;

use crate::model::Quote;
use crate::platform::UnixTime;

#[derive(Deserialize)]
struct Body {
    /// Current price.
    c: f32,
    /// Change.
    d: Option<f32>,
    /// Percent change.
    dp: Option<f32>,
    h: f32,
    l: f32,
    o: f32,
    /// Previous close.
    pc: f32,
    /// Time of the last trade.
    t: i64,
}

/// Parse a quote for the instrument shown as `label` with ticker `symbol`.
///
/// `Ok(None)` means Finnhub does not know the symbol (it answers with zeros
/// rather than an error).
pub fn parse_quote(body: &[u8], label: &str, symbol: &str) -> Result<Option<Quote>, String> {
    let b: Body = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    if b.t == 0 && b.c == 0.0 && b.pc == 0.0 {
        return Ok(None);
    }
    let change = b.d.unwrap_or(b.c - b.pc);
    let change_pct = b.dp.unwrap_or(if b.pc != 0.0 {
        change / b.pc * 100.0
    } else {
        0.0
    });
    Ok(Some(Quote {
        label: label.to_owned(),
        symbol: symbol.to_owned(),
        price: b.c,
        change,
        change_pct,
        day_low: b.l,
        day_high: b.h,
        open: b.o,
        previous_close: b.pc,
        at: UnixTime(b.t),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quote() {
        let q = parse_quote(
            include_bytes!("../../tests/fixtures/finnhub_quote.json"),
            "VOO",
            "VOO",
        )
        .unwrap()
        .unwrap();
        assert_eq!(q.price, 501.37);
        assert_eq!(q.change, -2.18);
        assert_eq!(q.change_pct, -0.4329);
        assert_eq!((q.day_low, q.day_high), (499.85, 504.12));
        assert_eq!(q.previous_close, 503.55);
        assert_eq!(q.at, UnixTime(1_794_944_400));
        assert!(q.is_down());
    }

    #[test]
    fn unknown_symbol_is_none() {
        let body = include_bytes!("../../tests/fixtures/finnhub_quote_unknown.json");
        assert_eq!(parse_quote(body, "X", "NOPE").unwrap(), None);
    }

    #[test]
    fn missing_change_is_computed() {
        let q = parse_quote(
            br#"{"c":110,"h":111,"l":99,"o":100,"pc":100,"t":5}"#,
            "A",
            "A",
        )
        .unwrap()
        .unwrap();
        assert_eq!(q.change, 10.0);
        assert_eq!(q.change_pct, 10.0);
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(parse_quote(b"<html>", "A", "A").is_err());
    }
}
