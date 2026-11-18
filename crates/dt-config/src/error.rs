//! Configuration errors, written for the person editing the files.

use core::fmt;

/// A problem in one of the configuration files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigError {
    /// File the problem is in, e.g. `config/api.toml`.
    pub file: String,
    /// Dotted key, e.g. `providers.newsapi.limits`, when known.
    pub key: Option<String>,
    /// What is wrong.
    pub message: String,
    /// How to fix it.
    pub fix: Option<String>,
}

impl ConfigError {
    pub fn new(file: &str, message: impl Into<String>) -> Self {
        ConfigError {
            file: file.to_owned(),
            key: None,
            message: message.into(),
            fix: None,
        }
    }

    pub fn at(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }

    /// A TOML syntax or schema error from the parser.
    pub(crate) fn from_toml(file: &str, err: toml::de::Error) -> Self {
        // The toml crate's message already shows the line, column and key.
        ConfigError::new(file, err.to_string().trim_end().to_owned())
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.file)?;
        if let Some(key) = &self.key {
            write!(f, ": {key}")?;
        }
        write!(f, ": {}", self.message)?;
        if let Some(fix) = &self.fix {
            write!(f, "\n  fix: {fix}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigError {}

/// Every problem found, so the user can fix them all in one go.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfigErrors(pub Vec<ConfigError>);

impl fmt::Display for ConfigErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, e) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{e}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigErrors {}
