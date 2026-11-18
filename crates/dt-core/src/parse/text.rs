//! Making provider text drawable: the display fonts cover Latin-1, so other
//! characters are transliterated or dropped, never drawn as boxes
//! (REQ-NEWS-04). The ellipsis "…" is kept; the UI draws it specially.

/// Replace characters outside Latin-1 with close equivalents, drop the ones
/// that have none, and collapse runs of whitespace.
pub fn transliterate(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' | '\u{2032}' => out.push('\''),
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' | '\u{2033}' => out.push('"'),
            '\u{2010}'..='\u{2015}' | '\u{2212}' => out.push('-'),
            '\u{2026}' => out.push('…'),
            '\u{00A0}' | '\u{2000}'..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}' => {
                out.push(' ')
            }
            '\u{2022}' | '\u{00B7}' => out.push('·'),
            '\u{20AC}' => out.push_str("EUR"),
            '\u{2122}' => out.push_str("(TM)"),
            c if (c as u32) < 0x20 => out.push(' '),
            c if (c as u32) <= 0xFF => out.push(c),
            c => {
                if let Some(base) = latin_extended_base(c) {
                    out.push_str(base);
                }
                // Anything else (emoji, CJK, symbols, zero-width marks) is dropped.
            }
        }
    }
    collapse_spaces(&out)
}

fn collapse_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = true;
    for c in s.chars() {
        if c == ' ' {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out.trim_end().to_owned()
}

/// Base letters for Latin Extended-A (U+0100 to U+017F) and a few others
/// common in names.
fn latin_extended_base(c: char) -> Option<&'static str> {
    const TABLE: &[(char, &str)] = &[
        ('Ā', "A"),
        ('ā', "a"),
        ('Ă', "A"),
        ('ă', "a"),
        ('Ą', "A"),
        ('ą', "a"),
        ('Ć', "C"),
        ('ć', "c"),
        ('Ĉ', "C"),
        ('ĉ', "c"),
        ('Ċ', "C"),
        ('ċ', "c"),
        ('Č', "C"),
        ('č', "c"),
        ('Ď', "D"),
        ('ď', "d"),
        ('Đ', "D"),
        ('đ', "d"),
        ('Ē', "E"),
        ('ē', "e"),
        ('Ĕ', "E"),
        ('ĕ', "e"),
        ('Ė', "E"),
        ('ė', "e"),
        ('Ę', "E"),
        ('ę', "e"),
        ('Ě', "E"),
        ('ě', "e"),
        ('Ĝ', "G"),
        ('ĝ', "g"),
        ('Ğ', "G"),
        ('ğ', "g"),
        ('Ġ', "G"),
        ('ġ', "g"),
        ('Ģ', "G"),
        ('ģ', "g"),
        ('Ĥ', "H"),
        ('ĥ', "h"),
        ('Ħ', "H"),
        ('ħ', "h"),
        ('Ĩ', "I"),
        ('ĩ', "i"),
        ('Ī', "I"),
        ('ī', "i"),
        ('Ĭ', "I"),
        ('ĭ', "i"),
        ('Į', "I"),
        ('į', "i"),
        ('İ', "I"),
        ('ı', "i"),
        ('Ĳ', "IJ"),
        ('ĳ', "ij"),
        ('Ĵ', "J"),
        ('ĵ', "j"),
        ('Ķ', "K"),
        ('ķ', "k"),
        ('ĸ', "k"),
        ('Ĺ', "L"),
        ('ĺ', "l"),
        ('Ļ', "L"),
        ('ļ', "l"),
        ('Ľ', "L"),
        ('ľ', "l"),
        ('Ŀ', "L"),
        ('ŀ', "l"),
        ('Ł', "L"),
        ('ł', "l"),
        ('Ń', "N"),
        ('ń', "n"),
        ('Ņ', "N"),
        ('ņ', "n"),
        ('Ň', "N"),
        ('ň', "n"),
        ('ŉ', "n"),
        ('Ŋ', "N"),
        ('ŋ', "n"),
        ('Ō', "O"),
        ('ō', "o"),
        ('Ŏ', "O"),
        ('ŏ', "o"),
        ('Ő', "O"),
        ('ő', "o"),
        ('Œ', "OE"),
        ('œ', "oe"),
        ('Ŕ', "R"),
        ('ŕ', "r"),
        ('Ŗ', "R"),
        ('ŗ', "r"),
        ('Ř', "R"),
        ('ř', "r"),
        ('Ś', "S"),
        ('ś', "s"),
        ('Ŝ', "S"),
        ('ŝ', "s"),
        ('Ş', "S"),
        ('ş', "s"),
        ('Š', "S"),
        ('š', "s"),
        ('Ș', "S"),
        ('ș', "s"),
        ('Ţ', "T"),
        ('ţ', "t"),
        ('Ť', "T"),
        ('ť', "t"),
        ('Ŧ', "T"),
        ('ŧ', "t"),
        ('Ț', "T"),
        ('ț', "t"),
        ('Ũ', "U"),
        ('ũ', "u"),
        ('Ū', "U"),
        ('ū', "u"),
        ('Ŭ', "U"),
        ('ŭ', "u"),
        ('Ů', "U"),
        ('ů', "u"),
        ('Ű', "U"),
        ('ű', "u"),
        ('Ų', "U"),
        ('ų', "u"),
        ('Ŵ', "W"),
        ('ŵ', "w"),
        ('Ŷ', "Y"),
        ('ŷ', "y"),
        ('Ÿ', "Y"),
        ('Ź', "Z"),
        ('ź', "z"),
        ('Ż', "Z"),
        ('ż', "z"),
        ('Ž', "Z"),
        ('ž', "z"),
        ('ſ', "s"),
        ('ƒ', "f"),
    ];
    TABLE.iter().find(|(k, _)| *k == c).map(|(_, v)| *v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_headline_cases() {
        let cases = [
            (
                "Apple’s new chip is “twice as fast” — and ships next week",
                "Apple's new chip is \"twice as fast\" - and ships next week",
            ),
            (
                "Łódź and Kraków brace for early snow 🌨️ as temperatures plunge",
                "Lódz and Kraków brace for early snow as temperatures plunge",
            ),
            (
                "Erdoğan meets Macron in Paris",
                "Erdogan meets Macron in Paris",
            ),
            (
                "Prices rise 3% in the euro zone (€2.1bn)",
                "Prices rise 3% in the euro zone (EUR2.1bn)",
            ),
            ("Café owners say “non!”", "Café owners say \"non!\""),
            ("Wait for it…", "Wait for it…"),
            ("北京 Beijing summit", "Beijing summit"),
            ("Tabs\tand\nnewlines\u{00A0}too", "Tabs and newlines too"),
            ("Zero\u{200B}width", "Zerowidth"),
        ];
        for (input, expected) in cases {
            assert_eq!(transliterate(input), expected, "input: {input}");
        }
    }

    #[test]
    fn output_is_latin1_plus_ellipsis() {
        let s = transliterate("Ωmega ☃ snow 🚀 rocket – ok … “x”");
        assert!(s.chars().all(|c| (c as u32) <= 0xFF || c == '…'), "{s}");
    }
}
