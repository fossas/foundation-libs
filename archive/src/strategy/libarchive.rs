use std::path::Path;

use compress_tools::{uncompress_archive, Ownership};
use derive_more::Constructor;
use tempfile::tempdir;

use super::*;

/// The libarchive powered strategy: https://github.com/libarchive/libarchive.
/// Formats: https://github.com/libarchive/libarchive#supported-formats
///
/// Try to decompress everything that doesn't have a better strategy with this strategy.
#[derive(Copy, Clone, Debug, Default, Constructor)]
pub struct Libarchive {
    _identification: Identification,
}

impl Strategy for Libarchive {
    fn expand(&self, mut path: File) -> Result<PathBuf, Error> {
        let dir = tempdir()?;
        uncompress_archive(&mut path, dir.path(), Ownership::Ignore)?;

        // It's up to the caller to clean up temp dirs.
        Ok(dir.into_path())
    }

    fn can_expand(&self, path: &Path) -> Result<File, Error> {
        let handle = File::open(path)?;
        Ok(handle)
    }
}
