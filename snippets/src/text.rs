//! Text based operations on byte streams and strings.

use base64::Engine;
use sha2::{Digest, Sha256};
use tap::Pipe;

pub use buffer::Buffer;
pub use normalize_lines::*;
pub use normalize_space::*;

pub mod buffer;
mod normalize_lines;
mod normalize_space;

/// Given a buffer, produce a fingerprint of its contents.
pub fn fingerprint(input: impl AsRef<[u8]>) -> Buffer {
    // Using an iterator here because according to the rough benchmarks it's much faster than cloning into vec.
    let normalized_line_endings = input.as_ref().iter().copied().convert_crlf_lf();
    let mut hasher = Sha256::new();
    for c in normalized_line_endings {
        hasher.update([c]);
    }
    hasher.finalize().as_slice().pipe(Buffer::new)
}

/// Given a buffer, produce a base64 representation of its contents.
pub fn as_base64(input: impl AsRef<[u8]>) -> String {
    base64::engine::general_purpose::STANDARD_NO_PAD.encode(input)
}
