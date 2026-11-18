//! Checks across the three configuration files. Every problem found is
//! reported, each naming the file, the key and the fix (REQ-CFG-06).

use crate::api::ApiConfig;
use crate::error::{ConfigError, ConfigErrors};
use crate::secrets::SecretsConfig;
use crate::user::UserConfig;

/// Endpoints the firmware relies on, as (provider, endpoint).
pub const WEATHER_CURRENT: (&str, &str) = ("openweathermap", "current");
pub const WEATHER_FORECAST: (&str, &str) = ("openweathermap", "forecast");
pub const NEWS_HEADLINES: (&str, &str) = ("newsapi", "top_headlines");
pub const FINANCE_QUOTE: (&str, &str) = ("finnhub", "quote");
pub const REQUIRED_ENDPOINTS: [(&str, &str); 4] = [
    WEATHER_CURRENT,
    WEATHER_FORECAST,
    NEWS_HEADLINES,
    FINANCE_QUOTE,
];

/// Most topics and instruments the pages are laid out for.
pub const MAX_TOPICS: usize = 6;
pub const MAX_INSTRUMENTS: usize = 6;
pub const MAX_TOPIC_TITLE_CHARS: usize = 20;
pub const MAX_LABEL_CHARS: usize = 14;

/// Smallest and largest receive buffers the firmware will allocate.
pub const MIN_RESPONSE_BYTES: u32 = 256;
pub const MAX_RESPONSE_BYTES: u32 = 32 * 1024;

/// Which files the configs came from, for messages.
#[derive(Clone, Copy, Debug)]
pub struct Files<'a> {
    pub api: &'a str,
    pub user: &'a str,
    pub secrets: &'a str,
}

/// Whether template placeholders are acceptable (CI builds without secrets).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placeholders {
    Reject,
    Allow,
}

pub fn validate(
    api: &ApiConfig,
    user: &UserConfig,
    secrets: &SecretsConfig,
    files: Files<'_>,
    placeholders: Placeholders,
) -> Result<(), ConfigErrors> {
    let mut errors = Vec::new();
    check_api(api, files.api, &mut errors);
    check_user(user, files.user, &mut errors);
    check_secrets(api, secrets, files, placeholders, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ConfigErrors(errors))
    }
}

/// A value copied unchanged from the template.
pub fn is_placeholder(value: &str) -> bool {
    let v = value.trim();
    v.is_empty() || v.starts_with("YOUR_") || v.starts_with("PASTE_")
}

