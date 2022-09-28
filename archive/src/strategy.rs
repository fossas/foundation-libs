//! Strategies for expanding archives.

use std::path::PathBuf;
use std::{fs::File, io, path::Path};

use derive_more::Constructor;
#[cfg(test)]
use mockall::predicate::*;
#[cfg(test)]
use mockall::*;

use thiserror::Error;
use walkdir::WalkDir;

use crate::Identification;

use self::deny::Deny;
use self::libarchive::Libarchive;

mod deny;
mod libarchive;

/// Errors encountered during archive expansion.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The archive at the provided path is not supported for expansion.
    #[error("archive is not supported for expansion")]
    NotSupported,

    /// Unable to walk entries.
    #[error("walk dir")]
    Walk(#[from] walkdir::Error),

    /// Generic IO error while reading the archive.
    #[error("generic io")]
    IO(#[from] io::Error),

    /// Libarchive expansion failed.
    #[error("libarchive strategy")]
    Libarchive(#[from] compress_tools::Error),
}

/// The result of attempting to extract a given path.
#[derive(Debug, Constructor)]
pub(crate) struct Attempt {
    pub(crate) source: PathBuf,
    pub(crate) result: Result<PathBuf, Error>,
}

/// Describes a strategy used to expand an archive.
#[cfg_attr(test, automock)]
pub trait Strategy {
    /// Expand an archive at the provided path into a new temporary directory.
    fn expand(&self, archive: File) -> Result<PathBuf, Error>;

    /// Check whether the archive can likely be expanded with the strategy.
    fn can_expand(&self, archive: &Path) -> Result<File, Error>;
}

/// Strategies monomorphized to the identification method used for an expand invocation.
pub struct List {
    strategies: Vec<Box<dyn Strategy>>,
}

impl List {
    /// Create a new set of strategies with the provided identification strategy.
    pub fn new(identification: Identification) -> Self {
        Self {
            strategies: vec![
                Box::new(Libarchive::new(identification)),
                Box::new(Deny::new(identification)),
            ],
        }
    }

    /// Expand the archive with one of the registered strategies.
    pub fn expand(&self, archive: &Path) -> Result<PathBuf, Error> {
        for strategy in &self.strategies {
            match strategy.can_expand(archive) {
                Ok(handle) => return strategy.expand(handle),
                Err(Error::NotSupported) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(Error::NotSupported)
    }

    /// Expand a single layer of archives (i.e. not recursively) in the directory
    /// using the first compatible strategy in the list.
    ///
    /// `include` determines whether a given path should be evaluated while iterating.
    /// Paths provided to `include` are relative to `dir`.
    /// - If a directory is skipped (`include` returns `false`) it is not descended into.
    /// - If a file is skipped (`include` returns `false`) it is not considered for unarchiving.
    pub(crate) fn expand_layer(
        &self,
        dir: &Path,
        include: impl Fn(&Path) -> bool,
    ) -> Result<Vec<Attempt>, Error> {
        let mut stack = Vec::new();
        let walker = WalkDir::new(&dir)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| include(e.path()));

        for entry in walker {
            let entry = entry?;
            let extracted = self.expand(entry.path());
            if let Err(Error::NotSupported) = extracted {
                continue;
            }

            stack.push(Attempt::new(entry.into_path(), extracted));
        }

        Ok(stack)
    }
}
