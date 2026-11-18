//! NewsAPI `v2/top-headlines`. https://newsapi.org/docs/endpoints/top-headlines

use serde::Deserialize;

use super::text::transliterate;
use crate::model::{Headline, NewsTopic};
use crate::platform::UnixTime;
use crate::time::Date;

#[derive(Deserialize)]
struct Body {
    status: String,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    articles: Vec<Article>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Article {
    #[serde(default)]
    source: Option<Source>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
}

#[derive(Deserialize)]
struct Source {
    #[serde(default)]
    name: Option<String>,
}

/// Parse a top-headlines response into the topic titled `topic_title`.
pub fn parse_top_headlines(body: &[u8], topic_title: &str) -> Result<NewsTopic, String> {
    let b: Body = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    if b.status != "ok" {
        return Err(b
            .message
            .unwrap_or_else(|| format!("status '{}'", b.status)));
    }
    let headlines = b.articles.into_iter().filter_map(|a| {
        let title = a.title?;
        if title.trim().is_empty() || title.trim() == "[Removed]" {
            return None;
        }
        let source = a.source.and_then(|s| s.name).unwrap_or_default();
        let title = strip_source_suffix(&title, &source);
        Some(Headline::new(
            &transliterate(title),
            &transliterate(&source),
            a.published_at.as_deref().and_then(parse_rfc3339),
        ))
    });
    Ok(NewsTopic::new(topic_title, headlines))
}

/// NewsAPI titles usually end with " - <source>"; that is shown separately.
fn strip_source_suffix<'a>(title: &'a str, source: &str) -> &'a str {
    let t = title.trim();
    if !source.is_empty() {
        for sep in [" - ", " | ", " – ", " — "] {
            if let Some(stripped) = t.strip_suffix(source).and_then(|r| r.strip_suffix(sep)) {
                if !stripped.trim().is_empty() {
                    return stripped.trim_end();
                }
            }
        }
    }
    t
}

/// Parse "2026-11-17T13:45:00Z", with optional fractional seconds and a "Z"
/// or "±hh:mm" offset.
pub fn parse_rfc3339(s: &str) -> Option<UnixTime> {
    let b = s.as_bytes();
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || !matches!(b[10], b'T' | b't' | b' ')
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let num = |r: core::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let (year, month, day) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hour, minute, second) = (num(11..13)?, num(14..16)?, num(17..19)?);
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    let mut rest = &s[19..];
    if let Some(r) = rest.strip_prefix('.') {
        rest = r.trim_start_matches(|c: char| c.is_ascii_digit());
    }
    let offset = match rest {
        "Z" | "z" => 0,
        r if r.len() == 6 && (r.starts_with('+') || r.starts_with('-')) && &r[3..4] == ":" => {
            let sign = if r.starts_with('-') { -1 } else { 1 };
            sign * (r[1..3].parse::<i64>().ok()? * 3600 + r[4..6].parse::<i64>().ok()? * 60)
        }
        _ => return None,
    };
    let days = Date {
        year: year as i32,
        month: month as u8,
        day: day as u8,
    }
    .days_since_epoch();
    Some(UnixTime(
        days * 86_400 + hour * 3600 + minute * 60 + second - offset,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = include_bytes!("../../tests/fixtures/newsapi_top_headlines.json");

    #[test]
    fn parses_fixture() {
        let t = parse_top_headlines(FIXTURE, "U.S.").unwrap();
        assert_eq!(t.title(), "U.S.");
        let h = t.headlines();
        assert_eq!(h.len(), 4, "removed article is dropped");
        assert_eq!(
            h[0].title(),
            "Senate passes stopgap funding bill hours before shutdown deadline"
        );
        assert_eq!(h[0].source(), "Associated Press");
        assert_eq!(h[0].published_at(), Some(UnixTime(1_794_923_100)));
        assert_eq!(
            h[1].title(),
            "Apple's new chip is \"twice as fast\" - and ships next week"
        );
        assert_eq!(h[1].published_at(), Some(UnixTime(1_794_917_131)));
        assert_eq!(
            h[2].title(),
            "Lódz and Kraków brace for early snow as temperatures plunge"
        );
        assert_eq!(h[2].published_at(), Some(UnixTime(1_794_906_000)));
        assert_eq!(h[3].published_at(), None, "bad date is tolerated");
    }

    #[test]
    fn error_status_is_reported() {
        let body = br#"{"status":"error","code":"parameterInvalid","message":"You must include at least one q, source, or domain."}"#;
        assert_eq!(
            parse_top_headlines(body, "x").unwrap_err(),
            "You must include at least one q, source, or domain."
        );
    }

    #[test]
    fn source_suffix() {
        assert_eq!(strip_source_suffix("Big news - CNN", "CNN"), "Big news");
        assert_eq!(
            strip_source_suffix("Big news | Reuters", "Reuters"),
            "Big news"
        );
        assert_eq!(
            strip_source_suffix("Big news - CNN", "BBC"),
            "Big news - CNN"
        );
        assert_eq!(strip_source_suffix(" - CNN", "CNN"), "- CNN");
    }

    #[test]
    fn timestamps() {
        assert_eq!(parse_rfc3339("1970-01-01T00:00:00Z"), Some(UnixTime(0)));
        assert_eq!(
            parse_rfc3339("2026-11-17T08:45:00-05:00"),
            Some(UnixTime(1_794_923_100))
        );
        assert_eq!(parse_rfc3339("2026-11-17T13:45:00"), None);
        assert_eq!(parse_rfc3339("2026-13-17T13:45:00Z"), None);
    }
}
