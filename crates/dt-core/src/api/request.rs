//! Building API requests from configuration (REQ-CFG-05) without leaking the
//! API key into logs (REQ-CFG-07).

use core::fmt;

use crate::config::{AuthLocation, EndpointConfig, ProviderConfig};

/// A request ready for the HTTP client.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiRequest {
    url: String,
    headers: Vec<(String, String)>,
    /// `url` with the key replaced by `***`.
    redacted_url: String,
}

impl ApiRequest {
    /// Full URL, possibly containing the API key. Pass it to the HTTP client
    /// only; log [`redacted_url`](Self::redacted_url) instead.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Headers, possibly including the API key.
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// The URL with the API key replaced by `***`, safe to log.
    pub fn redacted_url(&self) -> &str {
        &self.redacted_url
    }
}

impl fmt::Debug for ApiRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiRequest")
            .field("url", &self.redacted_url)
            .field(
                "headers",
                &self
                    .headers
                    .iter()
                    .map(|(k, _)| k.as_str())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// Build the request for `endpoint` of `provider`, adding `extra` parameters
/// (location, topic or symbol) after the endpoint's fixed ones.
pub fn build_request(
    provider: &ProviderConfig,
    endpoint: &EndpointConfig,
    extra: &[(&str, &str)],
    user_agent: &str,
) -> ApiRequest {
    let mut query = String::new();
    let mut redacted_query = String::new();
    let mut add = |k: &str, v: &str, secret: bool| {
        for q in [&mut query, &mut redacted_query] {
            q.push(if q.is_empty() { '?' } else { '&' });
            q.push_str(&encode(k));
            q.push('=');
        }
        let value = encode(v);
        query.push_str(&value);
        redacted_query.push_str(if secret { "***" } else { &value });
    };
    for (k, v) in endpoint.params.iter().chain(extra.iter()) {
        add(k, v, false);
    }
    let mut headers = vec![("User-Agent".to_owned(), user_agent.to_owned())];
    match provider.auth.location {
        AuthLocation::Query => add(provider.auth.name, provider.auth.key.expose(), true),
        AuthLocation::Header => headers.push((
            provider.auth.name.to_owned(),
            provider.auth.key.expose().to_owned(),
        )),
    }
    let base = format!("{}{}", provider.base_url, endpoint.path);
    ApiRequest {
        url: format!("{base}{query}"),
        headers,
        redacted_url: format!("{base}{redacted_query}"),
    }
}

/// Percent-encode everything but RFC 3986 unreserved characters.
pub fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Limit, Period};
    use crate::config::AuthConfig;
    use crate::Secret;

    const EP: EndpointConfig = EndpointConfig {
        id: "current",
        path: "/weather",
        params: &[("units", "metric")],
        weight: 1,
        min_interval_s: 600,
        max_staleness_s: 3600,
        max_response_bytes: 2048,
        off_hours_interval_s: None,
    };

    fn provider(location: AuthLocation) -> ProviderConfig {
        ProviderConfig {
            id: "p",
            name: "P",
            base_url: "https://api.example.com/v1",
            host: "api.example.com",
            auth: AuthConfig {
                location,
                name: if location == AuthLocation::Query {
                    "appid"
                } else {
                    "X-Api-Key"
                },
                secret_name: "p",
                key: Secret::new("s3cr3t&key"),
            },
            limits: &[Limit {
                calls: 1,
                per: Period::Day,
            }],
            utilisation: 0.9,
            timeout_ms: 1000,
            endpoints: &[],
        }
    }

    #[test]
    fn query_auth() {
        let r = build_request(
            &provider(AuthLocation::Query),
            &EP,
            &[("q", "New York, NY")],
            "ua/1",
        );
        assert_eq!(r.url(), "https://api.example.com/v1/weather?units=metric&q=New%20York%2C%20NY&appid=s3cr3t%26key");
        assert_eq!(
            r.redacted_url(),
            "https://api.example.com/v1/weather?units=metric&q=New%20York%2C%20NY&appid=***"
        );
        assert_eq!(r.headers(), [("User-Agent".to_owned(), "ua/1".to_owned())]);
    }

    #[test]
    fn header_auth() {
        let r = build_request(&provider(AuthLocation::Header), &EP, &[], "ua/1");
        assert_eq!(r.url(), "https://api.example.com/v1/weather?units=metric");
        assert_eq!(r.redacted_url(), r.url());
        assert_eq!(
            r.headers()[1],
            ("X-Api-Key".to_owned(), "s3cr3t&key".to_owned())
        );
    }

    #[test]
    fn debug_never_shows_the_key() {
        for loc in [AuthLocation::Query, AuthLocation::Header] {
            let r = build_request(&provider(loc), &EP, &[], "ua");
            let d = format!("{r:?}");
            assert!(!d.contains("s3cr3t"), "{d}");
        }
    }

    #[test]
    fn no_params() {
        let ep = EndpointConfig { params: &[], ..EP };
        let r = build_request(&provider(AuthLocation::Header), &ep, &[], "ua");
        assert_eq!(r.url(), "https://api.example.com/v1/weather");
    }
}
