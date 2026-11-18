//! Application errors with stable codes, user-facing messages and hints.
//!
//! Every failure that can reach the log or the screen is an [`AppError`]. Its
//! [`ErrorCode`] is stable: codes are listed in `docs/Troubleshooting.md` and
//! may be quoted by users, so an existing code is never renumbered or reused.

use core::fmt;

/// Broad area an error belongs to. Determines the code prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    /// Configuration problems. Most are caught at build time.
    Config,
    /// Joining or staying on the Wi-Fi network.
    Wifi,
    /// Reaching a server: DNS, TCP, TLS, timeouts.
    Network,
    /// A server answered, but not with usable data.
    Api,
    /// The e-ink panel.
    Display,
    /// Everything else on the device: crashes, memory.
    System,
}

impl Category {
    /// Prefix used in error codes, e.g. `E-WIFI`.
    pub const fn prefix(self) -> &'static str {
        match self {
            Category::Config => "E-CFG",
            Category::Wifi => "E-WIFI",
            Category::Network => "E-NET",
            Category::Api => "E-API",
            Category::Display => "E-DISP",
            Category::System => "E-SYS",
        }
    }
}

macro_rules! error_codes {
    ($( $(#[$doc:meta])* $variant:ident = ($cat:ident, $num:literal, $title:literal, $hint:literal), )+) => {
        /// A stable identifier for a kind of failure.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum ErrorCode {
            $( $(#[$doc])* $variant, )+
        }

        impl ErrorCode {
            /// Every code, in declaration order.
            pub const ALL: &'static [ErrorCode] = &[ $( ErrorCode::$variant, )+ ];

            /// The category the code belongs to.
            pub const fn category(self) -> Category {
                match self { $( ErrorCode::$variant => Category::$cat, )+ }
            }

            /// Number within the category, e.g. `1` in `E-WIFI-01`.
            pub const fn number(self) -> u8 {
                match self { $( ErrorCode::$variant => $num, )+ }
            }

            /// Short description of what failed.
            pub const fn title(self) -> &'static str {
                match self { $( ErrorCode::$variant => $title, )+ }
            }

            /// What the user should do, when no more specific hint is known.
            pub const fn default_hint(self) -> &'static str {
                match self { $( ErrorCode::$variant => $hint, )+ }
            }
        }
    };
}

error_codes! {
    /// Configuration is invalid in a way only detectable on the device.
    ConfigInvalid = (Config, 1, "Invalid configuration",
        "Check the files in config/ and rebuild the firmware."),

    /// The access point rejected the Wi-Fi password.
    WifiAuth = (Wifi, 1, "Wi-Fi password rejected",
        "Check wifi.password in config/secrets.toml."),
    /// The configured network could not be found.
    WifiNoAp = (Wifi, 2, "Wi-Fi network not found",
        "Check wifi.ssid in config/secrets.toml and that the network is 2.4 GHz and in range."),
    /// The connection to the access point was lost.
    WifiDisconnected = (Wifi, 3, "Wi-Fi disconnected",
        "Reconnecting automatically. If this persists, move the terminal closer to the router."),

    /// The server name could not be resolved.
    NetDns = (Network, 1, "Server name could not be resolved",
        "Check the internet connection. Will retry automatically."),
    /// A TCP connection could not be opened.
    NetConnect = (Network, 2, "Could not connect to server",
        "Check the internet connection. Will retry automatically."),
    /// The TLS handshake or certificate check failed.
    NetTls = (Network, 3, "Secure connection failed",
        "Check that the clock is set (time sync) and the URL in config/api.toml is correct."),
    /// The server did not answer in time.
    NetTimeout = (Network, 4, "Server did not respond in time",
        "Will retry automatically."),
    /// No network connection is available.
    NetOffline = (Network, 5, "No network connection",
        "Waiting for Wi-Fi."),

    /// HTTP 401: the API key was rejected.
    ApiUnauthorized = (Api, 1, "API key rejected",
        "Check the API key for this provider in config/secrets.toml."),
    /// HTTP 403: the plan does not allow this request.
    ApiForbidden = (Api, 2, "Request not allowed by the API plan",
        "This data may need a paid plan. Choose another symbol or topic in config/user.toml."),
    /// HTTP 404: the endpoint does not exist.
    ApiNotFound = (Api, 3, "API endpoint not found",
        "Check the URLs in config/api.toml."),
    /// HTTP 429: the provider's rate limit was hit.
    ApiRateLimited = (Api, 4, "API rate limit reached",
        "Requests are slowed automatically. Check the limits in config/api.toml match your plan."),
    /// HTTP 5xx: the provider failed.
    ApiServerError = (Api, 5, "API server error",
        "The provider is having problems. Will retry automatically."),
    /// Any other unexpected HTTP status.
    ApiUnexpectedStatus = (Api, 6, "Unexpected API response",
        "Will retry automatically. If this persists, check the provider's status page."),
    /// The response body could not be understood.
    ApiBadResponse = (Api, 7, "API response could not be read",
        "The provider may have changed its format. Please report this with the serial log."),
    /// The response was larger than the receive buffer.
    ApiResponseTooLarge = (Api, 8, "API response too large",
        "Raise max_response_bytes for this endpoint in config/api.toml, or request less data."),
    /// The provider reported an error in the response body.
    ApiProviderError = (Api, 9, "API reported an error",
        "See the message for the provider's explanation."),
    /// The provider does not know the requested symbol.
    ApiUnknownSymbol = (Api, 10, "Unknown symbol",
        "Check the ticker in config/user.toml."),
    /// The local call budget for this provider is used up.
    ApiBudgetExhausted = (Api, 11, "API call budget used up",
        "Waiting for the provider's limit window to reset."),

    /// The panel did not finish an operation in time.
    DisplayBusyTimeout = (Display, 1, "Display did not respond",
        "Check the panel's ribbon cable and wiring. Will re-initialise on the next refresh."),
    /// Communication with the panel failed.
    DisplayIo = (Display, 2, "Display communication failed",
        "Check the panel's ribbon cable and wiring."),

    /// The firmware crashed and restarted.
    SysPanic = (System, 1, "Restarted after an internal error",
        "Details are on the diagnostics screen (hold button 1). Please report repeated crashes."),
    /// A task stopped responding and the watchdog restarted the device.
    SysWatchdog = (System, 2, "Restarted after the system stopped responding",
        "Details are on the diagnostics screen (hold button 1). Please report repeated restarts."),
    /// Memory ran low.
    SysLowMemory = (System, 3, "Memory low",
        "Reduce the number of topics or instruments in config/user.toml."),
    /// An internal operation failed unexpectedly.
    SysInternal = (System, 4, "Internal error",
        "Please report this with the serial log."),
    /// Too many crashes in a row: running in safe mode.
    SysSafeMode = (System, 5, "Safe mode after repeated restarts",
        "Press any button to try a normal start. Check the serial log for the cause."),
}

impl ErrorCode {
    /// Formatted code, e.g. `E-WIFI-01`.
    pub fn id(self) -> String {
        self.to_string()
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{:02}", self.category().prefix(), self.number())
    }
}

/// A failure, ready to be logged or shown to the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppError {
    code: ErrorCode,
    message: String,
    hint: Option<String>,
}

