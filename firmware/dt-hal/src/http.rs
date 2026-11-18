//! HTTPS client, implementing [`dt_core::platform::HttpClient`].

use core::time::Duration;
use std::ffi::CString;

use dt_core::platform::{HttpClient, HttpRequest, HttpResponse, PlatformResult};
use dt_core::{AppError, ErrorCode};
use esp_idf_sys::dt_hal as ffi;

use crate::HalError;

/// HTTPS GET over the C HAL. One request at a time.
#[derive(Debug, Default)]
pub struct HalHttp;

impl HttpClient for HalHttp {
    fn get<'b>(&mut self, request: &HttpRequest<'_>, buf: &'b mut [u8]) -> PlatformResult<HttpResponse<'b>> {
        let nul = || AppError::with_message(ErrorCode::SysInternal, "Request contains a NUL character.");
        let url = CString::new(request.url).map_err(|_| nul())?;
        let owned: Vec<(CString, CString)> = request
            .headers
            .iter()
            .map(|(k, v)| Ok((CString::new(*k).map_err(|_| nul())?, CString::new(*v).map_err(|_| nul())?)))
            .collect::<PlatformResult<_>>()?;
        let headers: Vec<ffi::hal_http_header_t> = owned
            .iter()
            .map(|(k, v)| ffi::hal_http_header_t {
                name: k.as_ptr(),
                value: v.as_ptr(),
            })
            .collect();
        let req = ffi::hal_http_request_t {
            url: url.as_ptr(),
            headers: headers.as_ptr(),
            header_count: headers.len(),
            timeout_ms: request.timeout.as_millis().min(u128::from(u32::MAX)) as u32,
        };
        let mut resp = ffi::hal_http_response_t {
            status: 0,
            body_len: 0,
            truncated: false,
            retry_after_s: -1,
        };
        // SAFETY: `req` and everything it points to (`url`, `headers` and the
        // strings in `owned`) live until the call returns; `buf` is valid for
        // writes of `buf.len()` bytes; `resp` is a live local. The C side
        // keeps no pointers after returning.
        let rc = unsafe { ffi::hal_http_get(&req, buf.as_mut_ptr(), buf.len(), &mut resp) };
        HalError::check(rc).map_err(http_error)?;
        let len = resp.body_len.min(buf.len());
        Ok(HttpResponse {
            status: u16::try_from(resp.status).unwrap_or(0),
            body: &buf[..len],
            truncated: resp.truncated,
            retry_after: u64::try_from(resp.retry_after_s).ok().map(Duration::from_secs),
        })
    }
}

fn http_error(e: HalError) -> AppError {
    match e {
        // A connection that drops mid-response is a network failure.
        HalError::Io => AppError::new(ErrorCode::NetConnect),
        other => other.into_app_error("HTTP request"),
    }
}
