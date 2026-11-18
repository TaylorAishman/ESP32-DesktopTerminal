//! Turning HTTP results into [`AppError`]s that say which provider failed,
//! how, and what to do about it.

use serde::Deserialize;

use crate::error::{AppError, ErrorCode};

/// Who a request was for, used to make error messages specific.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProviderContext<'a> {
    /// Display name, e.g. "NewsAPI".
    pub name: &'a str,
    /// Host name, e.g. "newsapi.org".
    pub host: &'a str,
    /// Key of this provider's API key in config/secrets.toml.
    pub secret_key: &'a str,
}

/// Longest provider-supplied message included in an error.
const MAX_PROVIDER_MESSAGE: usize = 120;

/// Classify an HTTP response. `None` for success (2xx).
pub fn classify_status(ctx: &ProviderContext<'_>, status: u16, body: &[u8]) -> Option<AppError> {
    if (200..300).contains(&status) {
        return None;
    }
    let detail = provider_message(body)
        .map(|m| format!(": {m}"))
        .unwrap_or_default();
    let name = ctx.name;
    let error = match status {
        401 => AppError::with_message(
            ErrorCode::ApiUnauthorized,
            format!("{name} rejected the API key (HTTP 401){detail}."),
        )
        .hint(format!(
            "Check the key '{}' in config/secrets.toml.",
            ctx.secret_key
        )),
        403 => AppError::with_message(
            ErrorCode::ApiForbidden,
            format!("{name} refused the request (HTTP 403){detail}."),
        ),
        404 => AppError::with_message(
            ErrorCode::ApiNotFound,
            format!("{name} endpoint not found (HTTP 404){detail}."),
        ),
        429 => AppError::with_message(
            ErrorCode::ApiRateLimited,
            format!("{name} rate limit reached (HTTP 429){detail}."),
        ),
        500..=599 => AppError::with_message(
            ErrorCode::ApiServerError,
            format!("{name} server error (HTTP {status}){detail}."),
        ),
        _ => AppError::with_message(
            ErrorCode::ApiUnexpectedStatus,
            format!("{name} returned HTTP {status}{detail}."),
        ),
    };
    Some(error)
}

/// Add the provider's host to a transport error from the platform.
pub fn transport_error(ctx: &ProviderContext<'_>, error: AppError) -> AppError {
    let host = ctx.host;
    let message = match error.code() {
        ErrorCode::NetDns => format!("Could not find {host} (DNS lookup failed)."),
        ErrorCode::NetConnect => format!("Could not connect to {host}."),
        ErrorCode::NetTls => format!("Secure connection to {host} failed."),
        ErrorCode::NetTimeout => format!("{host} did not respond in time."),
        ErrorCode::NetOffline => format!("No network connection to reach {host}."),
        _ => return error,
    };
    AppError::with_message(error.code(), message)
}

/// The response did not fit in the receive buffer.
pub fn too_large(ctx: &ProviderContext<'_>, endpoint: &str, limit: usize) -> AppError {
    AppError::with_message(
        ErrorCode::ApiResponseTooLarge,
        format!(
            "{} response from '{endpoint}' exceeded {limit} bytes.",
            ctx.name
        ),
    )
    .hint(format!(
        "Raise max_response_bytes for '{endpoint}' in config/api.toml, or request less data."
    ))
}

/// The response body could not be parsed.
pub fn bad_response(ctx: &ProviderContext<'_>, endpoint: &str, reason: &str) -> AppError {
    AppError::with_message(
        ErrorCode::ApiBadResponse,
        format!(
            "{} response from '{endpoint}' could not be read: {}.",
            ctx.name,
            clip(reason)
        ),
    )
}

/// Whether retrying soon can help. Rejected keys and missing endpoints will
/// not fix themselves.
pub fn is_permanent(code: ErrorCode) -> bool {
    matches!(
        code,
        ErrorCode::ApiUnauthorized
            | ErrorCode::ApiForbidden
            | ErrorCode::ApiNotFound
            | ErrorCode::ApiUnknownSymbol
            | ErrorCode::ConfigInvalid
    )
}

/// Error text from a provider's JSON error body. Understands the shapes used
/// by NewsAPI (`message`), OpenWeatherMap (`message`) and Finnhub (`error`).
fn provider_message(body: &[u8]) -> Option<String> {
    #[derive(Deserialize)]
    struct ErrorBody {
        message: Option<String>,
        error: Option<String>,
    }
    let parsed: ErrorBody = serde_json::from_slice(body).ok()?;
    let text = parsed.message.or(parsed.error)?;
    let text = text.trim().trim_end_matches('.');
    (!text.is_empty()).then(|| clip(text))
}