fn check_api(api: &ApiConfig, file: &str, errors: &mut Vec<ConfigError>) {
    let err = |key: String, msg: String, fix: &str| ConfigError::new(file, msg).at(key).fix(fix);

    if api.user_agent.trim().is_empty() {
        errors.push(err(
            "user_agent".into(),
            "is empty".into(),
            "Set a User-Agent, e.g. \"ESP32-DesktopTerminal/0.1\"; NewsAPI rejects requests without one.",
        ));
    }

    for (pid, p) in &api.providers {
        let key = |k: &str| format!("providers.{pid}.{k}");
        if !p.base_url.starts_with("https://") {
            errors.push(err(
                key("base_url"),
                format!("'{}' is not an https:// URL", p.base_url),
                "All API traffic must use HTTPS (REQ-API-08).",
            ));
        }
        if p.base_url.ends_with('/') {
            errors.push(err(
                key("base_url"),
                "ends with '/'".into(),
                "Remove the trailing slash; endpoint paths start with '/'.",
            ));
        }
        if p.limits.is_empty() {
            errors.push(err(
                key("limits"),
                "no limits given".into(),
                "Add at least one limit, e.g. limits = [{ calls = 60, per = \"minute\" }].",
            ));
        }
        for (i, l) in p.limits.iter().enumerate() {
            if l.calls == 0 {
                errors.push(err(
                    format!("providers.{pid}.limits[{i}].calls"),
                    "is 0".into(),
                    "A limit must allow at least one call.",
                ));
            }
        }
        if !(p.utilisation > 0.0 && p.utilisation <= 1.0) {
            errors.push(err(
                key("utilisation"),
                format!("{} is outside (0, 1]", p.utilisation),
                "Use a fraction such as 0.9.",
            ));
        }
        if p.timeout_ms == 0 || p.timeout_ms > 60_000 {
            errors.push(err(
                key("timeout_ms"),
                format!("{} is outside 1..=60000", p.timeout_ms),
                "Use a timeout such as 10000.",
            ));
        }
        for (eid, e) in &p.endpoints {
            let key = |k: &str| format!("providers.{pid}.endpoints.{eid}.{k}");
            if !e.path.starts_with('/') {
                errors.push(err(
                    key("path"),
                    format!("'{}' does not start with '/'", e.path),
                    "Write the path as \"/name\".",
                ));
            }
            if e.weight == 0 {
                errors.push(err(
                    key("weight"),
                    "is 0".into(),
                    "Use a weight of 1 or more.",
                ));
            }
            if e.min_interval_s == 0 {
                errors.push(err(
                    key("min_interval_s"),
                    "is 0".into(),
                    "Use the shortest useful polling interval in seconds.",
                ));
            }
            if e.max_staleness_s < e.min_interval_s {
                errors.push(err(
                    key("max_staleness_s"),
                    format!(
                        "{} is less than min_interval_s ({})",
                        e.max_staleness_s, e.min_interval_s
                    ),
                    "max_staleness_s must be at least min_interval_s.",
                ));
            }
            if !(MIN_RESPONSE_BYTES..=MAX_RESPONSE_BYTES).contains(&e.max_response_bytes) {
                errors.push(err(
                    key("max_response_bytes"),
                    format!(
                        "{} is outside {MIN_RESPONSE_BYTES}..={MAX_RESPONSE_BYTES}",
                        e.max_response_bytes
                    ),
                    "Size the buffer to the largest expected response.",
                ));
            }
            if let Some(off) = e.off_hours_interval_s {
                if off < e.min_interval_s {
                    errors.push(err(
                        key("off_hours_interval_s"),
                        format!("{off} is less than min_interval_s ({})", e.min_interval_s),
                        "Off-hours polling should be slower than normal polling.",
                    ));
                }
            }
        }
    }

    for (pid, eid) in REQUIRED_ENDPOINTS {
        let present = api
            .providers
            .get(pid)
            .is_some_and(|p| p.endpoints.contains_key(eid));
        if !present {
            errors.push(err(
                format!("providers.{pid}.endpoints.{eid}"),
                "is missing".into(),
                "The firmware needs this endpoint; restore it from git history.",
            ));
        }
    }
}

