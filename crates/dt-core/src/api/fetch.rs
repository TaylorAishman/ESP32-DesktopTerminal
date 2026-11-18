//! Running one job: request, classify, parse, produce a store update.

use core::time::Duration;

use super::jobs::{Job, JobKind};
use super::request::build_request;
use super::scheduler::Outcome;
use crate::api_error::{
    bad_response, classify_status, too_large, transport_error, ProviderContext,
};
use crate::config::Config;
use crate::error::{AppError, ErrorCode};
use crate::model::Forecast;
use crate::parse::{finnhub, newsapi, owm};
use crate::platform::{Clock, HttpClient, HttpRequest, UnixTime};
use crate::store::{Update, UpdateKind};

/// Everything known about one fetch, for the store, the scheduler and the log.
#[derive(Clone, Debug, PartialEq)]
pub struct FetchReport {
    pub update: Update,
    pub outcome: Outcome,
    /// The URL with any API key replaced by `***`.
    pub redacted_url: String,
    /// HTTP status, if the server answered.
    pub status: Option<u16>,
    /// Body bytes received.
    pub bytes: usize,
}

/// Fetch and parse `job`. `buf` is the shared receive buffer; only the first
/// `max_response_bytes` of it are used for this endpoint.
pub fn execute<H: HttpClient + ?Sized>(
    job: &Job,
    config: &Config,
    http: &mut H,
    buf: &mut [u8],
    clock: &dyn Clock,
) -> FetchReport {
    let ctx = ProviderContext {
        name: job.provider.name,
        host: job.provider.host,
        secret_key: job.provider.auth.secret_name,
    };
    let request = build_request(
        job.provider,
        job.endpoint,
        &job.params_ref(),
        config.user_agent,
    );
    let headers: Vec<(&str, &str)> = request
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let limit = (job.endpoint.max_response_bytes as usize).min(buf.len());
    let response = http.get(
        &HttpRequest {
            url: request.url(),
            headers: &headers,
            timeout: Duration::from_millis(u64::from(job.provider.timeout_ms)),
        },
        &mut buf[..limit],
    );

    let mut status = None;
    let mut bytes = 0;
    let mut retry_after = None;
    let result: Result<UpdateKind, AppError> = match response {
        Err(e) => Err(transport_error(&ctx, e)),
        Ok(r) => {
            status = Some(r.status);
            bytes = r.body.len();
            retry_after = r.retry_after;
            if let Some(e) = classify_status(&ctx, r.status, r.body) {
                Err(e)
            } else if r.truncated {
                Err(too_large(&ctx, job.endpoint.id, limit))
            } else {
                match parse(job, config, r.body, clock) {
                    Ok(answer) => answer,
                    Err(reason) => Err(bad_response(&ctx, job.endpoint.id, &reason)),
                }
            }
        }
    };

    let at = clock.now().unwrap_or(UnixTime(0));
    let (kind, outcome) = match result {
        Ok(kind) => (kind, Outcome::Success),
        Err(error) => (
            failed(job.kind, error.clone()),
            Outcome::Failure { error, retry_after },
        ),
    };
    FetchReport {
        update: Update { at, kind },
        outcome,
        redacted_url: request.redacted_url().to_owned(),
        status,
        bytes,
    }
}

/// Parse a successful body. The outer `Result` is a parse failure (reason
/// text); the inner one a well-formed answer that is still an error, such as
/// an unknown symbol.
fn parse(
    job: &Job,
    config: &Config,
    body: &[u8],
    clock: &dyn Clock,
) -> Result<Result<UpdateKind, AppError>, String> {
    Ok(match job.kind {
        JobKind::CurrentWeather => Ok(UpdateKind::Current(Ok(owm::parse_current(body)?))),
        JobKind::Forecast => {
            let points = owm::parse_forecast(body)?;
            Ok(UpdateKind::Forecast(Ok(Forecast::from_points(
                points,
                |t| clock.utc_offset(t),
            ))))
        }
        JobKind::News(i) => {
            let title = config.topics.get(i).map_or("", |t| t.title);
            Ok(UpdateKind::News(
                i,
                Ok(newsapi::parse_top_headlines(body, title)?),
            ))
        }
        JobKind::Quote(i) => {
            let (label, symbol) = config
                .instruments
                .get(i)
                .map_or(("", ""), |x| (x.label, x.symbol));
            match finnhub::parse_quote(body, label, symbol)? {
                Some(q) => Ok(UpdateKind::Quote(i, Ok(q))),
                None => Err(AppError::with_message(
                    ErrorCode::ApiUnknownSymbol,
                    format!("{} does not know the symbol '{symbol}'.", job.provider.name),
                )
                .hint(format!(
                    "Check the symbol for '{label}' in config/user.toml."
                ))),
            }
        }
    })
}

fn failed(kind: JobKind, e: AppError) -> UpdateKind {
    match kind {
        JobKind::CurrentWeather => UpdateKind::Current(Err(e)),
        JobKind::Forecast => UpdateKind::Forecast(Err(e)),
        JobKind::News(i) => UpdateKind::News(i, Err(e)),
        JobKind::Quote(i) => UpdateKind::Quote(i, Err(e)),
    }
}