fn clip(s: &str) -> String {
    if s.chars().count() <= MAX_PROVIDER_MESSAGE {
        s.to_owned()
    } else {
        let mut out: String = s.chars().take(MAX_PROVIDER_MESSAGE - 1).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NEWS: ProviderContext<'static> = ProviderContext {
        name: "NewsAPI",
        host: "newsapi.org",
        secret_key: "newsapi",
    };

    #[test]
    fn success_is_not_an_error() {
        assert_eq!(classify_status(&NEWS, 200, b"{}"), None);
        assert_eq!(classify_status(&NEWS, 204, b""), None);
    }

    /// Status, body, expected code, expected message, expected hint.
    type Case = (
        u16,
        &'static [u8],
        ErrorCode,
        &'static str,
        Option<&'static str>,
    );

    #[test]
    fn status_table() {
        let cases: &[Case] = &[
            (
                401,
                br#"{"status":"error","code":"apiKeyInvalid","message":"Your API key is invalid or incorrect."}"#,
                ErrorCode::ApiUnauthorized,
                "NewsAPI rejected the API key (HTTP 401): Your API key is invalid or incorrect.",
                Some("Check the key 'newsapi' in config/secrets.toml."),
            ),
            (
                401,
                br#"{"cod":401, "message": "Invalid API key. Please see https://openweathermap.org/faq#error401 for more info."}"#,
                ErrorCode::ApiUnauthorized,
                "NewsAPI rejected the API key (HTTP 401): Invalid API key. Please see https://openweathermap.org/faq#error401 for more info.",
                None,
            ),
            (
                403,
                br#"{"error":"You don't have access to this resource."}"#,
                ErrorCode::ApiForbidden,
                "NewsAPI refused the request (HTTP 403): You don't have access to this resource.",
                None,
            ),
            (404, b"<html>", ErrorCode::ApiNotFound, "NewsAPI endpoint not found (HTTP 404).", None),
            (
                429,
                br#"{"status":"error","code":"rateLimited","message":"You have made too many requests."}"#,
                ErrorCode::ApiRateLimited,
                "NewsAPI rate limit reached (HTTP 429): You have made too many requests.",
                None,
            ),
            (503, b"", ErrorCode::ApiServerError, "NewsAPI server error (HTTP 503).", None),
            (302, b"", ErrorCode::ApiUnexpectedStatus, "NewsAPI returned HTTP 302.", None),
        ];
        for (status, body, code, message, hint) in cases {
            let e = classify_status(&NEWS, *status, body).expect("error");
            assert_eq!(e.code(), *code, "status {status}");
            assert_eq!(e.message(), *message, "status {status}");
            if let Some(h) = hint {
                assert_eq!(e.hint_text(), *h);
            }
        }
    }

    #[test]
    fn transport_errors_name_the_host() {
        let e = transport_error(&NEWS, AppError::new(ErrorCode::NetTimeout));
        assert_eq!(e.code(), ErrorCode::NetTimeout);
        assert_eq!(e.message(), "newsapi.org did not respond in time.");
        let other = AppError::new(ErrorCode::SysInternal);
        assert_eq!(transport_error(&NEWS, other.clone()), other);
    }

    #[test]
    fn long_provider_messages_are_clipped() {
        let body = format!(r#"{{"message":"{}"}}"#, "x".repeat(500));
        let e = classify_status(&NEWS, 500, body.as_bytes()).unwrap();
        assert!(e.message().chars().count() < 200);
    }

    #[test]
    fn permanence() {
        assert!(is_permanent(ErrorCode::ApiUnauthorized));
        assert!(!is_permanent(ErrorCode::ApiRateLimited));
        assert!(!is_permanent(ErrorCode::NetTimeout));
    }

    #[test]
    fn helper_messages() {
        let e = too_large(&NEWS, "top_headlines", 16384);
        assert_eq!(e.code(), ErrorCode::ApiResponseTooLarge);
        assert!(e.hint_text().contains("'top_headlines'"));
        let e = bad_response(&NEWS, "top_headlines", "missing field `articles`");
        assert_eq!(
            e.message(),
            "NewsAPI response from 'top_headlines' could not be read: missing field `articles`."
        );
    }
}
