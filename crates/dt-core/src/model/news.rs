//! News models.

use crate::platform::UnixTime;

/// Most headlines kept per topic. More than a page can show; the rest are
/// spares for when titles are long.
pub const MAX_HEADLINES: usize = 8;
/// Longest title kept, in characters. Longer titles are cut at a word
/// boundary and end with an ellipsis.
pub const MAX_TITLE_CHARS: usize = 160;
/// Longest source name kept, in characters.
pub const MAX_SOURCE_CHARS: usize = 40;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Headline {
    title: String,
    source: String,
    published_at: Option<UnixTime>,
}

impl Headline {
    /// A headline with title and source cut to their maximum lengths.
    pub fn new(title: &str, source: &str, published_at: Option<UnixTime>) -> Self {
        Headline {
            title: truncate_words(title.trim(), MAX_TITLE_CHARS),
            source: truncate_words(source.trim(), MAX_SOURCE_CHARS),
            published_at,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn published_at(&self) -> Option<UnixTime> {
        self.published_at
    }
}

/// The headlines for one configured topic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewsTopic {
    title: String,
    headlines: Vec<Headline>,
}

impl NewsTopic {
    /// A topic keeping at most [`MAX_HEADLINES`] headlines, in the given order.
    pub fn new(title: impl Into<String>, headlines: impl IntoIterator<Item = Headline>) -> Self {
        NewsTopic {
            title: title.into(),
            headlines: headlines.into_iter().take(MAX_HEADLINES).collect(),
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn headlines(&self) -> &[Headline] {
        &self.headlines
    }
}

/// Cut `s` to at most `max` characters, at a word boundary where possible,
/// ending with "…" when anything was removed.
fn truncate_words(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_owned();
    }
    let keep: String = s.chars().take(max.saturating_sub(1)).collect();
    let cut = match keep.rfind(' ') {
        // Only back up to a space if that keeps most of the text.
        Some(i) if i >= keep.len() / 2 => &keep[..i],
        _ => &keep[..],
    };
    format!("{}…", cut.trim_end_matches([' ', ',', ';', ':', '-']))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_unchanged() {
        let h = Headline::new("  Markets rally  ", "Reuters", None);
        assert_eq!(h.title(), "Markets rally");
    }

    #[test]
    fn long_title_is_cut_at_a_word() {
        let long = "word ".repeat(60);
        let h = Headline::new(&long, "x", None);
        assert!(h.title().chars().count() <= MAX_TITLE_CHARS);
        assert!(h.title().ends_with("word…"));
    }

    #[test]
    fn long_unbroken_text_is_cut_hard() {
        let h = Headline::new(&"a".repeat(300), &"b".repeat(100), None);
        assert_eq!(h.title().chars().count(), MAX_TITLE_CHARS);
        assert_eq!(h.source().chars().count(), MAX_SOURCE_CHARS);
    }

    #[test]
    fn topic_caps_headline_count() {
        let hs = (0..20).map(|i| Headline::new(&format!("h{i}"), "s", None));
        let t = NewsTopic::new("Tech", hs);
        assert_eq!(t.headlines().len(), MAX_HEADLINES);
        assert_eq!(t.headlines()[0].title(), "h0");
    }
}
