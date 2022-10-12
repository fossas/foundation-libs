//! Unit tests.

use crate::{expand::walk, Target, DEFAULT_ARCHIVE_POSTFIX};

#[test]
fn walk_removes_dirs() {
    let target = Target::builder().root("testdata/simplezip").build();
    assert!(target.root().exists());

    let walker = walk(target, Default::default());

    let mut temp_paths = Vec::new();
    for entry in walker {
        let entry = entry.expect("walk entry");
        let is_archive_child = entry
            .path()
            .to_string_lossy()
            .contains(DEFAULT_ARCHIVE_POSTFIX);
        if is_archive_child {
            temp_paths.push(entry);
        }
    }

    assert!(!temp_paths.is_empty());
    for entry in temp_paths {
        assert!(!entry.concrete().exists(), "entry {entry:?} must not exist");
    }
}