impl AppError {
    /// An error whose message is the code's title and whose hint is the default.
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            message: code.title().to_owned(),
            hint: None,
        }
    }

    /// An error with a specific message, e.g. naming the server or provider.
    pub fn with_message(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            hint: None,
        }
    }

    /// Replace the default hint with a more specific one.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }

    pub fn category(&self) -> Category {
        self.code.category()
    }

    /// What failed, in a sentence.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// What to do about it.
    pub fn hint_text(&self) -> &str {
        self.hint
            .as_deref()
            .unwrap_or_else(|| self.code.default_hint())
    }
}

impl From<ErrorCode> for AppError {
    fn from(code: ErrorCode) -> Self {
        AppError::new(code)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} {}", self.code, self.message, self.hint_text())
    }
}

impl std::error::Error for AppError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn codes_are_unique() {
        let ids: HashSet<String> = ErrorCode::ALL.iter().map(|c| c.id()).collect();
        assert_eq!(ids.len(), ErrorCode::ALL.len());
    }

    #[test]
    fn every_code_has_title_and_hint() {
        for code in ErrorCode::ALL {
            assert!(!code.title().is_empty(), "{code:?} has no title");
            assert!(
                code.default_hint().ends_with('.'),
                "{code:?} hint should be a full sentence"
            );
        }
    }

    #[test]
    fn code_format() {
        assert_eq!(ErrorCode::WifiAuth.id(), "E-WIFI-01");
        assert_eq!(ErrorCode::ApiUnknownSymbol.to_string(), "E-API-10");
        assert_eq!(ErrorCode::SysPanic.category(), Category::System);
    }

    #[test]
    fn display_uses_message_and_default_hint() {
        let e = AppError::with_message(
            ErrorCode::ApiUnauthorized,
            "NewsAPI rejected the API key (HTTP 401).",
        );
        assert_eq!(
            e.to_string(),
            "E-API-01: NewsAPI rejected the API key (HTTP 401). \
             Check the API key for this provider in config/secrets.toml."
        );
    }

    #[test]
    fn specific_hint_overrides_default() {
        let e =
            AppError::new(ErrorCode::ApiUnauthorized).hint("Check newsapi in config/secrets.toml.");
        assert_eq!(e.hint_text(), "Check newsapi in config/secrets.toml.");
        assert_eq!(e.message(), "API key rejected");
    }
}
