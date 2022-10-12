//! Unit tests.

use std::{thread::sleep, time::Duration};

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
            // Instead of pushing `Entry`, push its constituent paths and drop the entry.
            // This is required because `Entry` keeps its containing directory alive until all entries have been dropped.
            temp_paths.push((entry.concrete().to_owned(), entry.path().to_owned()));
        }
    }

    // Let some time pass: temp directories are cleaned up on drop, so this process may not be instant.
    sleep(Duration::from_millis(100));

    assert!(!temp_paths.is_empty());
    for (concrete, rendered) in temp_paths {
        assert!(
            !concrete.exists(),
            "entry {concrete:?} (rendered: {rendered:?}) must not exist"
        );
    }
}

#[test]
fn entries_keep_dirs_alive() {
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
            // Push entries so that the containing dirs are kept alive.
            temp_paths.push(entry);
        }
    }

    // Let some time pass: temp directories are cleaned up on drop, so this process may not be instant.
    // Wait a bit to ensure that if the drop cleanup was going to run, it has had a chance to do so.
    sleep(Duration::from_millis(100));

    let mut dirs = Vec::with_capacity(temp_paths.len());
    assert!(!temp_paths.is_empty());
    for entry in temp_paths {
        assert!(
            entry.concrete().exists(),
            "entry {entry:?} should be kept alive"
        );
        dirs.push(entry.concrete().to_owned());
    }

    // Let some time pass again: all entries should be dropped at this point.
    // Ensure that while we kept the containing dir alive, we didn't turn it into a zombie somehow.
    sleep(Duration::from_millis(100));
    for dir in dirs {
        assert!(!dir.exists(), "entry {dir:?} should now be cleaned up");
    }
}
