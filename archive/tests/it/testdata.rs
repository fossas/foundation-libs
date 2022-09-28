//! Helpers for accessing testdata.

use std::{collections::HashMap, fs, path::PathBuf};

use archive::{Destination, ProjectRoot, Target};
use walkdir::WalkDir;

#[track_caller]
fn root() -> ProjectRoot {
    PathBuf::from("testdata").canonicalize().unwrap().into()
}

/// Get the target for a given path inside `testdata`.
#[track_caller]
pub fn target(path: impl Into<PathBuf>) -> Target {
    Target::builder()
        .project(root())
        .root(path.into().canonicalize().unwrap())
        .build()
}

/// Assert the contents of the archive matched the provided tree.
#[track_caller]
pub fn assert_content(dest: &Destination, expected: Vec<(&str, &[u8])>) {
    let dest = dest.inner();
    let extracted = WalkDir::new(dest)
        .into_iter()
        .filter_map(|de| de.ok())
        .filter(|de| de.path().is_file())
        .map(|de| {
            let content = fs::read(de.path());
            (de.path().to_owned(), content)
        })
        .filter_map(|(path, content)| match content {
            Ok(content) => Some((path, content)),
            Err(_) => None,
        })
        .collect::<HashMap<PathBuf, Vec<u8>>>();

    let expected = HashMap::from_iter(
        expected
            .into_iter()
            .map(|(path, content)| (dest.join(path), content.to_vec())),
    );

    assert_eq!(extracted, expected);
}