fn check_user(user: &UserConfig, file: &str, errors: &mut Vec<ConfigError>) {
    let err = |key: String, msg: String, fix: &str| ConfigError::new(file, msg).at(key).fix(fix);
    let loc = &user.location;

    if loc.name.trim().is_empty() {
        errors.push(err(
            "location.name".into(),
            "is empty".into(),
            "Give the place name to show on screen.",
        ));
    }
    if !(-90.0..=90.0).contains(&loc.latitude) {
        errors.push(err(
            "location.latitude".into(),
            format!("{} is outside -90..=90", loc.latitude),
            "Use decimal degrees, north positive.",
        ));
    }
    if !(-180.0..=180.0).contains(&loc.longitude) {
        errors.push(err(
            "location.longitude".into(),
            format!("{} is outside -180..=180", loc.longitude),
            "Use decimal degrees, east positive.",
        ));
    }
    if !is_plausible_posix_tz(&loc.timezone) {
        errors.push(err(
            "location.timezone".into(),
            format!("'{}' is not a POSIX time-zone string", loc.timezone),
            "Use a string such as \"EST5EDT,M3.2.0,M11.1.0\"; see the examples in config/user.example.toml.",
        ));
    }

    let topics = &user.news.topics;
    if topics.is_empty() || topics.len() > MAX_TOPICS {
        errors.push(err(
            "news.topics".into(),
            format!(
                "{} topics; between 1 and {MAX_TOPICS} are supported",
                topics.len()
            ),
            "Add or remove [[news.topics]] entries.",
        ));
    }
    for (i, t) in topics.iter().enumerate() {
        let key = |k: &str| format!("news.topics[{i}].{k}");
        check_short_text(&t.title, MAX_TOPIC_TITLE_CHARS, key("title"), file, errors);
        let has = |k: &str| t.params.contains_key(k);
        if !(has("country") || has("category") || has("sources") || has("q")) {
            errors.push(err(
                key("params"),
                format!(
                    "topic '{}' has none of country, category, sources or q",
                    t.title
                ),
                "NewsAPI top-headlines needs at least one of them.",
            ));
        }
        if has("sources") && (has("country") || has("category")) {
            errors.push(err(
                key("params"),
                format!("topic '{}' combines sources with country or category", t.title),
                "NewsAPI does not allow this; use sources alone, or country/category without sources.",
            ));
        }
    }

    let instruments = &user.finance.instruments;
    if instruments.is_empty() || instruments.len() > MAX_INSTRUMENTS {
        errors.push(err(
            "finance.instruments".into(),
            format!(
                "{} instruments; between 1 and {MAX_INSTRUMENTS} are supported",
                instruments.len()
            ),
            "Add or remove [[finance.instruments]] entries.",
        ));
    }
    for (i, inst) in instruments.iter().enumerate() {
        check_short_text(
            &inst.label,
            MAX_LABEL_CHARS,
            format!("finance.instruments[{i}].label"),
            file,
            errors,
        );
        let ok = !inst.symbol.is_empty()
            && inst.symbol.len() <= 12
            && inst.symbol.chars().all(|c| {
                c.is_ascii_uppercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | ':')
            });
        if !ok {
            errors.push(err(
                format!("finance.instruments[{i}].symbol"),
                format!("'{}' is not a ticker symbol", inst.symbol),
                "Use the exchange ticker in capitals, e.g. \"VOO\".",
            ));
        }
    }
}

fn check_short_text(
    value: &str,
    max: usize,
    key: String,
    file: &str,
    errors: &mut Vec<ConfigError>,
) {
    let n = value.trim().chars().count();
    if n == 0 || n > max {
        errors.push(
            ConfigError::new(file, format!("'{value}' must be 1 to {max} characters"))
                .at(key)
                .fix("Shorten it so it fits on screen."),
        );
    }
}

fn check_secrets(
    api: &ApiConfig,
    secrets: &SecretsConfig,
    files: Files<'_>,
    placeholders: Placeholders,
    errors: &mut Vec<ConfigError>,
) {
    let file = files.secrets;
    let reject = placeholders == Placeholders::Reject;

    let ssid = &secrets.wifi.ssid;
    if reject && is_placeholder(ssid) {
        errors.push(
            ConfigError::new(file, "is still the template placeholder")
                .at("wifi.ssid")
                .fix("Enter your Wi-Fi network name."),
        );
    }
    if ssid.len() > 32 {
        errors.push(
            ConfigError::new(file, "is longer than 32 bytes")
                .at("wifi.ssid")
                .fix("Wi-Fi network names are at most 32 bytes."),
        );
    }
    let pass = secrets.wifi.password.expose();
    if reject && is_placeholder(pass) && !pass.is_empty() {
        errors.push(
            ConfigError::new(file, "is still the template placeholder")
                .at("wifi.password")
                .fix("Enter your Wi-Fi password, or \"\" for an open network."),
        );
    }
    if !pass.is_empty() && !(8..=63).contains(&pass.len()) {
        errors.push(
            ConfigError::new(
                file,
                format!("is {} characters; WPA passwords are 8 to 63", pass.len()),
            )
            .at("wifi.password")
            .fix("Check the Wi-Fi password."),
        );
    }

    for (pid, p) in &api.providers {
        let name = &p.auth.secret;
        match secrets.api_keys.get(name) {
            None => errors.push(
                ConfigError::new(file, format!("no key for provider '{pid}' ({})", p.name))
                    .at(format!("api_keys.{name}"))
                    .fix(format!("Add {name} = \"...\" under [api_keys], as named by providers.{pid}.auth.secret in {}.", files.api)),
            ),
            Some(key) if reject && is_placeholder(key.expose()) => errors.push(
                ConfigError::new(file, "is still the template placeholder")
                    .at(format!("api_keys.{name}"))
                    .fix(format!("Paste your {} API key.", p.name)),
            ),
            Some(_) => {}
        }
    }
}

