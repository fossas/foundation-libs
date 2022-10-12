//! Archive expansion functionality.

use log::debug;
use std::{collections::VecDeque, path::Path};

use crate::{strategy::Attempt, *};

/// Synchronously expand all the archives in the provided `target`.
///
/// If the provided `target` is a directory, its contents are walked according to the provided `options`.
/// If it is an archive, it is expanded, and then its contents are walked according to the provided options.
///
/// Any walked path is joined with `project` and then compared against the filters for inclusion.
///
/// It is recommended to use the iterator walker if possible instead, as it keeps disk space more under control
/// by removing temporary directories after they are no longer needed instead of unarchiving all contents at once.
pub fn all(target: Target, options: Options) -> Result<Expansion, Error> {
    debug!("Expanding {target:?} with {options:?}");

    // Since filters are unused today, don't let the user use anything other than the default filter set.
    // This way nobody can accidentally rely on passing in some ignored filter that later silently breaks
    // functionality without breaking the signature.
    if options.filter != Filter::default() {
        return invariant!(FiltersUnsupported);
    }

    // Special case: if root is a link, error.
    if target.root.is_symlink() {
        debug!("{:?} is a symlink", target.root);
        return invariant!(TargetSymlink, target);
    }

    // Build strategies depending on the identification type.
    // Strategies can use this however they wish.
    let strategies = strategy::List::new(options.identification);
    debug!("using {strategies}");

    // Stack of recursive archives to walk, and the results of the walk.
    // Using a manual stack because Rust doesn't do super well with recursive function calls (it's missing TCE).
    let mut stack = VecDeque::new();
    let mut expansion = Expansion::default();

    // Branch based on whether the initial path is an archive or a directory.
    if target.root.is_dir() {
        debug!("{:?} is a directory", target.root);
        let extracted = strategies.expand_layer(&target.root, noop_filter)?;
        stack.extend(extracted.into_iter().map(|p| (0, p)));
    } else if target.root.is_file() {
        debug!("{:?} is a file, treating as an archive", target.root);
        let extracted = strategies.expand(&target.root);
        stack.push_back((0, Attempt::new(target.root, extracted)));
    } else {
        debug!("{:?} is neither directory nor file", target.root);
        return invariant!(Walkable, target);
    }

    // Iterate through the stack,
    // converting it to locations (successful extractions)
    // or warnings (failed extractions).
    //
    // The stack grows as new archives are discovered and expanded.
    match options.recursion {
        Recursion::Enabled { depth: max_depth } => {
            debug!("recursing (max depth '{max_depth}')");
            while let Some((depth, attempt)) = stack.pop_front() {
                debug!("recording at depth '{depth}': {attempt:?}");
                if depth >= max_depth {
                    debug!("recursion limit reached!");
                    expansion.warn(attempt.source, Error::RecursionLimit);
                    continue;
                }

                let expanded = attempt.result.as_ref().map(|d| d.to_owned()).ok();
                expansion.record(attempt);

                if let Some(next_path) = expanded {
                    let depth = depth + 1;
                    let next = strategies.expand_layer(&next_path, noop_filter)?;
                    stack.extend(next.into_iter().map(|p| (depth, p)));
                }
            }
        }
        Recursion::Disabled => expansion.record_many(stack.into_iter().map(|(_, b)| b)),
    }

    debug!("finished expanding");
    Ok(expansion)
}

/// The results of synchronously expanding all archives in a given root.
///
/// # Temporary directory deletion
///
/// Directories, represented by `Destination` entries in `locations`,
/// are deleted automatically when `Expansion` goes out of scope.
///
/// It is the user's responsibility to ensure that no further interaction is attempted after dropping `Expansion`.
///
/// - To avoid automatically deleting the temporary directories, use `persist`.
/// - Automatic cleanup doesn't allow the user to view potential errors; to view cleanup errors use `cleanup` directly.
///
/// # Resource leaking
///
/// Various platform-specific conditions may cause `Expansion` to fail to delete the underlying directory.
/// It is also possible to prevent cleanup via segfault, SIGINT, `std::process::exit` or similar.
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

impl Drop for Expansion {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

impl Expansion {
    /// Persist the temporary directories to disk, returning the contents of `Self` as a tuple.
    ///
    /// This consumes the `Expansion` without deleting temporary directories on the file system,
    /// meaning that they are no longer automatically deleted.
    pub fn persist(self) -> (BiHashMap<Source, Destination>, HashMap<Source, Vec<Error>>) {
        let mut this = mem::ManuallyDrop::new(self);
        (
            mem::take(&mut this.locations),
            mem::take(&mut this.warnings),
        )
    }

    /// Delete all destinations and clear the `locations` map.
    ///
    /// If no errors are encountered the result is ok;
    /// if any errors are encountered they are collected into the returned error list.
    ///
    /// This function is equivalent to causing `Expansion` to be dropped by letting it go out of scope;
    /// the intention with this function is to enable checking for cleanup errors when desired.
    ///
    /// It is supported to call `cleanup` multiple times; subsequent operations are a no-op,
    /// regardless whether the first call to `cleanup` was fully successful.
    pub fn cleanup(&mut self) -> Result<(), Vec<Error>> {
        // Special case for when dropped after manually running `cleanup`.
        if self.locations.is_empty() {
            return Ok(());
        }

        let errors: Vec<_> = mem::take(&mut self.locations)
            .into_iter()
            .filter_map(|(_, dest)| {
                let destination = dest.into_inner();
                if let Err(error) = fs::remove_dir_all(&destination) {
                    Some(Error::Cleanup { destination, error })
                } else {
                    None
                }
            })
            .collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// List all destinations.
    pub fn destinations(&self) -> HashSet<Destination> {
        self.locations.right_values().cloned().collect()
    }

    fn record(&mut self, attempt: Attempt) {
        match attempt.result {
            Ok(destination) => {
                self.locations
                    .insert(attempt.source.into(), destination.into());
            }
            Err(err) => {
                self.warn(attempt.source, err.into());
            }
        }
    }

    fn warn(&mut self, source: PathBuf, warning: Error) {
        let errs = self.warnings.entry(Source::from(source)).or_insert(vec![]);
        errs.push(warning);
    }

    fn record_many(&mut self, attempts: impl IntoIterator<Item = Attempt>) {
        attempts.into_iter().for_each(|a| self.record(a))
    }
}

fn noop_filter(_: &Path) -> bool {
    true
}
