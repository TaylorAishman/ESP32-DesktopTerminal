//! A wrapper that keeps credentials out of logs and the screen.

use core::fmt;

/// A credential (API key, Wi-Fi password). `Debug` and `Display` print `***`;
/// the value is only reachable through [`Secret::expose`], so every place that
/// uses it is easy to find.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    pub const fn new(value: T) -> Self {
        Secret(value)
    }
}

impl<T: AsRef<str>> Secret<T> {
    /// The secret value. Never log or display it.
    pub fn expose(&self) -> &str {
        self.0.as_ref()
    }

    pub fn is_empty(&self) -> bool {
        self.0.as_ref().is_empty()
    }
}

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

impl<T> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

impl<'de> serde::Deserialize<'de> for Secret<String> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d).map(Secret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_formats_the_value() {
        let s = Secret::new("hunter2");
        assert_eq!(format!("{s} {s:?} {:?}", Some(s)), "*** *** Some(***)");
        assert_eq!(s.expose(), "hunter2");
    }
}
