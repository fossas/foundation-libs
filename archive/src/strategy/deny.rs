use std::path::Path;

use derive_more::Constructor;

use super::*;

/// The final fallback strategy; it denies every archive passed to it.
#[derive(Copy, Clone, Debug, Default, Constructor)]
pub struct Deny {
    _identification: Identification,
}

impl Strategy for Deny {
    fn expand(&self, _: File) -> Result<PathBuf, Error> {
        Err(Error::NotSupported)
    }

    fn can_expand(&self, _: &Path) -> Result<File, Error> {
        Err(Error::NotSupported)
    }
}

impl Display for Deny {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "deny")
    }
}