/// Loose check for a POSIX TZ string: a 3+ letter (or <...>) zone name
/// followed by an offset, e.g. "UTC0", "EST5EDT,M3.2.0,M11.1.0", "<+03>-3".
fn is_plausible_posix_tz(tz: &str) -> bool {
    let s = tz.trim();
    let rest = if let Some(r) = s.strip_prefix('<') {
        match r.find('>') {
            Some(i) => &r[i + 1..],
            None => return false,
        }
    } else {
        let n = s.chars().take_while(|c| c.is_ascii_alphabetic()).count();
        if n < 3 {
            return false;
        }
        &s[n..]
    };
    let rest = rest.trim_start_matches(['+', '-']);
    rest.starts_with(|c: char| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_api, parse_secrets, parse_user};

    const API: &str = include_str!("../../../config/api.toml");
    const USER: &str = include_str!("../../../config/user.example.toml");
    const SECRETS: &str = include_str!("../../../config/secrets.example.toml");

    const FILES: Files<'static> = Files {
        api: "config/api.toml",
        user: "config/user.toml",
        secrets: "config/secrets.toml",
    };

    fn filled_secrets() -> String {
        SECRETS
            .replace("YOUR_WIFI_NAME", "HomeNet")
            .replace("YOUR_WIFI_PASSWORD", "correct horse")
            .replace("PASTE_OPENWEATHERMAP_KEY_HERE", "owm-key")
            .replace("PASTE_NEWSAPI_KEY_HERE", "news-key")
            .replace("PASTE_FINNHUB_KEY_HERE", "fh-key")
    }

    fn run(api: &str, user: &str, secrets: &str, p: Placeholders) -> Result<(), ConfigErrors> {
        validate(
            &parse_api(api).unwrap(),
            &parse_user(FILES.user, user).unwrap(),
            &parse_secrets(FILES.secrets, secrets).unwrap(),
            FILES,
            p,
        )
    }

    /// Run with one file modified; return all messages joined.
    fn errors(api: &str, user: &str, secrets: &str) -> String {
        run(api, user, secrets, Placeholders::Reject)
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn committed_config_with_real_secrets_passes() {
        run(API, USER, &filled_secrets(), Placeholders::Reject).unwrap();
    }

    #[test]
    fn templates_pass_only_when_placeholders_allowed() {
        run(API, USER, SECRETS, Placeholders::Allow).unwrap();
        let e = errors(API, USER, SECRETS);
        assert!(
            e.contains("config/secrets.toml: api_keys.newsapi: is still the template placeholder"),
            "{e}"
        );
        assert!(e.contains("fix: Paste your NewsAPI API key."), "{e}");
        assert!(
            e.contains("wifi.ssid: is still the template placeholder"),
            "{e}"
        );
    }

    #[test]
    fn http_url_is_rejected() {
        let api = API.replace("https://newsapi.org/v2", "http://newsapi.org/v2");
        let e = errors(&api, USER, &filled_secrets());
        assert!(e.contains("config/api.toml: providers.newsapi.base_url: 'http://newsapi.org/v2' is not an https:// URL"), "{e}");
    }

    #[test]
    fn missing_required_endpoint_is_rejected() {
        let api = API.replace(
            "[providers.finnhub.endpoints.quote]",
            "[providers.finnhub.endpoints.quotes]",
        );
        let e = errors(&api, USER, &filled_secrets());
        assert!(
            e.contains("providers.finnhub.endpoints.quote: is missing"),
            "{e}"
        );
    }

    #[test]
    fn missing_key_for_provider() {
        let secrets = filled_secrets().replace("finnhub = \"fh-key\"", "");
        let e = errors(API, USER, &secrets);
        assert!(
            e.contains("api_keys.finnhub: no key for provider 'finnhub' (Finnhub)"),
            "{e}"
        );
    }

    #[test]
    fn bad_user_values() {
        let user = USER
            .replace("latitude = 40.7128", "latitude = 140.0")
            .replace("EST5EDT,M3.2.0,M11.1.0", "America/New_York")
            .replace(r#"symbol = "VOO""#, r#"symbol = "voo""#)
            .replace(
                r#"title = "Technology""#,
                r#"title = "Technology and the Internet""#,
            );
        let e = errors(API, &user, &filled_secrets());
        assert!(
            e.contains("location.latitude: 140 is outside -90..=90"),
            "{e}"
        );
        assert!(
            e.contains("location.timezone: 'America/New_York' is not a POSIX time-zone string"),
            "{e}"
        );
        assert!(
            e.contains("finance.instruments[1].symbol: 'voo' is not a ticker symbol"),
            "{e}"
        );
        assert!(e.contains("news.topics[3].title"), "{e}");
    }

    #[test]
    fn news_params_rules() {
        let user = USER.replace(
            r#"params = { sources = "bbc-news,associated-press,al-jazeera-english" }"#,
            r#"params = { sources = "bbc-news", country = "gb" }"#,
        );
        let e = errors(API, &user, &filled_secrets());
        assert!(
            e.contains("combines sources with country or category"),
            "{e}"
        );
        let user = USER.replace(
            r#"params = { country = "us", category = "general" }"#,
            "params = {}",
        );
        let e = errors(API, &user, &filled_secrets());
        assert!(
            e.contains("has none of country, category, sources or q"),
            "{e}"
        );
    }

    #[test]
    fn too_many_instruments() {
        let extra = "\n[[finance.instruments]]\nlabel = \"X\"\nsymbol = \"X\"\n".repeat(3);
        let e = errors(API, &format!("{USER}{extra}"), &filled_secrets());
        assert!(
            e.contains("finance.instruments: 7 instruments; between 1 and 6 are supported"),
            "{e}"
        );
    }

    #[test]
    fn limits_and_intervals() {
        let api = API
            .replace(
                "{ calls = 100, per = \"day\" }",
                "{ calls = 0, per = \"day\" }",
            )
            .replace("max_staleness_s = 900", "max_staleness_s = 30");
        let e = errors(&api, USER, &filled_secrets());
        assert!(e.contains("providers.newsapi.limits[0].calls: is 0"), "{e}");
        assert!(e.contains("providers.finnhub.endpoints.quote.max_staleness_s: 30 is less than min_interval_s (60)"), "{e}");
    }

    #[test]
    fn errors_never_contain_secrets() {
        let secrets = filled_secrets().replace("correct horse", "short");
        let e = errors(API, USER, &secrets);
        assert!(e.contains("wifi.password: is 5 characters"), "{e}");
        assert!(!e.contains("short\""), "{e}");
    }

    #[test]
    fn posix_tz_examples() {
        for ok in [
            "UTC0",
            "EST5EDT,M3.2.0,M11.1.0",
            "CET-1CEST,M3.5.0,M10.5.0/3",
            "<+03>-3",
            "GMT0BST,M3.5.0/1,M10.5.0",
        ] {
            assert!(is_plausible_posix_tz(ok), "{ok}");
        }
        for bad in ["", "UTC", "America/New_York", "ES5", "<+03"] {
            assert!(!is_plausible_posix_tz(bad), "{bad}");
        }
    }
}
