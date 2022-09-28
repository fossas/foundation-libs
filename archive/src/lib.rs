//! Supports recursively expanding archives in a given directory to temporary directories.

#![deny(unsafe_code)]
#![deny(missing_docs)]
#![warn(rust_2018_idioms)]

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
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

/// Options for expanding archives.
#[derive(Clone, Debug, Default, TypedBuilder, Derivative)]
pub struct Options {
    /// The recursion strategy for archives.
    /// Files are always walked recursively; this setting solely controls archive expansion recursion.
    recursion: Recursion,

    /// Filters for file matching.
    /// Currently unused.
    _filter: Filter,

    /// The strategy for identifying expansion strategies for archives.
    identification: Identification,
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
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum Identification {
    /// Use the file extension to identify an archive expansion strategy.
    #[default]
    MatchExtension,

    /// Attempt all possible expanders on each file walked. Output failed expansions as warnings.
    ///
    /// This is maximally correct, as it extracts any archive that this library is capable of extracting.
    /// However this can incur a performance overhead.
    Experiment,
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
    #[builder(setter(into))]
    include: HashSet<PathBuf>,

    /// Paths provided here are not included.
    ///
    /// Note that exclusion takes precedence; see parent doc comments for details.
    #[builder(setter(into))]
    exclude: HashSet<PathBuf>,
}

/// The results of expanding archives.
///
/// # Temporary directory deletion
///
/// **It is the responsibility of the caller to delete temporary directories when finished.**
///
/// `Destination` entries are always temporary. Two convenience methods exist to help with this:
/// - `delete_destinations`: Deletes all destinations and clears the `locations` map.
/// - `destinations`: List all destinations as a `HashSet`.
#[derive(Debug, Getters, Default)]
pub struct Expansion {
    /// Locations mapping a path discovered in the root (the [`Source`])
    /// to the location on the file system to which it was expanded (the [`Destination`]).
    #[getset(get = "pub")]
    locations: BiHashMap<Source, Destination>,

    /// Warnings encountered during the expansion process.
    ///
    /// Any non-fatal error encountered is treated as a warning and attached to the file
    /// upon which the operation was attempted.
    #[getset(get = "pub")]
    warnings: HashMap<Source, Vec<Error>>,
}

impl Expansion {
    /// Delete all destinations and clear the `locations` map.
    ///
    /// If no errors are encountered the result is ok;
    /// if any errors are encountered they are collected into the returned error list.
    pub fn delete_destinations(&mut self) -> Result<(), Vec<Error>> {
        let errors = self
            .locations
            .right_values()
            .map(|d| d.inner().to_owned())
            .fold(Vec::new(), |mut acc, destination| {
                if let Err(error) = fs::remove_dir_all(&destination) {
                    acc.push(Error::Cleanup { destination, error });
                }
                acc
            });

        self.locations = BiHashMap::default();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// List all destinations.
    pub fn list_destinations(&self) -> HashSet<Destination> {
        self.locations.right_values().cloned().collect()
    }
}

/// The target of an expansion operation.
#[derive(Clone, Debug, TypedBuilder)]
pub struct Target {
    /// The root of the project within which `target` is being searched for archives to expand.
    /// Any walked path is joined with `project_root` and compared against the filters for inclusion.
    #[builder(setter(into))]
    project: ProjectRoot,

    /// The directory within `project_root` that is being expanded.
    #[builder(setter(into))]
    root: PathBuf,
}

/// The project root directory is a special instance of a file path with special meaning.
/// It represents the root of the project in which archives in some subdir are being expanded.
/// Paths are generated with the project root in mind and compared against the root for filtering.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, From)]
pub struct ProjectRoot(PathBuf);

/// The source at which an archive was discovered in the root during an expansion operation.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, From)]
pub struct Source(PathBuf);

/// The destination to which an archive was expanded during an expansion operation.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, From)]
pub struct Destination(PathBuf);

#[duplicate_item(
    name          returns;
    [Source]      [PathBuf];
    [Destination] [PathBuf];
    [ProjectRoot] [PathBuf];
)]
impl name {
    /// Convert self into its inner value.
    pub fn into_inner(self) -> returns {
        self.0
    }

    /// Reference the inner value of self.
    pub fn inner(&self) -> &returns {
        &self.0
    }
}

#[cfg(test)]
mod test;
