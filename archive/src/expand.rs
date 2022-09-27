//! Archive expansion functionality.

use derive_more::Constructor;
use std::{collections::VecDeque, path::Path};
use walkdir::WalkDir;

use super::*;

/// Wrap elements of the provided iterator into a tuple,
/// where the first element is the depth and the second is the original iterator item.
macro_rules! extend_at_depth {
    ($depth:expr, $stack:expr, $iter:expr) => {
        $stack.extend($iter.into_iter().map(|p| ($depth, p)))
    };
}

/// Pushes a warning for the provided path into the warnings map.
macro_rules! warn {
    ($warnings:ident, $path:ident, $err:expr) => {{
        let errs = $warnings.entry(Source::from($path)).or_insert(vec![]);
        errs.push($err.into());
    }};
}

/// Records a source and destination.
macro_rules! record {
    ($locations:ident, $source:ident, $destination:ident) => {{
        $locations.insert(Source::from($source), Destination::from($destination));
    }};
}

/// Expand all the archives in the provided `target`.
///
/// If the provided `target` is a directory, its contents are walked according to the provided `options`.
/// If it is an archive, it is expanded, and then its contents are walked according to the provided options.
///
/// Any walked path is joined with `project` and then compared against the filters for inclusion.
pub fn all(target: Target, options: Options) -> Result<Expansion, Error> {
    // Special case: don't try to scan a directory outside the project root.
    if !has_ancestor(&target.project_root, &target.root) {
        return invariant!(TargetProjectSubdir, target);
    }

    // Special case: if root is a link, error.
    if target.root.is_symlink() {
        return invariant!(TargetSymlink, target);
    }

    // Build strategies depending on the identification type.
    // Strategies can use this however they wish.
    let strategies = strategy::List::new(options.identification);

    // Stack of recursive archives to walk, and the results of the walk.
    // Rust doesn't do well with recursive function calls; instead build a stack manually.
    let mut stack = VecDeque::new();

    // Branch based on whether the initial path is an archive or a directory.
    if target.root.is_dir() {
        let extracted = expand_in_dir(&strategies, &target.root)?;
        extend_at_depth!(0, stack, extracted);
    } else if target.root.is_file() {
        let extracted = strategies.expand(&target.root);
        stack.push_back((0, Attempt::new(target.root, extracted)));
    } else {
        return invariant!(Walkable, target);
    }

    // Iterate through the stack,
    // converting it to locations (successful extractions)
    // or warnings (failed extractions).
    let mut locations = HashMap::new();
    let mut warnings = HashMap::new();

    // Branch on whether recursion is enabled.
    if let Recursion::Enabled { depth: max_depth } = options.recursion {
        // As mentioned, Rust doesn't do well with recursive function calls.
        // Work through the stack, building the results and sometimes adding to the stack as new archives are found.
        while let Some((depth, Attempt { source, result })) = stack.pop_front() {
            if depth >= max_depth {
                warn!(warnings, source, Error::RecursionLimit);
                continue;
            }

            match result {
                Ok(destination) => {
                    let next = expand_in_dir(&strategies, destination.path())?;
                    record!(locations, source, destination);
                    extend_at_depth!(depth + 1, stack, next);
                }
                Err(err) => warn!(warnings, source, err),
            }
        }
    } else {
        while let Some((_, Attempt { source, result })) = stack.pop_front() {
            match result {
                Ok(destination) => record!(locations, source, destination),
                Err(err) => warn!(warnings, source, err),
            }
        }
    }

    Ok(Expansion {
        warnings,
        locations,
    })
}

/// The result of attempting to extract a given path.
#[derive(Debug, Constructor)]
struct Attempt {
    source: PathBuf,
    result: Result<TempDir, strategy::Error>,
}

fn expand_in_dir(strats: &strategy::List, dir: &Path) -> Result<Vec<Attempt>, Error> {
    let mut stack = Vec::new();

    for entry in WalkDir::new(&dir).follow_links(false) {
        let entry = entry?;
        let extracted = strats.expand(entry.path());
        if let Err(strategy::Error::NotSupported) = extracted {
            continue;
        }

        stack.push(Attempt::new(entry.into_path(), extracted));
    }

    Ok(stack)
}

fn has_ancestor(ancestor: &Path, child: &Path) -> bool {
    child.ancestors().any(|parent| parent == ancestor)
}
