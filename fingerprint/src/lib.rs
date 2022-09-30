//! A fingerprint is a unique identifier for a file's contents.
//!
//! Fingerprints come in multiple "kinds", which are represented by textual identifiers.
//! Fingerprints themselves are represented as binary blobs.
//!
//! Fingerprint kinds MUST maintain exact implementation compatibility; once the algorithm for a given kind
//! has been created and its fingerprints have been crawled, it can't be changed. If a change is needed,
//! that has to be a new kind of fingerprint.
//!
//! This rule means that we start out with two kinds that existed prior to this library being created,
//! which have specific rules about how to compute the fingerprint, and specific text identifiers.
//!
//! For more information, refer to the documentation for the types below.

use std::{
    fmt::Display,
    fs::File,
    io::{self, Read, Seek},
    marker::PhantomData,
    path::Path,
};

use derive_getters::Getters;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
#[cfg(test)]
use typed_builder::TypedBuilder;

mod fingerprint;
pub mod serialize;
mod stream;

/// Errors that may be encountered during fingerprinting.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A generic IO error occurred while reading the content to be hashed.
    /// This error may be retried, but if it fails multiple times it's generally not recoverable.
    #[error("i/o error: {0}")]
    IO(#[from] io::Error),

    /// An invariant was not followed. These errors are not recoverable and indicate a program bug.
    #[error("invariant: {0}")]
    Invariant(InvariantError),

    /// Unimplemented functionality. This is temporary while this library is under development.
    /// Testing `todo` panics with `join` is annoying.
    #[error("unimplemented: {0}")]
    Unimplemented(String),
}

/// Kinds of invariants that may be reported in [`Error::Invariant`].
#[derive(Error, Debug, Eq, PartialEq)]
pub enum InvariantError {
    #[error("the resulting hash digest was not 32 bytes")]
    HashDigestSize,
}

/// Fingerprint kinds MUST maintain exact implementation compatibility; once the algorithm for a given kind
/// has been created and its fingerprints have been crawled, it can't be changed. If a change is needed,
/// that has to be a new kind of fingerprint. Similarly, the text representation for a given algorithm
/// cannot change either: some services assume certain things about the fingerprints that we cannot easily change
/// (for example, the VSI Forensics Service assumes all files have a `sha_256` fingerprint).
///
/// This is because fingerprints form the backbone of how VSI operates:
/// - FOSSA CLI creates them.
/// - The VSI Forensics Service assumes certain things about them.
/// - The VSI Cloud Store assumes certain things about them.
/// - The VSI Cloud Store's Crawlers create them.
/// - Crawlers and FOSSA CLI must create them in the same way.
/// - ... and all of this has to be compatible with the fingerprinting in the MVP store, which formed the initial basis of VSI.
///
/// All valid fingerprint kinds implement this trait.
///
/// This trait is sealed, indicating nothing outside this module may implement it.
///
/// ### Future work
///
/// The current implementation of `Kind` causes an issue when we want to actually send kind information
/// across a serialization boundary, because `Kind`s aren't concrete and therefore aren't
/// generally serializable.
///
/// Specifically, this is an issue for `FinalizeRevision` and `CheckRevision` methods in the VSI Cloud Store,
/// where it's not simple to send a list of `Kind`s used to fingerprint a set of files,
/// and it's not simple to then retreive that list from the API.
///
/// Instead, for `FinalizeRevision`, clients are forced to:
/// - Know what kinds of fingerprints are possible, separately.
/// - Manually call `.to_string` on those kinds to get a list of kinds used.
/// - Send them as opaque strings.
/// And for `CheckRevision`, clients are forced to:
/// - Manually compare the API result (which is a set of opaque strings) against known kinds, using the `to_string` method.
/// And the server is required to treat all this as opaque strings.
///
/// To make this less error prone, this is all handled in this library under the `serialize` module,
/// and it works for now so it's not a massive problem. But if we have ideas for how to improve this for the future,
/// we should do them.
pub trait Kind: private::Sealed {}

/// Represents a fingerprint derived by hashing the raw contents of a file with the SHA256 algorithm.
///
/// This is the default kind of fingerprint, and the kind of fingerprint with the maximal comparison signal,
/// as the raw SHA256 hash of two files matching indicates that the two files are exactly the same content.
/// It's also the fingerprint kind that works for literally all kinds of files, whereas other fingerprint kinds
/// generally require specific circumstances: `CommentStrippedSHA256` requires that the file is text, and
/// hypothetical future fingerprint kinds such as something based on an AST would require that the file is source code.
///
/// This fingerprint kind has been finalized and may not change (except to fix a bug).
#[derive(Clone, Eq, PartialEq, Debug, Hash, Serialize, Deserialize)]
pub struct RawSHA256;

impl private::Sealed for RawSHA256 {}
impl Kind for RawSHA256 {}

impl Display for RawSHA256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sha_256")
    }
}

/// Represents a fingerprint derived by hashing the contents of a file with the SHA256 algorithm
/// after performing basic C-style comment stripping.
///
/// This fingerprint kind has been finalized and may not change (except to fix a bug).
#[derive(Clone, Eq, PartialEq, Debug, Hash, Serialize, Deserialize)]
pub struct CommentStrippedSHA256;

