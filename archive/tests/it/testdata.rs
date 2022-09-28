//! Helpers for accessing testdata.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use archive::{Destination, ProjectRoot, Target};
use sha2::{Digest, Sha256};
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

    let extracted = HashMap::from_iter(file_content(dest));
    let expected = map_expected(dest, expected);

    assert_eq!(extracted, expected);
}

/// Assert the contents of the archive matched in the provided tree against the provided hashes.
#[track_caller]
pub fn assert_hashed_content(dest: &Destination, expected: Vec<(&str, &str)>) {
    let dest = dest.inner();

    let hashed = file_content(dest).map(|(path, content)| {
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let buf = &hasher.finalize()[..];
        (path, hex::encode(buf))
    });

    let extracted = HashMap::from_iter(hashed);
    let expected = map_expected(dest, expected);

    assert_eq!(extracted, expected);
}

#[track_caller]
fn file_content(dest: &Path) -> impl Iterator<Item = (PathBuf, Vec<u8>)> {
    WalkDir::new(dest)
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
}

fn map_expected<T>(root: &Path, expected: Vec<(&str, impl Into<T>)>) -> HashMap<PathBuf, T> {
    HashMap::from_iter(
        expected
            .into_iter()
            .map(|(path, content)| (root.join(path), content.into())),
    )
}