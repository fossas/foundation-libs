//! Strategies for expanding archives.

use std::{fs::File, io, path::Path};

use tempfile::TempDir;
use thiserror::Error;

use crate::Identification;

use self::deny::Deny;

mod deny;

/// Errors encountered during archive expansion.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The archive at the provided path is not supported for expansion.
    #[error("archive is not supported for expansion")]
    NotSupported,

    /// Generic IO error while reading the archive.
    #[error("generic io")]
    IO(#[from] io::Error),
}

/// Describes a strategy used to expand an archive.
pub trait Strategy {
    /// Expand an archive at the provided path into a new temporary directory.
    fn expand(&self, archive: File) -> Result<TempDir, Error>;

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
            strategies: vec![Box::new(Deny::new(identification))],
        }
    }

    /// Expand the archive with one of the registered strategies.
    pub fn expand(&self, archive: &Path) -> Result<TempDir, Error> {
        for strategy in &self.strategies {
            match strategy.can_expand(archive) {
                Ok(handle) => return strategy.expand(handle),
                Err(Error::NotSupported) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(Error::NotSupported)
    }
}
