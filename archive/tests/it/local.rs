use archive::*;

use crate::testdata;

#[test]
fn cleanup_on_drop() {
    pretty_env_logger::init();

    let target = testdata::target("testdata/simplezip");
    let opts = Options::default();

    let result = expand::all(target.clone(), opts).unwrap();
    let destination = result
        .locations()
        .get_by_left(&Source::from(target.root().join("simple.zip")))
        .unwrap()
        .inner()
        .clone();

    assert!(destination.exists(), "must have extracted");
    drop(result);
    assert!(!destination.exists(), "must have been cleaned up");
}

#[test]
fn cleanup_on_cleanup() {
    pretty_env_logger::init();

    let target = testdata::target("testdata/simplezip");
    let opts = Options::default();

    let mut result = expand::all(target.clone(), opts).unwrap();
    let destination = result
        .locations()
        .get_by_left(&Source::from(target.root().join("simple.zip")))
        .unwrap()
        .inner()
        .clone();

    assert!(destination.exists(), "must have extracted");
    result.cleanup().unwrap();
    assert!(!destination.exists(), "must have been cleaned up");
}

#[test]
fn cleanup_multiple() {
    pretty_env_logger::init();

    let target = testdata::target("testdata/simplezip");
    let opts = Options::default();

    let mut result = expand::all(target.clone(), opts).unwrap();
    let destination = result
        .locations()
        .get_by_left(&Source::from(target.root().join("simple.zip")))
        .unwrap()
        .inner()
        .clone();

    assert!(destination.exists(), "must have extracted");
    result.cleanup().unwrap();
    assert!(!destination.exists(), "must have been cleaned up");

    // Calling cleanup more times is fine
    result.cleanup().unwrap();
    result.cleanup().unwrap();
    drop(result);
}
