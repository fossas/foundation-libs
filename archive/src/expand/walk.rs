//! Iterator based directory traversal with unarchiving.

use std::{
    collections::VecDeque,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    thread,
};

use crossbeam::channel::{bounded, Sender};
use tryvial::tryvial;
use walkdir::{DirEntry, WalkDir};

use crate::{
    strategy::{self, List},
    Error, Options, Recursion, Target,
};

/// A directory entry discovered by the walker.
#[derive(Debug)]
pub struct Entry {
    /// The logical path, relative to the expanding root.
    /// This is the path reported to clients.
    logical: PathBuf,

    /// The actual path on disk. This is hidden from clients as an implementation detail.
    concrete: PathBuf,
}

impl Entry {
    /// Create an instance with direct ancestry.
    /// Errors if the logical entry cannot be created.
    #[tryvial]
    fn direct(dir: &Path, file: &Path) -> Result<Self, Error> {
        let logical = try_make_relative(dir, file)?;
        Self {
            logical: logical.to_owned(),
            concrete: file.to_owned(),
        }
    }

    /// Create an instance from a walkdir entry with derived ancestry.
    ///
    /// Errors if the logical entry cannot be created.
    #[tryvial]
    fn derived(parent: Option<&Path>, dir: &Path, file: &Path) -> Result<Self, Error> {
        let entry = Self::direct(dir, file)?;
        match parent {
            Some(parent) => Entry {
                logical: parent.join(entry.logical),
                concrete: entry.concrete,
            },
            None => entry,
        }
    }

    /// The canonical path for the entry relative to the expanding root.
    ///
    /// This path is not suitable for opening the file; instead use the `open` method.
    /// This is because this path may not actually exist as written on disk.
    /// For more information, see the documentation for [`Walk`].
    pub fn path(&self) -> &Path {
        &self.logical
    }

    /// Open a file handle for the entry.
    pub fn open(&mut self) -> Result<File, io::Error> {
        // Even though this function doesn't actually mutate `Entry` itself,
        // it _does_ allow mutation of the underlying file, so its receiver is mut.
        File::open(&self.concrete)
    }

    /// List the concrete path at which the entry is located.
    #[cfg(test)]
    pub fn concrete(&self) -> &Path {
        &self.concrete
    }
}

/// Walks `target` recursively, outputting discovered [`Entry`] items as an iterator.
///
/// When an entry is found that references an archive that is supported for extraction,
/// it is decompressed and its contents are walked as if the archive was a directory.
/// The directory name is postfixed with the provided postfix.
///
/// For example, with the default postfix "!_fossa.virtual_!", the file tree:
/// ```not_rust
/// root/
///   some_dir.zip
/// ```
///
/// Becomes something more like:
/// ```not_rust
/// root/
///   some_dir.zip
///   some_dir.zip!_fossa.virtual_!/
///     nested.zip
///     nested.zip!_fossa.virtual_!/
///       content.txt
/// ```
///
/// Filters are evaluated based on this path structure. Symlinks are not followed.
///
/// The actual location to which the archive is extracted is the system's temporary directory.
/// It's just reported by this walker as though it is in the original target directory structure.
/// Given this, it is important to use the [`Entry`] methods to obtain a file handle for reading the file,
/// as attempting to read the path directly results in an error if the file is inside an archive.
///
/// After an archive has been fully walked it is removed from the disk.
pub fn walk(target: Target, options: Options) -> impl Iterator<Item = Result<Entry, Error>> {
    // `walk_inner` contains multiple nested iterations that need to be flattened.
    // After _much_ trial and error, this results in some _really nasty_ iterator code.
    // Instead of doing that, just use a channel and pull an iterator out of it, which keeps things much simpler.
    let (send, recv) = bounded(0);

    // Of course, the downside here is that this has to be in its own thread.
    thread::spawn(move || walk_inner(send, target.root, options));
    recv.into_iter()
}

struct WalkTarget {
    parent: Option<PathBuf>,
    depth: usize,
    dir: PathBuf,
    temp: bool,
}

impl WalkTarget {
    fn base(dir: PathBuf) -> Self {
        Self {
            dir,
            depth: 0,
            parent: None,
            temp: false,
        }
    }

    fn expanded(parent: PathBuf, dir: PathBuf, depth: usize) -> Self {
        Self {
            dir,
            depth,
            parent: Some(parent),
            temp: true,
        }
    }
}

impl Drop for WalkTarget {
    fn drop(&mut self) {
        if self.temp {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

/// Walks entries in `target` according to `options`, sending them to `tx`.
/// Any error encountered is written to `tx`, and then this function exits.
fn walk_inner(tx: Sender<Result<Entry, Error>>, root: PathBuf, options: Options) {
    let strategies = List::new(options.identification);
    let mut queue = VecDeque::from([WalkTarget::base(root)]);
    let logical_suffix = |path: &Path| {
        let mut path = path.as_os_str().to_owned();
        path.push(&options.archive_postfix);
        PathBuf::from(path)
    };

    while let Some(target) = queue.pop_front() {
        // Attempt to expand the entry.
        // If it is a supported archive, the new expanded entry is pushed onto the stack.
        // Either way, the original entry is still returned for iteration.
        let mut process = |entry: Entry| -> Result<Entry, Error> {
            match options.recursion {
                Recursion::Enabled { depth } => match strategies.expand(&entry.concrete) {
                    Ok(expanded) => {
                        let new_depth = target.depth + 1;
                        if new_depth > depth {
                            // Don't recurse further if it'd exceed the recursion depth.
                            return Ok(entry);
                        }

                        let parent = logical_suffix(&entry.logical);
                        queue.push_back(WalkTarget::expanded(parent, expanded, new_depth));
                        Ok(entry)
                    }
                    Err(strategy::Error::NotSupported) => Ok(entry),
                    Err(err) => Err(Error::Expand(err)),
                },
                Recursion::Disabled => Ok(entry),
            }
        };

        let parent = target.parent.as_deref();
        let render = |de: DirEntry| Entry::derived(parent, &target.dir, de.path());
        let allowed = |e: &Entry| options.filter.allows(e.path());
        let walk = WalkDir::new(&target.dir)
            .follow_links(false)
            .into_iter()
            .filter(|de| de.as_ref().map(|de| de.path().is_file()).unwrap_or(true))
            .flat_map(|de| de.map(render).map_err(Error::Walk))
            .filter(|entry| entry.as_ref().map(allowed).unwrap_or(true))
            .flat_map(|entry| entry.map(&mut process))
            .try_for_each(|entry| tx.send(entry));

        // If walk is error, it indicates the channel is closed; just exit.
        if walk.is_err() {
            break;
        }
    }
}

fn try_make_relative<'a>(parent: &'a Path, child: &'a Path) -> Result<&'a Path, Error> {
    child
        .strip_prefix(parent)
        .map_err(|err| Error::RenderPathRelative {
            parent: parent.to_owned(),
            child: child.to_owned(),
            err,
        })
}
