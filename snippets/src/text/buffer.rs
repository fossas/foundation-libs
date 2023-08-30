use std::borrow::Cow;

use base64::Engine;
use derive_more::{Deref, Index};
use tap::Pipe;
use thiserror::Error;

use crate::text::as_base64;

/// Errors reported when decoding a buffer.
#[derive(Debug, Error)]
pub enum Error {
    #[error("decode base64 input")]
    DecodeBase64(#[from] DecodeBase64Error),
}

/// The kind of encoding to use when displaying the buffer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Encoding {
    /// Indicate the buffer was created with arbitrary bytes.
    #[default]
    Bytes,

    /// Indicate the buffer was created with UTF8 text.
    UTF8,
}

/// A byte buffer that reports its value in a more human-readable form.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Index, Deref)]
pub struct Buffer {
    encoding: Encoding,

    #[index]
    #[deref]
    bytes: Vec<u8>,
}

impl Buffer {
    fn new_internal(encoding: Encoding, bytes: Vec<u8>) -> Self {
        Self { encoding, bytes }
    }

    /// Create a new buffer from arbitrary bytes.
    pub fn new(input: impl Into<Vec<u8>>) -> Self {
        Self::new_internal(Encoding::Bytes, input.into())
    }

    /// Decode a string into an instance.
    pub fn utf8(input: impl Into<String>) -> Self {
        Self::new_internal(Encoding::UTF8, input.into().into_bytes())
    }

    /// Decode a base64 string into an instance.
    pub fn base64(input: impl AsRef<str>) -> Result<Self, Error> {
        base64::engine::general_purpose::STANDARD_NO_PAD
            .decode(input.as_ref())
            .map_err(Error::from)
            .map(|bytes| Self::new_internal(Encoding::Bytes, bytes))
    }
}

impl std::fmt::Display for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.encoding {
            Encoding::Bytes => as_base64(&self.bytes).pipe(Cow::Owned),
            Encoding::UTF8 => try_decode_utf8(&self.bytes),
        }
        .pipe(|repr| write!(f, "{repr}"))
    }
}

fn try_decode_utf8(input: &[u8]) -> Cow<'_, str> {
    std::str::from_utf8(input)
        .map(Cow::Borrowed)
        .unwrap_or_else(|_| as_base64(input).pipe(Cow::Owned))
}

/// An error that occurs when trying to decode into a buffer.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct DecodeBase64Error(#[from] base64::DecodeError);

impl From<base64::DecodeError> for Error {
    fn from(value: base64::DecodeError) -> Self {
        DecodeBase64Error(value).pipe(Self::DecodeBase64)
    }
}
