//! Supports recursively expanding archives in a given directory to temporary directories.

#![deny(unsafe_code)]
#![deny(missing_docs)]
#![warn(rust_2018_idioms)]

use std::{
    collections::{HashMap, HashSet},
    fs, mem,
    path::{Path, PathBuf},
};

use bimap::BiHashMap;
use derivative::Derivative;
use derive_more::From;
use duplicate::duplicate_item;
use getset::Getters;
use typed_builder::TypedBuilder;

mod error;
pub mod expand;
mod strategy;

pub use error::*;

/// The default archive postfix.
pub const DEFAULT_ARCHIVE_POSTFIX: &str = "!_fossa.virtual_!";

/// Options for expanding archives.
#[derive(Clone, Debug, TypedBuilder, Derivative)]
#[derivative(Default)]
pub struct Options {
    /// The recursion strategy for archives.
    /// Files are always walked recursively; this setting solely controls archive expansion recursion.
    #[builder(default)]
    recursion: Recursion,

    /// The strategy for identifying expansion strategies for archives.
    #[builder(default)]
    identification: Identification,

    /// Filters for file walking.
    ///
    /// Currently unused but planned to be used in the future.
    /// Accepting it as an option today means it can be used in the future without a breaking change.
    #[builder(default)]
    filter: Filter,

    /// The postfix to append to any expanded archive.
    ///
    /// This postfix does not affect the actual path on disk to which archives are expanded;
    /// this postfix is appended to the _rendered_ path on disk for children of the archive.
    #[builder(setter(into), default = String::from(DEFAULT_ARCHIVE_POSTFIX))]
    #[derivative(Default(value = "String::from(DEFAULT_ARCHIVE_POSTFIX)"))]
    archive_postfix: String,
}

/// Recursion mode for expanding archives.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Derivative)]
#[derivative(Default)]
pub enum Recursion {
    /// Recursive archive expansion is enabled with the specified associated options.
    #[derivative(Default)]
    Enabled {
        /// The recursive depth to which archives are expanded.
        ///
        /// Each time an archive is expanded, that increments the "depth" counter for the operation.
        /// While walking within the expanded archive, finding another archive and expanding it increments the counter again.
        /// Walking back up out of the archive decrements the counter.
        ///
        /// The intention with this is to prevent common archive attacks which would normally result in infinite recursion leading to a denial of service,
        /// without meaningfully impacting actual valid recursion. This default was chosen based on observed archive recursion depths in large projects
        /// such as Chromium or AOSP.
        ///
        /// The depth limit _does not_ include the root archive if the root is an archive;
        /// this is intended to make roots more interchangable with less surprises for users.
        /// A value of zero is supported and results in no archives being expanded other than the root (if it is an archive).
        #[derivative(Default(value = "1000"))]
        depth: usize,
    },

    /// Recursive archive expansion is disabled. Only the root directory is evaluated for archives.
    ///
    /// If the root provided is an archive and recursion is disabled the root's children are _not_ expanded.
    /// An option if this is not desired is `Recursion::Enabled{ depth: 1 }`, since the initial archive
    /// does not count against the recursion limit.
    Disabled,
}

/// Identification mode for identifying archives to expand.
///
/// While this currently consists of one option, the idea is that there may be other options in the future;
/// by using an enum new options may be added as non-breaking changes.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum Identification {
    /// Use the file extension to identify an archive expansion strategy.
    #[default]
    MatchExtension,
}

/// Filters for file walking.
///
/// Because everything is walked by default, the filter mechanism is
/// intended to _reduce_ outputs from one stage to the next.
/// This comes in two forms:
///
/// - `exclude`: Do not include this item.
/// - `include`: Do not include anything that _isn't_ this item.
///
/// Notably, there is no mechanism for _adding_ items.
/// This is intentional: everything is included by default, so what could be added?
/// This can lead to some unintuitive filtering combinations:
/// `exclude 'a/b' AND include 'a/b/c'` removes `a/b/*`, _along with the "included" `a/b/c`_.
///
/// However, given the reduction-focused nature of the filters, this can be reworded using the descriptions above,
/// resulting in intuitive behavior:
/// ```not_rust
/// exclude 'a/b' AND include 'a/b/c'
/// ```
///
/// Is better understood as:
/// ```not_rust
/// Do not include 'a/b/**'
/// AND
/// Do not include anything that is not 'a/b/c/**'
/// ```
///
/// In this wording, it is clear that `a/b/c` _must not be included_, because it has been excluded from the results.
/// In fact, given only these filters, nothing is ever returned!
///
/// This means the filters are wrong: the user probably wanted simply `include a/b/c`, without the exclude clause.
/// This basic property of reduction-only filters leads to a fundamental rule:
/// _if an item is both included and excluded, it is not included_.
///
/// # Filtering mechanics
///
/// Filtering is accomplished during the walk process. For each file or directory walked:
/// - It is made relative to the scan root.
/// - It is compared to the filters list.
///
/// A given path is considered "filtered" and not walked if one of the below are true:
/// - The path is present in `exclude`. Directories matching `exclude` are skipped from being traversed.
/// - `include` is not empty and neither the path nor any of its ancestors are present in `include`.
///
/// Files and directories inside archives are still compared to the filters.
/// They trace their parent ancestry through the archive as though the archive were a directory.
/// For example the following tree:
/// ```not_rust
/// root/
///   foo/
///     bar.zip/
///       inner.rs
/// ```
#[derive(Clone, Eq, PartialEq, Debug, Default, TypedBuilder)]
pub struct Filter {
    /// Paths provided here are included.
    ///
    /// Note that exclusion takes precedence; see parent doc comments for details.
    #[builder(setter(into), default)]
    include: HashSet<PathBuf>,

    /// Paths provided here are not included.
    ///
    /// Note that exclusion takes precedence; see parent doc comments for details.
    #[builder(setter(into), default)]
    exclude: HashSet<PathBuf>,
}

impl Filter {
    /// Test whether the filters exclude the given path.
    pub(crate) fn excludes(&self, path: &Path) -> bool {
        self.exclude.iter().any(|ex| path.starts_with(ex))
    }

    /// Test whether the filters allow for the given path.
    pub(crate) fn allows(&self, path: &Path) -> bool {
        self.include.is_empty() || self.include.iter().any(|inc| path.starts_with(inc))
    }
}

/// The target of an expansion operation.
#[derive(Clone, Debug, TypedBuilder, Getters, From)]
pub struct Target {
    /// The directory within `project_root` that is being expanded.
    #[builder(setter(into))]
    #[getset(get = "pub")]
    root: PathBuf,
}

/// The source at which an archive was discovered in the root during an expansion operation.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, From)]
pub struct Source(PathBuf);

/// The destination to which an archive was expanded during an expansion operation.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, From)]
pub struct Destination(PathBuf);

#[duplicate_item(
    name          internal;
    [Source]      [PathBuf];
    [Destination] [PathBuf];
)]
impl name {
    /// Create a new instance of self.
    pub fn new(inner: impl Into<internal>) -> Self {
        Self(inner.into())
    }

    /// Convert self into its inner value.
    pub fn into_inner(self) -> internal {
        self.0
    }

    /// Reference the inner value of self.
    pub fn inner(&self) -> &internal {
        &self.0
    }
}

#[cfg(test)]
mod test;
