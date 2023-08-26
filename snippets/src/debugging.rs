//! Provides debugging helpers to snippet extractors.

/// Conversion trait for types that can be represented with [`EscapedText`].
pub trait ToDisplayEscaped {
    fn display_escaped(&self) -> EscapedText;
}

impl<T: AsRef<[u8]>> ToDisplayEscaped for T {
    fn display_escaped(&self) -> EscapedText {
        EscapedText { buf: self.as_ref() }
    }
}

/// Wraps a byte buffer, escaping all non-ascii bytes when invoked by `Display`.
#[derive(Debug)]
pub struct EscapedText<'a> {
    buf: &'a [u8],
}

impl<'a> std::fmt::Display for EscapedText<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for c in self.buf.iter().map(|c| c.escape_ascii()) {
            write!(f, "{c}")?;
        }
        Ok(())
    }
}
