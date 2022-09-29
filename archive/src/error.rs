use std::{io, path::PathBuf};

use thiserror::Error;

use crate::{strategy, Target};

/// Convenience macro to create an invariant error.
#[macro_export]
macro_rules! invariant {
    ($kind:ident, $arg:ident) => {
        Err(Error::Invariant(Invariant::$kind { $arg }))
    };
    ($kind:ident, $( $arg:expr ),*) => {
        Err(Error::Invariant(Invariant::$kind { $($arg),* }))
    }
}

/// Errors encountered while expanding archives.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// An invariant was violated.
    #[error("invariant")]
    Invariant(#[from] Invariant),

    /// Unable to walk entries.
    #[error(transparent)]
    Walk(#[from] walkdir::Error),

    /// Unable to expand an archive.
    #[error("expand")]
    Expand(#[from] strategy::Error),

    /// Unable to remove a [`Destination`].
    #[error("cleanup destination {destination:?}")]
    Cleanup {
        /// The destination directory that failed to be cleaned up.
        destination: PathBuf,

        /// The error encountered when attempting to clean up the directory.
        #[source]
        error: io::Error,
    },

    /// Reached the recursion limit.
    #[error("recursion limit")]
    RecursionLimit,
}

/// Invariants expected by this library.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Invariant {
    /// The target is not a subdirectory of the project root.
    #[error("target {:?} is not a subdirectory of project root {:?}", target.root, target.project)]
    TargetProjectSubdir {
        /// The target provided to the archive expansion function.
        target: Target,
    },

    /// The target is not walkable. It is either not a supported archive or not a directory.
    #[error("{:?} is not walkable; it must be an archive or a directory", target.root)]
    Walkable {
        /// The target provided to the archive expansion function.
        target: Target,
    },

    /// The target is a symlink, and symlinks are disabled.
    #[error("{:?} is a symbolic link, and symbolic link following is disabled", target.root)]
    TargetSymlink {
        /// The target provided to the archive expansion function.
        target: Target,
    },
}
