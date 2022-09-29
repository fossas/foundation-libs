use std::{
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use compress_tools::{uncompress_archive, Ownership};
use derive_more::Constructor;
use lazy_static::lazy_static;
use tempfile::tempdir;

use super::*;

lazy_static! {
    static ref SUPPORTED_EXTS: Vec<&'static str> =
        vec![".zip", ".tar", ".tar.gz", ".tar.xz", ".tar.bz2", ".rpm"];
}

/// The libarchive powered strategy: https://github.com/libarchive/libarchive.
/// Formats: https://github.com/libarchive/libarchive#supported-formats
///
/// Try to decompress everything that doesn't have a better strategy with this strategy.
#[derive(Copy, Clone, Debug, Default, Constructor)]
pub struct Libarchive {
    identification: Identification,
}

impl Strategy for Libarchive {
    fn expand(&self, mut path: File) -> Result<PathBuf, Error> {
        let dir = tempdir()?;
        uncompress_archive(&mut path, dir.path(), Ownership::Ignore)?;

        // It's up to the caller to clean up temp dirs.
        Ok(dir.into_path())
    }

    fn can_expand(&self, path: &Path) -> Result<File, Error> {
        // libarchive happily "expands" things that are not archives:
        //
        // ```not_rust
        // DEBUG archive::strategy > attempting to expand entry: "/var/folders/q7/3nvvpy0d6js28m8lypw3tcx80000gn/T/.tmpOWi7Zw/simple/b.txt"
        // DEBUG archive::strategy > expanded to "/var/folders/q7/3nvvpy0d6js28m8lypw3tcx80000gn/T/.tmpyXNng0"
        // ```
        //
        // So only pass things that look like archives to it.
        if self.identification == Identification::MatchExtension {
            if ext_is_supported(path) {
                File::open(path).map_err(Error::IO)
            } else {
                Err(Error::NotSupported)
            }
        } else {
            let mut handle = File::open(path)?;
            let supported = content_is_binary(&mut handle)?;
            if supported {
                handle.seek(SeekFrom::Start(0))?;
                Ok(handle)
            } else {
                Err(Error::NotSupported)
            }
        }
    }
}

/// Inspect the file to determine if it is binary.
///
/// Uses the same method as git: "is there a zero byte in the first 8000 bytes of the file"
fn content_is_binary<R: Read>(stream: &mut R) -> Result<bool, io::Error> {
    let mut buf = Vec::new();
    stream.take(8000).read_to_end(&mut buf)?;
    Ok(buf.contains(&0))
}

fn ext_is_supported(path: &Path) -> bool {
    match path.file_name().map(|file| file.to_string_lossy()) {
        Some(file) => SUPPORTED_EXTS.iter().any(|ext| file.ends_with(ext)),
        None => false,
    }
}

impl Display for Libarchive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "libarchive")
    }
}
