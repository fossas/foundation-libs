//! Archive expansion functionality.

use std::{collections::VecDeque, path::Path};

use super::*;
use strategy::Attempt;

/// Expand all the archives in the provided `target`.
///
/// If the provided `target` is a directory, its contents are walked according to the provided `options`.
/// If it is an archive, it is expanded, and then its contents are walked according to the provided options.
///
/// Any walked path is joined with `project` and then compared against the filters for inclusion.
pub fn all(target: Target, options: Options) -> Result<Expansion, Error> {
    // Special case: don't try to scan a directory outside the project root.
    if !has_ancestor(target.project.inner(), &target.root) {
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
    // Using a manual stack because Rust doesn't do super well with recursive function calls (it's missing TCE).
    let mut stack = VecDeque::new();
    let mut expansion = Expansion::default();

    // Branch based on whether the initial path is an archive or a directory.
    if target.root.is_dir() {
        let extracted = strategies.expand_layer(&target.root, noop_filter)?;
        stack.extend(extracted.into_iter().map(|p| (0, p)));
    } else if target.root.is_file() {
        let extracted = strategies.expand(&target.root);
        stack.push_back((0, Attempt::new(target.root, extracted)));
    } else {
        return invariant!(Walkable, target);
    }

    // Iterate through the stack,
    // converting it to locations (successful extractions)
    // or warnings (failed extractions).
    //
    // The stack grows as new archives are discovered and expanded.
    match options.recursion {
        Recursion::Enabled { depth: max_depth } => {
            while let Some((depth, attempt)) = stack.pop_front() {
                if depth >= max_depth {
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

    Ok(expansion)
}

impl Expansion {
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

    fn record_many(&mut self, attempts: impl Iterator<Item = Attempt>) {
        attempts.into_iter().for_each(|a| self.record(a))
    }
}

fn has_ancestor(ancestor: &Path, child: &Path) -> bool {
    child.ancestors().any(|parent| parent == ancestor)
}

fn noop_filter(_: &Path) -> bool {
    true
}