impl private::Sealed for CommentStrippedSHA256 {}
impl Kind for CommentStrippedSHA256 {}

impl Display for CommentStrippedSHA256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "comment_stripped:sha_256")
    }
}

/// An array of bytes representing a fingerprint's content.
#[derive(Clone, Eq, PartialEq, Hash, Debug, Default, Serialize, Deserialize)]
pub struct Blob([u8; 32]);

impl Blob {
    fn from_digest<D: Digest>(digest: D) -> Result<Blob, Error> {
        let buf = &digest.finalize()[..];
        let fixed = buf
            .try_into()
            .map_err(|_| Error::Invariant(InvariantError::HashDigestSize))?;
        Ok(Blob(fixed))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Fingerprints need to be hashable by their `Kind` and `Content` values
/// for the VSI Cloud Store to properly interact with them.
pub trait Hashable {
    /// Create a new hash from a fingerprint kind and a fingerprint.
    fn to_hash(&self) -> Vec<u8>;
}

/// An opaque, deterministic value for the file's contents.
/// If two fingerprints are the same, the contents of the files used to create the fingerprints are the same.
#[derive(Clone, Eq, PartialEq, Hash, Default, Debug, Getters, Serialize, Deserialize)]
#[cfg_attr(test, derive(TypedBuilder))]
pub struct Fingerprint<K: Kind> {
    kind: PhantomData<K>,
    content: Blob,
}

impl<K> Fingerprint<K>
where
    K: Kind,
{
    fn new(content: Blob) -> Self {
        Self {
            content,
            kind: PhantomData {},
        }
    }

    fn from_digest<D: Digest>(digest: D) -> Result<Self, Error> {
        let content = Blob::from_digest(digest)?;
        Ok(Fingerprint::new(content))
    }
}

impl Hashable for Fingerprint<RawSHA256> {
    /// Create a new hash from a fingerprint kind and a fingerprint
    fn to_hash(&self) -> Vec<u8> {
        let mut bs = RawSHA256.to_string().as_bytes().to_vec();
        bs.extend_from_slice(self.content.as_bytes());
        Sha256::digest(&bs).to_vec()
    }
}

impl Hashable for Fingerprint<CommentStrippedSHA256> {
    /// Create a new hash from a fingerprint kind and a fingerprint
    fn to_hash(&self) -> Vec<u8> {
        let mut bs = CommentStrippedSHA256.to_string().as_bytes().to_vec();
        bs.extend_from_slice(self.content.as_bytes());
        Sha256::digest(&bs).to_vec()
    }
}

impl<K> Display for Fingerprint<K>
where
    K: Kind,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.content.0))
    }
}

/// The result of eagerly running all fingerprint [`Kind`]s on some given content.
///
/// When creating a [`Combined`], the same content is run through each [`Kind`].
/// Any [`Kind`] returning [`Error::Unsupported`] is silently dropped from the [`Combined`] data structure.
///
/// For example, this means that if [`Combined`] is created over a binary file, [`CommentStrippedSHA256`] is not
/// in the resulting data structure, because that kind of fingerprint requires UTF8 encoded text content to run.
#[derive(Clone, Hash, Eq, PartialEq, Debug, Getters, Serialize, Deserialize)]
#[cfg_attr(test, derive(TypedBuilder))]
pub struct Combined {
    // Important: if this struct is changed, update `serialize::kind::kinds_evaluated` to reflect the change.
    // `kinds_evaluated` may be replaced by a macro in the future.
    raw: Fingerprint<RawSHA256>,
    comment_stripped: Option<Fingerprint<CommentStrippedSHA256>>,
}

impl Combined {
    /// Create a vector of fingerprint hashes, the equivalent of running
    /// `Fingerprint::to_hash` on each `Fingerprint` stored in this struct.
    ///
    /// For `Optional` fingerprints, a `None` value is dropped from the
    /// resulting vector.
    pub fn to_hashes(&self) -> Vec<Vec<u8>> {
        let raw = self.raw.to_hash();
        if let Some(stripped) = &self.comment_stripped {
            vec![raw, stripped.to_hash()]
        } else {
            vec![raw]
        }
    }
}

impl Display for Combined {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(comment_stripped) = &self.comment_stripped {
            write!(
                f,
                "{}({}); {}({})",
                RawSHA256, self.raw, CommentStrippedSHA256, comment_stripped,
            )
        } else {
            write!(f, "{}({})", RawSHA256, self.raw())
        }
    }
}

/// Fingerprint the provided file with all fingerprint [`Kind`]s.
pub fn fingerprint(path: &Path) -> Result<Combined, Error> {
    let mut file = File::open(path)?;
    fingerprint_stream(&mut file)
}

/// Fingerprint the provided stream (typically a file handle) with all fingerprint [`Kind`]s.
pub fn fingerprint_stream<R: Read + Send + Seek + 'static>(
    stream: &mut R,
) -> Result<Combined, Error> {
    let raw = fingerprint::raw(stream)?;
    stream.seek(io::SeekFrom::Start(0))?;
    let comment_stripped = fingerprint::comment_stripped(stream)?;
    Ok(Combined {
        raw,
        comment_stripped,
    })
}

#[cfg(test)]
mod tests;

mod private {
    pub trait Sealed {}
}
